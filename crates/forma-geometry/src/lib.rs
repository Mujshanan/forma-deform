//! Host-independent geometric value primitives shared by Forma crates.
//!
//! This crate intentionally does not assign a coordinate space, unit, or
//! document ownership to its values. Those meanings belong to the API that
//! carries the value.

/// A two-component geometric value.
///
/// The type is deliberately a small data primitive. Whether a value represents
/// a point, displacement, direction, or another pair—and which coordinate
/// space it belongs to—is defined by the surrounding API.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    /// Construct a two-component value.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Return whether both components are finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::Vec2;

    #[test]
    fn constructs_and_preserves_components() {
        assert_eq!(Vec2::new(1.25, -4.5), Vec2 { x: 1.25, y: -4.5 });
    }

    #[test]
    fn finite_check_rejects_non_finite_components() {
        assert!(Vec2::new(1.0, -2.0).is_finite());
        assert!(!Vec2::new(f64::NAN, 0.0).is_finite());
        assert!(!Vec2::new(0.0, f64::INFINITY).is_finite());
    }
}
