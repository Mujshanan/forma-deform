//! Typed output contributions. Application/composition is implemented in Phase 4.
use crate::{error::CoreError, ids::{BindingId, ControllerId, TargetId}, value::TypedValue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionMode { Replace, Additive, WeightedBlend, PriorityOverride }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum BaseRef { Neutral, Authored, Input(String) }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contribution {
    pub controller_id: ControllerId,
    pub binding_id: BindingId,
    pub target_id: TargetId,
    pub value: TypedValue,
    pub mode: CompositionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<BaseRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_value: Option<TypedValue>,
}

impl Contribution {
    /// Enforces that mode-specific fields are present only for their mode.
    pub fn validate(&self) -> Result<(), CoreError> {
        let valid = match self.mode {
            CompositionMode::Replace => self.weight.is_none() && self.priority.is_none() && self.base_ref.is_none() && self.base_value.is_none(),
            CompositionMode::Additive => self.weight.is_none() && self.priority.is_none() && self.base_ref.is_some() && match self.base_ref.as_ref() { Some(BaseRef::Authored) => self.base_value.is_some(), Some(BaseRef::Neutral | BaseRef::Input(_)) => self.base_value.is_none(), None => false },
            CompositionMode::WeightedBlend => self.weight.is_some_and(|w| w.is_finite() && w >= 0.0) && self.priority.is_none() && self.base_ref.is_none() && self.base_value.is_none(),
            CompositionMode::PriorityOverride => self.priority.is_some() && self.weight.is_none() && self.base_ref.is_none() && self.base_value.is_none(),
        };
        if self.mode == CompositionMode::WeightedBlend && self.weight.is_some_and(|w| !w.is_finite() || w < 0.0) {
            return Err(CoreError::CompositionWeightInvalid { binding_id: self.binding_id.to_string() });
        }
        if let Some(BaseRef::Input(id)) = &self.base_ref {
            if id.trim().is_empty() {
                return Err(CoreError::InvalidModel("contribution input base reference must not be empty".into()));
            }
        }
        if !valid {
            return Err(CoreError::InvalidModel(format!("invalid fields for {:?} contribution", self.mode)));
        }
        Ok(())
    }
}
