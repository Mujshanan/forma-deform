//! Pure Smart Controller binding model and evaluator.
//!
//! This crate has no AniCraft, WASM, browser, or UI dependencies.
//! Host identity resolution, persistence, and display belong in an adapter crate.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputRange {
    pub minimum: f64,
    pub neutral: f64,
    pub maximum: f64,
}

impl InputRange {
    pub fn validate(self) -> Result<(), ControllerError> {
        let lower_span = self.neutral - self.minimum;
        let upper_span = self.maximum - self.neutral;
        if !self.minimum.is_finite()
            || !self.neutral.is_finite()
            || !self.maximum.is_finite()
            || !lower_span.is_finite()
            || !upper_span.is_finite()
            || lower_span <= 0.0
            || upper_span <= 0.0
        {
            return Err(ControllerError::InvalidInputRange);
        }
        Ok(())
    }

    /// Clamp and normalize an input to [-1, 1], preserving neutral at exactly 0.
    pub fn normalize(self, input: f64) -> Result<f64, ControllerError> {
        self.validate()?;
        if !input.is_finite() {
            return Err(ControllerError::NonFiniteInput);
        }

        let value = input.clamp(self.minimum, self.maximum);
        let normalized = if value <= self.neutral {
            -1.0 + (value - self.minimum) / (self.neutral - self.minimum)
        } else {
            (value - self.neutral) / (self.maximum - self.neutral)
        };

        if normalized.is_finite() {
            Ok(normalized.clamp(-1.0, 1.0))
        } else {
            Err(ControllerError::ArithmeticOverflow)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EaseCurve {
    Linear,
    Smoothstep,
}

impl EaseCurve {
    fn apply(self, value: f64) -> f64 {
        let t = value.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::Smoothstep => t * t * (3.0 - 2.0 * t),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerMode {
    /// Return the absolute value/offset declared by the target anchors.
    AbsolutePose,
    /// Return a geometry offset relative to the target's neutral anchor.
    RelativeDelta,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControllerTarget {
    SymbolFrame {
        symbol_instance_id: String,
        negative_frame: u32,
        neutral_frame: u32,
        positive_frame: u32,
    },
    GeometryDeform {
        element_id: String,
        negative_offset: Vec2,
        neutral_offset: Vec2,
        positive_offset: Vec2,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControllerBinding {
    /// Unique identity of this binding (not the controller/bone identity).
    pub id: String,
    /// Stable host identity of the input controller/bone.
    pub controller_id: String,
    pub input_range: InputRange,
    pub easing: EaseCurve,
    pub mode: ControllerMode,
    pub target: ControllerTarget,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PreviewResult {
    SymbolFrame {
        symbol_instance_id: String,
        frame: u32,
    },
    GeometryDeform {
        element_id: String,
        offset: Vec2,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControllerError {
    EmptyBindingId,
    EmptyControllerId,
    EmptyTargetId,
    InvalidInputRange,
    NonFiniteInput,
    NonFiniteTargetValue,
    RelativeDeltaUnsupportedForSymbolFrame,
    DuplicateBindingId(String),
    ArithmeticOverflow,
}

impl fmt::Display for ControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBindingId => write!(f, "binding ID must not be empty"),
            Self::EmptyControllerId => write!(f, "controller ID must not be empty"),
            Self::EmptyTargetId => write!(f, "target ID must not be empty"),
            Self::InvalidInputRange => write!(f, "input range must be finite and strictly ordered"),
            Self::NonFiniteInput => write!(f, "controller input must be finite"),
            Self::NonFiniteTargetValue => write!(f, "target anchors must contain only finite values"),
            Self::RelativeDeltaUnsupportedForSymbolFrame => {
                write!(f, "RelativeDelta is not supported for SymbolFrame targets")
            }
            Self::DuplicateBindingId(id) => write!(f, "duplicate binding ID: {id}"),
            Self::ArithmeticOverflow => write!(f, "controller evaluation produced a non-finite result"),
        }
    }
}

impl Error for ControllerError {}

impl ControllerBinding {
    pub fn validate(&self) -> Result<(), ControllerError> {
        if self.id.trim().is_empty() {
            return Err(ControllerError::EmptyBindingId);
        }
        if self.controller_id.trim().is_empty() {
            return Err(ControllerError::EmptyControllerId);
        }
        self.input_range.validate()?;

        match &self.target {
            ControllerTarget::SymbolFrame { symbol_instance_id, .. } => {
                if symbol_instance_id.trim().is_empty() {
                    return Err(ControllerError::EmptyTargetId);
                }
                if self.mode == ControllerMode::RelativeDelta {
                    return Err(ControllerError::RelativeDeltaUnsupportedForSymbolFrame);
                }
            }
            ControllerTarget::GeometryDeform {
                element_id,
                negative_offset,
                neutral_offset,
                positive_offset,
            } => {
                if element_id.trim().is_empty() {
                    return Err(ControllerError::EmptyTargetId);
                }
                if !negative_offset.is_finite()
                    || !neutral_offset.is_finite()
                    || !positive_offset.is_finite()
                {
                    return Err(ControllerError::NonFiniteTargetValue);
                }
            }
        }
        Ok(())
    }

    /// Evaluate one explicit controller input. This function never mutates the binding.
    /// A disabled binding intentionally produces an empty preview result.
    pub fn evaluate(&self, input: f64) -> Result<Option<PreviewResult>, ControllerError> {
        self.validate()?;
        if !self.enabled {
            return Ok(None);
        }

        let normalized = self.input_range.normalize(input)?;
        let eased = if normalized < 0.0 {
            -self.easing.apply(-normalized)
        } else {
            self.easing.apply(normalized)
        };

        let result = match &self.target {
            ControllerTarget::SymbolFrame {
                symbol_instance_id,
                negative_frame,
                neutral_frame,
                positive_frame,
            } => {
                let frame = if eased < 0.0 {
                    interpolate(
                        *negative_frame as f64,
                        *neutral_frame as f64,
                        eased + 1.0,
                    )?
                } else {
                    interpolate(*neutral_frame as f64, *positive_frame as f64, eased)?
                };
                PreviewResult::SymbolFrame {
                    symbol_instance_id: symbol_instance_id.clone(),
                    frame: frame.round() as u32,
                }
            }
            ControllerTarget::GeometryDeform {
                element_id,
                negative_offset,
                neutral_offset,
                positive_offset,
            } => {
                let absolute = if eased < 0.0 {
                    interpolate_vec2(*negative_offset, *neutral_offset, eased + 1.0)?
                } else {
                    interpolate_vec2(*neutral_offset, *positive_offset, eased)?
                };
                let offset = match self.mode {
                    ControllerMode::AbsolutePose => absolute,
                    ControllerMode::RelativeDelta => Vec2 {
                        x: absolute.x - neutral_offset.x,
                        y: absolute.y - neutral_offset.y,
                    },
                };
                if !offset.is_finite() {
                    return Err(ControllerError::ArithmeticOverflow);
                }
                PreviewResult::GeometryDeform {
                    element_id: element_id.clone(),
                    offset,
                }
            }
        };

        Ok(Some(result))
    }
}

/// Validate a collection and reject duplicate binding identities.
pub fn validate_bindings(bindings: &[ControllerBinding]) -> Result<(), ControllerError> {
    let mut ids = HashSet::with_capacity(bindings.len());
    for binding in bindings {
        binding.validate()?;
        if !ids.insert(binding.id.as_str()) {
            return Err(ControllerError::DuplicateBindingId(binding.id.clone()));
        }
    }
    Ok(())
}

fn interpolate(from: f64, to: f64, amount: f64) -> Result<f64, ControllerError> {
    // Weighted interpolation avoids overflowing the endpoint difference.
    let value = from * (1.0 - amount) + to * amount;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ControllerError::ArithmeticOverflow)
    }
}

fn interpolate_vec2(from: Vec2, to: Vec2, amount: f64) -> Result<Vec2, ControllerError> {
    let value = Vec2 {
        x: interpolate(from.x, to.x, amount)?,
        y: interpolate(from.y, to.y, amount)?,
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ControllerError::ArithmeticOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol_binding() -> ControllerBinding {
        ControllerBinding {
            id: "binding-head-turn".into(),
            controller_id: "bone-head-turn".into(),
            input_range: InputRange {
                minimum: -45.0,
                neutral: 0.0,
                maximum: 45.0,
            },
            easing: EaseCurve::Linear,
            mode: ControllerMode::AbsolutePose,
            target: ControllerTarget::SymbolFrame {
                symbol_instance_id: "instance-head".into(),
                negative_frame: 1,
                neutral_frame: 5,
                positive_frame: 9,
            },
            enabled: true,
        }
    }

    fn geometry_binding(mode: ControllerMode) -> ControllerBinding {
        ControllerBinding {
            id: "binding-arm-corrective".into(),
            controller_id: "bone-arm".into(),
            input_range: InputRange {
                minimum: -45.0,
                neutral: 0.0,
                maximum: 45.0,
            },
            easing: EaseCurve::Linear,
            mode,
            target: ControllerTarget::GeometryDeform {
                element_id: "shape-arm".into(),
                negative_offset: Vec2 { x: -4.0, y: 2.0 },
                neutral_offset: Vec2 { x: 1.0, y: 1.0 },
                positive_offset: Vec2 { x: 9.0, y: 5.0 },
            },
            enabled: true,
        }
    }

    #[test]
    fn symbol_frame_maps_negative_neutral_and_positive_inputs() {
        let binding = symbol_binding();
        assert_eq!(
            binding.evaluate(-45.0).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 1 })
        );
        assert_eq!(
            binding.evaluate(0.0).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 5 })
        );
        assert_eq!(
            binding.evaluate(45.0).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 9 })
        );
    }

    #[test]
    fn symbol_frame_interpolates_and_rounds() {
        let binding = symbol_binding();
        assert_eq!(
            binding.evaluate(-22.5).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 3 })
        );
    }

    #[test]
    fn input_is_clamped_to_the_configured_range() {
        let binding = symbol_binding();
        assert_eq!(binding.evaluate(-90.0).unwrap(), binding.evaluate(-45.0).unwrap());
        assert_eq!(binding.evaluate(90.0).unwrap(), binding.evaluate(45.0).unwrap());
    }

    #[test]
    fn smoothstep_preserves_neutral_and_eases_each_side() {
        let mut binding = symbol_binding();
        binding.easing = EaseCurve::Smoothstep;
        assert_eq!(
            binding.evaluate(0.0).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 5 })
        );
        assert_eq!(
            binding.evaluate(22.5).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 7 })
        );
    }

    #[test]
    fn relative_geometry_delta_is_zero_at_neutral() {
        let binding = geometry_binding(ControllerMode::RelativeDelta);
        assert_eq!(
            binding.evaluate(0.0).unwrap(),
            Some(PreviewResult::GeometryDeform {
                element_id: "shape-arm".into(),
                offset: Vec2 { x: 0.0, y: 0.0 }
            })
        );
        assert_eq!(
            binding.evaluate(-45.0).unwrap(),
            Some(PreviewResult::GeometryDeform {
                element_id: "shape-arm".into(),
                offset: Vec2 { x: -5.0, y: 1.0 }
            })
        );
    }

    #[test]
    fn absolute_geometry_returns_the_anchored_offset() {
        let binding = geometry_binding(ControllerMode::AbsolutePose);
        assert_eq!(
            binding.evaluate(45.0).unwrap(),
            Some(PreviewResult::GeometryDeform {
                element_id: "shape-arm".into(),
                offset: Vec2 { x: 9.0, y: 5.0 }
            })
        );
    }

    #[test]
    fn relative_delta_is_rejected_for_symbol_frames() {
        let mut binding = symbol_binding();
        binding.mode = ControllerMode::RelativeDelta;
        assert_eq!(
            binding.validate(),
            Err(ControllerError::RelativeDeltaUnsupportedForSymbolFrame)
        );
    }

    #[test]
    fn invalid_and_overflowing_input_ranges_are_rejected() {
        let invalid = InputRange { minimum: 0.0, neutral: 0.0, maximum: 1.0 };
        assert_eq!(invalid.validate(), Err(ControllerError::InvalidInputRange));

        let overflowing = InputRange {
            minimum: -f64::MAX,
            neutral: 0.0,
            maximum: f64::MAX,
        };
        assert_eq!(overflowing.validate(), Err(ControllerError::InvalidInputRange));
    }

    #[test]
    fn non_finite_input_is_rejected() {
        let binding = symbol_binding();
        assert_eq!(binding.evaluate(f64::NAN), Err(ControllerError::NonFiniteInput));
    }

    #[test]
    fn non_finite_geometry_anchor_is_rejected() {
        let mut binding = geometry_binding(ControllerMode::AbsolutePose);
        if let ControllerTarget::GeometryDeform { positive_offset, .. } = &mut binding.target {
            positive_offset.x = f64::INFINITY;
        }
        assert_eq!(binding.validate(), Err(ControllerError::NonFiniteTargetValue));
    }

    #[test]
    fn relative_delta_detects_subtraction_overflow() {
        let mut binding = geometry_binding(ControllerMode::RelativeDelta);
        if let ControllerTarget::GeometryDeform {
            negative_offset,
            neutral_offset,
            ..
        } = &mut binding.target
        {
            negative_offset.x = -f64::MAX;
            neutral_offset.x = f64::MAX;
        }
        assert_eq!(binding.evaluate(-45.0), Err(ControllerError::ArithmeticOverflow));
    }

    #[test]
    fn disabled_binding_returns_no_preview() {
        let mut binding = symbol_binding();
        binding.enabled = false;
        assert_eq!(binding.evaluate(0.0).unwrap(), None);
    }

    #[test]
    fn binding_collection_rejects_duplicate_ids() {
        let first = symbol_binding();
        let mut second = geometry_binding(ControllerMode::AbsolutePose);
        second.id = first.id.clone();
        assert_eq!(
            validate_bindings(&[first, second]),
            Err(ControllerError::DuplicateBindingId("binding-head-turn".into()))
        );
    }

    #[test]
    fn repeated_evaluation_is_deterministic_and_does_not_mutate_binding() {
        let binding = geometry_binding(ControllerMode::RelativeDelta);
        let original = binding.clone();
        let first = binding.evaluate(18.0).unwrap();
        let second = binding.evaluate(18.0).unwrap();
        assert_eq!(first, second);
        assert_eq!(binding, original);
    }
}
