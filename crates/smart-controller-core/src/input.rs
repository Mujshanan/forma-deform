//! Explicit, host-independent input definitions and immutable sample snapshots.
//!
//! Freshness is based only on caller-supplied monotonic tick ages. This module
//! never reads a clock and never invents freshness when age information is absent.

use crate::{
    error::CoreError,
    ids::{BindingId, ControllerId, InputId},
    value::TypedValue,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputDimension {
    Scalar,
    Vector2,
    Angle,
}

impl InputDimension {
    pub const fn accepts(self, value: TypedValue) -> bool {
        matches!(
            (self, value),
            (Self::Scalar, TypedValue::Scalar(_))
                | (Self::Vector2, TypedValue::Vector2(_))
                | (Self::Angle, TypedValue::Angle(_))
        )
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Vector2 => "vector2",
            Self::Angle => "angle",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoneTransformChannel {
    Rotation,
    Translation,
    Scale,
}

/// Describes where an input originates without importing host SDK types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputSource {
    Manual,
    /// Transitional scalar source for imported V1 controller IDs whose units were host-defined.
    LegacyScalar { source_id: String },
    BoneTransform {
        bone_id: String,
        channel: BoneTransformChannel,
    },
    ControllerOutput {
        controller_id: ControllerId,
        binding_id: BindingId,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum FallbackPolicy {
    /// Do not substitute a value; the input remains unavailable.
    Unavailable,
    /// Use this explicitly authored value when the sample cannot be used.
    UseValue(TypedValue),
    /// Treat a missing, invalid, unavailable or stale sample as an error.
    Reject,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputDefinition {
    pub id: InputId,
    pub source: InputSource,
    pub dimension: InputDimension,
    pub fallback: FallbackPolicy,
}

impl InputDefinition {
    pub fn validate(&self) -> Result<(), CoreError> {
        if let InputSource::BoneTransform { bone_id, channel } = &self.source {
            if bone_id.trim().is_empty() {
                return Err(CoreError::InvalidModel(
                    "bone transform input requires a non-empty bone_id".into(),
                ));
            }
            let expected = match channel {
                BoneTransformChannel::Rotation => InputDimension::Angle,
                BoneTransformChannel::Translation | BoneTransformChannel::Scale => {
                    InputDimension::Vector2
                }
            };
            if self.dimension != expected {
                return Err(CoreError::InputValueTypeMismatch {
                    input_id: self.id.to_string(),
                    expected: expected.name(),
                    actual: self.dimension.name(),
                });
            }
        }

        if let FallbackPolicy::UseValue(value) = &self.fallback {
            if !self.dimension.accepts(*value) {
                return Err(CoreError::InputValueTypeMismatch {
                    input_id: self.id.to_string(),
                    expected: self.dimension.name(),
                    actual: value.kind_name(),
                });
            }
        }
        Ok(())
    }

    pub fn validate_sample(&self, sample: &InputSample) -> Result<(), CoreError> {
        sample.validate()?;
        if sample.id != self.id {
            return Err(CoreError::InvalidInputSample {
                input_id: sample.id.to_string(),
                reason: "sample ID does not match its input definition",
            });
        }
        if let Some(value) = sample.value {
            if !self.dimension.accepts(value) {
                return Err(CoreError::InputValueTypeMismatch {
                    input_id: self.id.to_string(),
                    expected: self.dimension.name(),
                    actual: value.kind_name(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputValidity {
    Valid,
    Missing,
    Invalid,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSample {
    pub id: InputId,
    pub value: Option<TypedValue>,
    pub validity: InputValidity,
    /// Age in caller-defined monotonic ticks. None means freshness is unknown.
    pub age_ticks: Option<u64>,
}

impl InputSample {
    pub fn valid(id: InputId, value: TypedValue, age_ticks: Option<u64>) -> Self {
        Self {
            id,
            value: Some(value),
            validity: InputValidity::Valid,
            age_ticks,
        }
    }

    pub fn missing(id: InputId) -> Self {
        Self {
            id,
            value: None,
            validity: InputValidity::Missing,
            age_ticks: None,
        }
    }

    pub fn invalid(id: InputId) -> Self {
        Self {
            id,
            value: None,
            validity: InputValidity::Invalid,
            age_ticks: None,
        }
    }

    pub fn unavailable(id: InputId) -> Self {
        Self {
            id,
            value: None,
            validity: InputValidity::Unavailable,
            age_ticks: None,
        }
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        let has_value = self.value.is_some();
        let status_requires_value = self.validity == InputValidity::Valid;
        if has_value != status_requires_value {
            return Err(CoreError::InvalidInputSample {
                input_id: self.id.to_string(),
                reason: "only a valid sample may contain a value",
            });
        }
        Ok(())
    }

    pub const fn freshness(&self, max_age_ticks: u64) -> Freshness {
        match self.age_ticks {
            Some(age) if age > max_age_ticks => Freshness::Stale,
            Some(_) => Freshness::Fresh,
            None => Freshness::Unknown,
        }
    }
}

/// A complete, immutable collection of samples for one evaluation invocation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputSnapshot {
    pub max_age_ticks: u64,
    pub samples: Vec<InputSample>,
}

impl InputSnapshot {
    pub fn new(max_age_ticks: u64, samples: Vec<InputSample>) -> Result<Self, CoreError> {
        let snapshot = Self {
            max_age_ticks,
            samples,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        let mut ids = HashSet::with_capacity(self.samples.len());
        for sample in &self.samples {
            sample.validate()?;
            if !ids.insert(sample.id.clone()) {
                return Err(CoreError::DuplicateInputId(sample.id.to_string()));
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &InputId) -> Option<&InputSample> {
        self.samples.iter().find(|sample| &sample.id == id)
    }

    pub const fn max_age_ticks(&self) -> u64 {
        self.max_age_ticks
    }

    pub fn freshness(&self, sample: &InputSample) -> Freshness {
        sample.freshness(self.max_age_ticks)
    }
}
