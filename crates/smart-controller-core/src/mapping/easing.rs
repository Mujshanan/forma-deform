//! Deterministic easing primitives shared by scalar mappings.
use crate::error::CoreError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing { Linear, Smoothstep }

impl Easing {
    pub fn apply(self, value: f64) -> Result<f64, CoreError> {
        if !value.is_finite() {
            return Err(CoreError::NonFiniteValue { context: "easing input" });
        }
        let t = value.clamp(0.0, 1.0);
        Ok(match self { Self::Linear => t, Self::Smoothstep => t*t*(3.0-2.0*t) })
    }
}
