//! Ordered, host-independent two-dimensional pose value contract.
use crate::{error::CoreError, ids::BindingId, value::TypedValue};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

/// A pose is an ordered list of binding/property values, not a frame number or
/// an opaque host-state override. The order is preserved for deterministic
/// serialization and presentation; matching is always performed by BindingId.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pose2D {
    values: Vec<(BindingId, TypedValue)>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pose2DWire {
    values: Vec<(BindingId, TypedValue)>,
}

impl<'de> Deserialize<'de> for Pose2D {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = Pose2DWire::deserialize(deserializer)?;
        Self::new(wire.values).map_err(de::Error::custom)
    }
}

impl Pose2D {
    pub fn new(values: Vec<(BindingId, TypedValue)>) -> Result<Self, CoreError> {
        let mut ids = HashSet::with_capacity(values.len());
        for (id, _) in &values {
            if !ids.insert(id) {
                return Err(CoreError::DuplicatePoseBindingId(id.to_string()));
            }
        }
        Ok(Self { values })
    }

    pub fn values(&self) -> &[(BindingId, TypedValue)] {
        &self.values
    }

    pub fn iter(&self) -> impl Iterator<Item = (&BindingId, &TypedValue)> {
        self.values.iter().map(|(id, value)| (id, value))
    }

    pub fn get(&self, id: &BindingId) -> Option<&TypedValue> {
        self.values.iter().find(|(binding_id, _)| binding_id == id).map(|(_, value)| value)
    }

    /// Verify that two poses contain the same binding IDs with compatible
    /// value types and angle semantics. Vector order may differ.
    pub fn validate_compatible(&self, other: &Self) -> Result<(), CoreError> {
        for (id, value) in &self.values {
            let other_value = other.get(id).ok_or_else(|| CoreError::PoseBindingMissing {
                binding_id: id.to_string(),
            })?;
            if value.kind_name() != other_value.kind_name() {
                return Err(CoreError::PoseValueTypeMismatch {
                    binding_id: id.to_string(),
                    expected: value.kind_name(),
                    actual: other_value.kind_name(),
                });
            }
            if let (TypedValue::Angle(left), TypedValue::Angle(right)) = (*value, *other_value) {
                if left.is_wrapped() != right.is_wrapped() {
                    return Err(CoreError::PoseAngleSemanticsMismatch { binding_id: id.to_string() });
                }
            }
        }
        for (id, _) in &other.values {
            if self.get(id).is_none() {
                return Err(CoreError::PoseBindingMissing { binding_id: id.to_string() });
            }
        }
        Ok(())
    }
}
