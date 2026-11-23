//! Deterministic, non-accumulating bone-to-warp-pin deformation math.

pub mod composition_evaluation;
pub mod geometry;
pub mod composition_graph;
pub mod evaluation_contract;
pub mod host_contract;
pub mod path2d;
pub mod persistence;
pub mod topology;
pub mod regular_lattice;
pub mod irregular_cage;
pub mod topology_edit;
pub mod lattice_edit;
pub mod embedding;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 { pub x: f64, pub y: f64 }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeformSpec {
    /// Authored neutral pin position in AniCraft stage coordinates.
    pub neutral_position: Vec2,
    pub neutral_angle_degrees: f64,
    /// Stage-space displacement per degree of bone rotation.
    pub gain_per_degree: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeformError { NonFiniteInput, ArithmeticOverflow }

impl std::fmt::Display for DeformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "deformation inputs must be finite"),
            Self::ArithmeticOverflow => write!(f, "deformation result is not finite"),
        }
    }
}
impl std::error::Error for DeformError {}

/// Calculate an absolute stage-space pin position from an authored neutral position.
/// Repeated evaluations do not accumulate displacement.
pub fn evaluate_pin_position(spec: DeformSpec, current_angle_degrees: f64) -> Result<Vec2, DeformError> {
    let values = [spec.neutral_position.x, spec.neutral_position.y,
        spec.neutral_angle_degrees, spec.gain_per_degree.x, spec.gain_per_degree.y,
        current_angle_degrees];
    if values.iter().any(|value| !value.is_finite()) { return Err(DeformError::NonFiniteInput); }
    let delta = current_angle_degrees - spec.neutral_angle_degrees;
    if !delta.is_finite() { return Err(DeformError::ArithmeticOverflow); }
    let result = Vec2 {
        x: spec.neutral_position.x + delta * spec.gain_per_degree.x,
        y: spec.neutral_position.y + delta * spec.gain_per_degree.y,
    };
    if result.x.is_finite() && result.y.is_finite() { Ok(result) }
    else { Err(DeformError::ArithmeticOverflow) }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec() -> DeformSpec {
        DeformSpec {
            neutral_position: Vec2 { x: 100.0, y: 50.0 },
            neutral_angle_degrees: 0.0,
            gain_per_degree: Vec2 { x: 0.5, y: -0.25 },
        }
    }
    #[test]
    fn neutral_pose_returns_authored_neutral_position() {
        assert_eq!(evaluate_pin_position(spec(), 0.0).unwrap(), Vec2 { x: 100.0, y: 50.0 });
    }
    #[test]
    fn positive_and_negative_angles_are_deterministic() {
        assert_eq!(evaluate_pin_position(spec(), 20.0).unwrap(), Vec2 { x: 110.0, y: 45.0 });
        assert_eq!(evaluate_pin_position(spec(), -20.0).unwrap(), Vec2 { x: 90.0, y: 55.0 });
    }
    #[test]
    fn repeated_evaluation_does_not_accumulate() {
        assert_eq!(evaluate_pin_position(spec(), 35.0).unwrap(), evaluate_pin_position(spec(), 35.0).unwrap());
    }
    #[test]
    fn non_finite_values_are_rejected() {
        assert_eq!(evaluate_pin_position(spec(), f64::NAN), Err(DeformError::NonFiniteInput));
    }
}
