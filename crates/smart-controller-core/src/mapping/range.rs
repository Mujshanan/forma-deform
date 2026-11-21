//! Pure scalar range mapping with explicit finite-value checks.
use crate::{error::CoreError, value::Scalar};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThreePointRange { pub minimum: Scalar, pub neutral: Scalar, pub maximum: Scalar }

impl ThreePointRange {
    pub fn new(minimum: f64, neutral: f64, maximum: f64) -> Result<Self, CoreError> {
        let (minimum, neutral, maximum) = (Scalar::new(minimum)?, Scalar::new(neutral)?, Scalar::new(maximum)?);
        if neutral.get() <= minimum.get() || maximum.get() <= neutral.get() {
            return Err(CoreError::InvalidMapping("three-point range must be strictly ordered".into()));
        }
        Ok(Self { minimum, neutral, maximum })
    }

    /// Clamps to the authored range and maps minimum/neutral/maximum to -1/0/1.
    pub fn normalize_signed(self, input: f64) -> Result<f64, CoreError> {
        if !input.is_finite() { return Err(CoreError::NonFiniteValue { context: "range input" }); }
        let min = self.minimum.get(); let mid = self.neutral.get(); let max = self.maximum.get();
        let x = input.clamp(min, max);
        let result = if x <= mid { (x-mid)/(mid-min) } else { (x-mid)/(max-mid) };
        if result.is_finite() { Ok(result) } else { Err(CoreError::InvalidMapping("range normalization overflow".into())) }
    }
}

/// Normalize an arbitrary directed interval to [0, 1]; reversed endpoints are valid.
pub fn normalize_directed(input: f64, start: f64, end: f64) -> Result<f64, CoreError> {
    if !input.is_finite() || !start.is_finite() || !end.is_finite() || start == end {
        return Err(CoreError::InvalidMapping("input and distinct endpoints must be finite".into()));
    }
    let clamped = input.clamp(start.min(end), start.max(end));
    let value = (clamped-start)/(end-start);
    if value.is_finite() { Ok(value.clamp(0.0,1.0)) }
    else { Err(CoreError::InvalidMapping("directed normalization overflow".into())) }
}
