//! Host-independent orchestration for independent deformation evaluations.
//!
//! This crate deliberately does not compose controller output with pin-deformation
//! output. It invokes each existing evaluator with its own explicit inputs and
//! preserves each result/error channel independently. Multi-source composition,
//! coordinate conversion, precedence, and additive semantics belong to a future
//! FDM architecture decision.

use form_deform_core::{DeformError, DeformSpec, Vec2, evaluate_pin_position};
use smart_controller_core::{ControllerBinding, ControllerError, PreviewResult};

/// Explicit input for one Smart Controller binding evaluation.
#[derive(Clone, Debug)]
pub struct ControllerEvaluationRequest {
    pub binding: ControllerBinding,
    pub input: f64,
}

/// Explicit input for one bone-driven pin evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct PinDeformationRequest {
    /// Stable host identity of the warped element.
    pub element_id: String,
    /// Stable host identity of the pin within the element.
    pub pin_id: String,
    pub spec: DeformSpec,
    pub current_angle_degrees: f64,
}

/// Absolute result from the existing pin-deformation evaluator.
#[derive(Clone, Debug, PartialEq)]
pub struct PinDeformationResult {
    pub element_id: String,
    pub pin_id: String,
    pub position: Vec2,
}

/// Results are intentionally separate: neither channel consumes or modifies the other.
#[derive(Debug, PartialEq)]
pub struct IndependentEvaluation {
    /// None means no controller evaluation was requested.
    /// Some(Ok(None)) means the requested binding was disabled.
    pub controller: Option<Result<Option<PreviewResult>, ControllerError>>,
    /// None means no pin evaluation was requested.
    pub pin: Option<Result<PinDeformationResult, DeformError>>,
}

/// Evaluate the supplied requests independently.
///
/// The function does not match targets, convert controller offsets into deformation
/// gains, add displacements, choose a winning source, or mutate a host document.
/// A failure in one channel does not suppress evaluation of the other.
pub fn evaluate_independent(
    controller: Option<ControllerEvaluationRequest>,
    pin: Option<PinDeformationRequest>,
) -> IndependentEvaluation {
    let controller = controller.map(|request| request.binding.evaluate(request.input));
    let pin = pin.map(|request| {
        evaluate_pin_position(request.spec, request.current_angle_degrees).map(|position| {
            PinDeformationResult {
                element_id: request.element_id,
                pin_id: request.pin_id,
                position,
            }
        })
    });

    IndependentEvaluation { controller, pin }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_controller_core::{ControllerMode, ControllerTarget, EaseCurve, InputRange};

    fn binding(mode: ControllerMode) -> ControllerBinding {
        ControllerBinding {
            id: "binding-1".into(),
            controller_id: "bone-1".into(),
            input_range: InputRange { minimum: -1.0, neutral: 0.0, maximum: 1.0 },
            easing: EaseCurve::Linear,
            mode,
            target: ControllerTarget::GeometryDeform {
                element_id: "element-1".into(),
                pin_id: "pin-1".into(),
                negative_offset: Vec2 { x: -10.0, y: 0.0 },
                neutral_offset: Vec2 { x: 0.0, y: 0.0 },
                positive_offset: Vec2 { x: 10.0, y: 0.0 },
            },
            enabled: true,
        }
    }

    fn pin_request() -> PinDeformationRequest {
        PinDeformationRequest {
            element_id: "element-1".into(),
            pin_id: "pin-1".into(),
            spec: DeformSpec {
                neutral_position: Vec2 { x: 100.0, y: 50.0 },
                neutral_angle_degrees: 0.0,
                gain_per_degree: Vec2 { x: 0.5, y: -0.25 },
            },
            current_angle_degrees: 20.0,
        }
    }

    #[test]
    fn both_evaluators_run_and_keep_their_own_results() {
        let result = evaluate_independent(
            Some(ControllerEvaluationRequest {
                binding: binding(ControllerMode::RelativeDelta),
                input: 0.5,
            }),
            Some(pin_request()),
        );

        assert_eq!(
            result.controller.unwrap().unwrap(),
            Some(PreviewResult::GeometryDeform {
                element_id: "element-1".into(),
                pin_id: "pin-1".into(),
                offset: Vec2 { x: 5.0, y: 0.0 },
            })
        );
        assert_eq!(
            result.pin.unwrap().unwrap(),
            PinDeformationResult {
                element_id: "element-1".into(),
                pin_id: "pin-1".into(),
                position: Vec2 { x: 110.0, y: 45.0 },
            }
        );
    }

    #[test]
    fn controller_failure_does_not_suppress_pin_evaluation() {
        let mut invalid = binding(ControllerMode::RelativeDelta);
        invalid.input_range.maximum = 0.0;

        let result = evaluate_independent(
            Some(ControllerEvaluationRequest { binding: invalid, input: 0.5 }),
            Some(pin_request()),
        );

        assert!(result.controller.unwrap().is_err());
        assert_eq!(result.pin.unwrap().unwrap().position, Vec2 { x: 110.0, y: 45.0 });
    }

    #[test]
    fn pin_failure_does_not_suppress_controller_preview() {
        let mut invalid_pin = pin_request();
        invalid_pin.current_angle_degrees = f64::NAN;

        let result = evaluate_independent(
            Some(ControllerEvaluationRequest {
                binding: binding(ControllerMode::AbsolutePose),
                input: 0.5,
            }),
            Some(invalid_pin),
        );

        assert_eq!(
            result.controller.unwrap().unwrap(),
            Some(PreviewResult::GeometryDeform {
                element_id: "element-1".into(),
                pin_id: "pin-1".into(),
                offset: Vec2 { x: 5.0, y: 0.0 },
            })
        );
        assert_eq!(result.pin.unwrap().unwrap_err(), DeformError::NonFiniteInput);
    }

    #[test]
    fn either_evaluation_can_be_omitted_or_disabled() {
        assert_eq!(
            evaluate_independent(None, None),
            IndependentEvaluation { controller: None, pin: None }
        );

        let mut disabled = binding(ControllerMode::RelativeDelta);
        disabled.enabled = false;
        let result = evaluate_independent(
            Some(ControllerEvaluationRequest { binding: disabled, input: 0.5 }),
            None,
        );
        assert_eq!(result.controller.unwrap().unwrap(), None);
        assert!(result.pin.is_none());
    }
}
