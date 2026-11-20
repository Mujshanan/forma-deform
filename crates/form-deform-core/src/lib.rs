//! Deterministic, non-accumulating deformation model.
//!
//! This crate contains only domain data and mathematical evaluation. Host protocols,
//! serialization, JSON, and WASM concerns belong in plugin adapters.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

/// Authored deformation parameters for one pin in stage coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformSpec {
    /// Authored neutral pin position in stage coordinates (pixels).
    pub neutral_position: Vec2,
    /// Bone angle at which the pin is at its neutral position (degrees).
    pub neutral_angle_degrees: f64,
    /// Stage-space displacement per degree of bone rotation (pixels/degree).
    pub gain_per_degree: Vec2,
}

/// The evaluated bone-angle data supplied to a deformation request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneAngle {
    /// Constrained/keyed bone angle at the evaluation frame (degrees).
    pub keyed_degrees: f64,
    /// Separately evaluated spring swing (degrees), when the host provides it.
    pub spring_swing_degrees: Option<f64>,
}

/// Complete input to one deterministic pin-deformation evaluation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformRequest {
    pub spec: DeformSpec,
    pub bone_angle: BoneAngle,
    /// Whether the supplied spring swing participates in the input angle.
    pub include_spring_swing: bool,
}

/// Fully specified output of a deformation evaluation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformResult {
    pub keyed_angle_degrees: f64,
    pub spring_swing_degrees: Option<f64>,
    pub effective_angle_degrees: f64,
    pub angle_delta_degrees: f64,
    /// Absolute target position; never relative to the currently displayed pin.
    pub target_position: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeformError {
    NonFiniteInput,
    MissingSpringSwing,
    ArithmeticOverflow,
}

impl std::fmt::Display for DeformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "deformation inputs must be finite"),
            Self::MissingSpringSwing => write!(f, "spring swing was requested but is unavailable"),
            Self::ArithmeticOverflow => write!(f, "deformation result is not finite"),
        }
    }
}
impl std::error::Error for DeformError {}

/// Evaluate an absolute stage-space pin position from an authored neutral position.
pub fn evaluate_deformation(request: DeformRequest) -> Result<DeformResult, DeformError> {
    let spec = request.spec;
    let keyed = request.bone_angle.keyed_degrees;
    let spring = request.bone_angle.spring_swing_degrees;
    let values = [
        spec.neutral_position.x,
        spec.neutral_position.y,
        spec.neutral_angle_degrees,
        spec.gain_per_degree.x,
        spec.gain_per_degree.y,
        keyed,
    ];
    if values.iter().any(|value| !value.is_finite())
        || spring.is_some_and(|value| !value.is_finite())
    {
        return Err(DeformError::NonFiniteInput);
    }
    if request.include_spring_swing && spring.is_none() {
        return Err(DeformError::MissingSpringSwing);
    }

    let effective_angle = keyed + if request.include_spring_swing { spring.unwrap_or(0.0) } else { 0.0 };
    if !effective_angle.is_finite() {
        return Err(DeformError::ArithmeticOverflow);
    }
    let delta = effective_angle - spec.neutral_angle_degrees;
    if !delta.is_finite() {
        return Err(DeformError::ArithmeticOverflow);
    }
    let target = Vec2 {
        x: spec.neutral_position.x + delta * spec.gain_per_degree.x,
        y: spec.neutral_position.y + delta * spec.gain_per_degree.y,
    };
    if !target.x.is_finite() || !target.y.is_finite() {
        return Err(DeformError::ArithmeticOverflow);
    }

    Ok(DeformResult {
        keyed_angle_degrees: keyed,
        spring_swing_degrees: spring,
        effective_angle_degrees: effective_angle,
        angle_delta_degrees: delta,
        target_position: target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(keyed_degrees: f64) -> DeformRequest {
        DeformRequest {
            spec: DeformSpec {
                neutral_position: Vec2 { x: 100.0, y: 50.0 },
                neutral_angle_degrees: 0.0,
                gain_per_degree: Vec2 { x: 0.5, y: -0.25 },
            },
            bone_angle: BoneAngle { keyed_degrees, spring_swing_degrees: Some(4.0) },
            include_spring_swing: false,
        }
    }

    #[test]
    fn neutral_pose_returns_authored_neutral_position() {
        let result = evaluate_deformation(request(0.0)).unwrap();
        assert_eq!(result.target_position, Vec2 { x: 100.0, y: 50.0 });
        assert_eq!(result.effective_angle_degrees, 0.0);
    }

    #[test]
    fn positive_and_negative_angles_are_deterministic() {
        assert_eq!(evaluate_deformation(request(20.0)).unwrap().target_position, Vec2 { x: 110.0, y: 45.0 });
        assert_eq!(evaluate_deformation(request(-20.0)).unwrap().target_position, Vec2 { x: 90.0, y: 55.0 });
    }

    #[test]
    fn spring_swing_is_only_included_when_explicitly_enabled() {
        let mut input = request(10.0);
        assert_eq!(evaluate_deformation(input).unwrap().effective_angle_degrees, 10.0);
        input.include_spring_swing = true;
        let result = evaluate_deformation(input).unwrap();
        assert_eq!(result.effective_angle_degrees, 14.0);
        assert_eq!(result.angle_delta_degrees, 14.0);
    }

    #[test]
    fn requested_but_unavailable_spring_swing_is_an_error() {
        let mut input = request(10.0);
        input.bone_angle.spring_swing_degrees = None;
        input.include_spring_swing = true;
        assert_eq!(evaluate_deformation(input), Err(DeformError::MissingSpringSwing));
    }

    #[test]
    fn repeated_evaluation_does_not_accumulate() {
        let input = request(35.0);
        assert_eq!(evaluate_deformation(input), evaluate_deformation(input));
    }

    #[test]
    fn non_finite_values_are_rejected() {
        assert_eq!(evaluate_deformation(request(f64::NAN)), Err(DeformError::NonFiniteInput));
    }
}
