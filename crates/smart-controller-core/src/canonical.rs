//! Canonical, host-independent controller configuration.
use crate::{
    error::CoreError,
    model::EndpointPolicy,
    ids::{BindingId, ControllerId, InputId, TargetId},
    value::TypedValue,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MappingConfig {
    Range1D { minimum: f64, neutral: f64, maximum: f64 },
    DirectedRange1D { start: f64, end: f64 },
    PoseSet1D { pose_ids: Vec<String> },
    JoystickFivePose { center: String, up: String, right: String, down: String, left: String },
    Discrete { values: Vec<TypedValue> },
    ThreeAnchorRange { minimum: f64, neutral: f64, maximum: f64, easing: EasingConfig },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EasingConfig { Linear, Smoothstep }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TargetMapping {
    SymbolFrame { symbol_instance_id: String, negative_frame: u32, neutral_frame: u32, positive_frame: u32 },
    FrameRange { action_layer_id: u64, start_frame: u32, end_frame: u32, endpoint_policy: EndpointPolicy },
    GeometryDeform { element_id: String, pin_id: String, negative_offset: [f64; 2], neutral_offset: [f64; 2], positive_offset: [f64; 2] },
    Property { target_id: String, property: String, scale: f64, additive: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingMode { AbsolutePose, RelativeDelta }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerBinding {
    pub id: BindingId,
    pub target_id: TargetId,
    pub target: TargetMapping,
    pub mode: BindingMode,
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
        let mut input_ids = std::collections::HashSet::new();
        if self.inputs.iter().any(|id| !input_ids.insert(id)) {
            return Err(CoreError::InvalidModel(format!("controller {} contains duplicate input references", self.id)));
        }
        let mut ids = std::collections::HashSet::new();
        for binding in &self.bindings {
            if !ids.insert(binding.id.clone()) {
                return Err(CoreError::InvalidModel(format!("duplicate binding ID {}", binding.id)));
            }
            if binding.mode == BindingMode::RelativeDelta && binding.base_value.is_none() {
                return Err(CoreError::InvalidModel(format!("relative binding {} requires an explicit base value", binding.id)));
            }
            if binding.mode == BindingMode::AbsolutePose && binding.base_value.is_some() {
                return Err(CoreError::InvalidModel(format!("base value is only valid for relative binding {}", binding.id)));
            }
            match &binding.target {
                TargetMapping::SymbolFrame { .. } | TargetMapping::FrameRange { .. }
                    if binding.mode != BindingMode::AbsolutePose =>
                    return Err(CoreError::InvalidModel(format!("frame target binding {} must use AbsolutePose", binding.id))),
                TargetMapping::GeometryDeform { .. } if binding.mode == BindingMode::RelativeDelta
                    && !matches!(binding.base_value.as_ref(), Some(TypedValue::Vector2(_))) =>
                    return Err(CoreError::InvalidModel(format!("relative geometry binding {} requires a Vector2 base", binding.id))),
                TargetMapping::Property { additive, .. } if *additive != (binding.mode == BindingMode::RelativeDelta) =>
                    return Err(CoreError::InvalidModel(format!("property target additive flag conflicts with binding mode for {}", binding.id))),
                TargetMapping::Property { .. } if binding.mode == BindingMode::RelativeDelta
                    && !matches!(binding.base_value.as_ref(), Some(TypedValue::Scalar(_))) =>
                    return Err(CoreError::InvalidModel(format!("relative property binding {} requires a scalar base", binding.id))),
                TargetMapping::FrameRange { action_layer_id, start_frame, end_frame, .. } if *action_layer_id == 0 || end_frame <= start_frame =>
                    return Err(CoreError::InvalidModel("frame range requires a nonzero layer and increasing endpoints".into())),
                TargetMapping::SymbolFrame { symbol_instance_id, .. } if symbol_instance_id.trim().is_empty() =>
                    return Err(CoreError::InvalidModel("symbol frame target requires an identity".into())),
                TargetMapping::GeometryDeform { element_id, pin_id, negative_offset, neutral_offset, positive_offset } => {
                    if element_id.trim().is_empty() || pin_id.trim().is_empty() ||
                        [negative_offset, neutral_offset, positive_offset].iter().flatten().any(|v| !v.is_finite()) {
                        return Err(CoreError::InvalidModel("geometry target identity and anchors must be valid".into()));
                    }
                }
                TargetMapping::Property { target_id, property, scale, .. } if target_id.trim().is_empty() || property.trim().is_empty() || !scale.is_finite() =>
                    return Err(CoreError::InvalidModel("property target identity, property and scale must be valid".into())),
                _ => {}
            }
        }
        match &self.mapping {
            MappingConfig::Range1D { minimum, neutral, maximum } => {
                if !minimum.is_finite() || !neutral.is_finite() || !maximum.is_finite() || !(*neutral - *minimum).is_finite() || !(*maximum - *neutral).is_finite() || minimum >= neutral || neutral >= maximum {
                    return Err(CoreError::InvalidMapping("Range1D endpoints must be finite and strictly ordered".into()));
                }
            }
            MappingConfig::DirectedRange1D { start, end } => {
                if !start.is_finite() || !end.is_finite() || !(*end - *start).is_finite() || start == end {
                    return Err(CoreError::InvalidMapping("DirectedRange1D endpoints must be distinct and finite".into()));
                }
            }
            MappingConfig::PoseSet1D { pose_ids } => if pose_ids.is_empty() || pose_ids.iter().any(|id| id.trim().is_empty()) { return Err(CoreError::InvalidMapping("PoseSet1D requires non-empty pose IDs".into())); },
            MappingConfig::JoystickFivePose { center, up, right, down, left } => {
                let ids = [center, up, right, down, left];
                if ids.iter().any(|id| id.trim().is_empty()) || ids.iter().collect::<std::collections::HashSet<_>>().len() != 5 {
                    return Err(CoreError::InvalidMapping("five-pose joystick requires five distinct non-empty pose IDs".into()));
                }
            }
            MappingConfig::Discrete { values } => if values.is_empty() { return Err(CoreError::InvalidMapping("Discrete mapping requires at least one value".into())); },
            MappingConfig::ThreeAnchorRange { minimum, neutral, maximum, .. } => {
                if !minimum.is_finite() || !neutral.is_finite() || !maximum.is_finite() || !(*neutral - *minimum).is_finite() || !(*maximum - *neutral).is_finite() || minimum >= neutral || neutral >= maximum {
                    return Err(CoreError::InvalidMapping("three-anchor range must be finite and strictly ordered".into()));
                }
            },
        }
        Ok(())
    }
}
