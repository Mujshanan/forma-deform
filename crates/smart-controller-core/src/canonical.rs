//! Canonical, host-independent controller configuration.
use crate::{contribution::Contribution, error::CoreError, ids::{ControllerId, InputId}, value::TypedValue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MappingConfig {
    Range1D { minimum: f64, neutral: f64, maximum: f64 },
    DirectedRange1D { start: f64, end: f64 },
    PoseSet1D { pose_ids: Vec<String> },
    JoystickFivePose { center: String, up: String, right: String, down: String, left: String },
    Discrete { values: Vec<TypedValue> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerBinding {
    pub id: crate::ids::BindingId,
    pub target_id: crate::ids::TargetId,
    pub mode: crate::contribution::CompositionMode,
    pub base_value: Option<TypedValue>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Controller {
    pub id: ControllerId,
    pub enabled: bool,
    pub inputs: Vec<InputId>,
    pub dependencies: Vec<ControllerId>,
    pub mapping: MappingConfig,
    pub bindings: Vec<ControllerBinding>,
    /// Shared DAG nodes cannot carry branch-specific configuration.
    #[serde(default)]
    pub parent_scoped: bool,
}

impl Controller {
    pub fn validate_local(&self) -> Result<(), CoreError> {
        if self.inputs.is_empty() {
            return Err(CoreError::InvalidModel(format!("controller {} has no inputs", self.id)));
        }
        if self.dependencies.iter().any(|id| id == &self.id) {
            return Err(CoreError::InvalidGraph(format!("controller {} depends on itself", self.id)));
        }
        let mut ids = std::collections::HashSet::new();
        for binding in &self.bindings {
            if !ids.insert(binding.id.clone()) {
                return Err(CoreError::InvalidModel(format!("duplicate binding ID {}", binding.id)));
            }
            if binding.mode == crate::contribution::CompositionMode::Additive && binding.base_value.is_none() {
                return Err(CoreError::InvalidModel(format!("additive binding {} requires an explicit base value", binding.id)));
            }
            if binding.mode != crate::contribution::CompositionMode::Additive && binding.base_value.is_some() {
                return Err(CoreError::InvalidModel(format!("base value is only valid for additive binding {}", binding.id)));
            }
        }
        match &self.mapping {
            MappingConfig::Range1D { minimum, neutral, maximum } => {
                if !minimum.is_finite() || !neutral.is_finite() || !maximum.is_finite() || minimum >= neutral || neutral >= maximum {
                    return Err(CoreError::InvalidMapping("Range1D endpoints must be finite and strictly ordered".into()));
                }
            }
            MappingConfig::DirectedRange1D { start, end } => {
                if !start.is_finite() || !end.is_finite() || start == end {
                    return Err(CoreError::InvalidMapping("DirectedRange1D endpoints must be distinct and finite".into()));
                }
            }
            MappingConfig::PoseSet1D { pose_ids } => if pose_ids.is_empty() { return Err(CoreError::InvalidMapping("PoseSet1D requires at least one pose".into())); },
            MappingConfig::JoystickFivePose { center, up, right, down, left } => {
                let ids = [center, up, right, down, left];
                if ids.iter().any(|id| id.trim().is_empty()) || ids.iter().collect::<std::collections::HashSet<_>>().len() != 5 {
                    return Err(CoreError::InvalidMapping("five-pose joystick requires five distinct non-empty pose IDs".into()));
                }
            }
            MappingConfig::Discrete { values } => if values.is_empty() { return Err(CoreError::InvalidMapping("Discrete mapping requires at least one value".into())); },
        }
        Ok(())
    }
}
