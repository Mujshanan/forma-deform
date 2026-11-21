//! Explicit migrations from persisted V1 controller formats to the canonical document.
use crate::{
    canonical::{BindingMode, Controller, ControllerBinding, MappingConfig, TargetMapping},
    error::CoreError,
    ids::{BindingId, ControllerId, InputId, TargetId},
    input::{FallbackPolicy, InputDefinition, InputDimension, InputSource},
    legacy::{BindingSet, ControllerMode, ControllerTarget},
    model::{EndpointPolicy, Mode, Params},
    schema::{SchemaIdentity, CURRENT_SCHEMA_VERSION, SCHEMA_NAME},
    validation,
    value::TypedValue,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationReport {
    pub source_format: String,
    pub notices: Vec<String>,
    pub losses: Vec<String>,
}

impl MigrationReport {
    pub fn is_lossless(&self) -> bool { self.losses.is_empty() }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalDocument {
    pub schema: SchemaIdentity,
    pub inputs: Vec<InputDefinition>,
    pub controllers: Vec<Controller>,
}

impl CanonicalDocument {
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema.name != SCHEMA_NAME {
            return Err(CoreError::Serialization(format!("unexpected schema name: {}", self.schema.name)));
        }
        if self.schema.version != CURRENT_SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSchemaVersion(self.schema.version));
        }
        let mut input_ids = std::collections::HashSet::new();
        for input in &self.inputs {
            input.validate()?;
            if !input_ids.insert(input.id.clone()) {
                return Err(CoreError::DuplicateInputId(input.id.to_string()));
            }
        }
        for controller in &self.controllers {
            controller.validate_local()?;
            for input_id in &controller.inputs {
                if !input_ids.contains(input_id) {
                    return Err(CoreError::MissingInput(input_id.to_string()));
                }
            }
        }
        crate::graph::ControllerGraph::new(self.controllers.clone())?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationOutput {
    pub document: CanonicalDocument,
    pub report: MigrationReport,
}

fn empty_document(inputs: Vec<InputDefinition>, controllers: Vec<Controller>) -> CanonicalDocument {
    CanonicalDocument {
        schema: SchemaIdentity { name: SCHEMA_NAME.into(), version: CURRENT_SCHEMA_VERSION },
        inputs,
        controllers,
    }
}

/// Migrate a versioned V1 BindingSet without changing its three-anchor behavior.
/// One canonical controller is emitted per binding so the original per-binding
/// range/easing configuration remains exact and no shared-node scope is invented.
pub fn migrate_binding_set_v1(source: &BindingSet) -> Result<MigrationOutput, CoreError> {
    source.validate().map_err(|e| CoreError::InvalidModel(e.to_string()))?;
    let mut inputs = Vec::with_capacity(source.bindings.len());
    let mut controllers = Vec::with_capacity(source.bindings.len());

    for binding in &source.bindings {
        let controller_id = ControllerId::new(format!("legacy-binding:{}", binding.id))?;
        let input_id = InputId::new(format!("legacy-controller:{}", binding.controller_id))?;
        let (target_id, target, base_value) = match &binding.target {
            ControllerTarget::SymbolFrame { symbol_instance_id, negative_frame, neutral_frame, positive_frame } => (
                TargetId::new(format!("symbol:{symbol_instance_id}"))?,
                TargetMapping::SymbolFrame {
                    symbol_instance_id: symbol_instance_id.clone(),
                    negative_frame: *negative_frame,
                    neutral_frame: *neutral_frame,
                    positive_frame: *positive_frame,
                },
                None,
            ),
            ControllerTarget::GeometryDeform { element_id, pin_id, negative_offset, neutral_offset, positive_offset } => {
                let base = TypedValue::vector2(neutral_offset.x, neutral_offset.y)?;
                (
                    TargetId::new(format!("geometry:{element_id}:{pin_id}"))?,
                    TargetMapping::GeometryDeform {
                        element_id: element_id.clone(),
                        pin_id: pin_id.clone(),
                        negative_offset: [negative_offset.x, negative_offset.y],
                        neutral_offset: [neutral_offset.x, neutral_offset.y],
                        positive_offset: [positive_offset.x, positive_offset.y],
                    },
                    (binding.mode == ControllerMode::RelativeDelta).then_some(base),
                )
            }
        };
        let input_source = InputSource::LegacyScalar { source_id: binding.controller_id.clone() };
        let input = InputDefinition {
            id: input_id.clone(),
            source: input_source,
            dimension: InputDimension::Scalar,
            fallback: FallbackPolicy::Reject,
        };
        input.validate()?;
        inputs.push(input);
        let canonical_binding = ControllerBinding {
            id: BindingId::new(binding.id.clone())?,
            target_id,
            target,
            mode: match binding.mode {
                ControllerMode::AbsolutePose => BindingMode::AbsolutePose,
                ControllerMode::RelativeDelta => BindingMode::RelativeDelta,
            },
            base_value,
        };
        let controller = Controller {
            id: controller_id,
            enabled: binding.enabled,
            inputs: vec![input_id],
            dependencies: vec![],
            mapping: MappingConfig::LegacyThreeAnchor { input_range: binding.input_range, easing: binding.easing },
            bindings: vec![canonical_binding],
            parent_scoped: false,
        };
        controller.validate_local()?;
        controllers.push(controller);
    }

    // Reuse one definition for each shared legacy driver ID.
    inputs.sort_by(|a, b| a.id.cmp(&b.id));
    inputs.dedup_by(|a, b| a.id == b.id);
    let document = empty_document(inputs, controllers);
    document.validate()?;
    Ok(MigrationOutput {
        document,
        report: MigrationReport {
            source_format: "binding_set_v1".into(),
            notices: vec!["One canonical controller was created per legacy binding to preserve per-binding mapping semantics.".into()],
            losses: vec![],
        },
    })
}

/// Migrate the unversioned legacy Params object. IDs are deterministic migration
/// identities, not claims about durable host identity; the report calls this out.
pub fn migrate_unversioned_params(source: &Params) -> Result<MigrationOutput, CoreError> {
    validation::validate(source).map_err(CoreError::InvalidModel)?;
    let input_id = InputId::new("legacy-params-input")?;
    let controller_id = ControllerId::new("legacy-params-controller")?;
    let binding_id = BindingId::new("legacy-params-binding")?;
    let (input_source, fallback) = if let Some(value) = source.manual_value {
        (InputSource::Manual, FallbackPolicy::UseValue(TypedValue::scalar(value)?))
    } else {
        let bone_id = match (source.driver_layer_id, source.driver_bone_name.as_deref()) {
            (Some(layer), Some(name)) => format!("layer:{layer}/bone:{name}"),
            (_, Some(name)) => name.to_owned(),
            _ => return Err(CoreError::InvalidModel("legacy driver identity is incomplete".into())),
        };
        (InputSource::LegacyScalar { source_id: bone_id }, FallbackPolicy::Reject)
    };
    let input = InputDefinition {
        id: input_id.clone(),
        source: input_source,
        dimension: InputDimension::Scalar,
        fallback,
    };
    input.validate()?;

    let minimum = source.min_rotation_deg.ok_or_else(|| CoreError::InvalidModel("min rotation is missing".into()))?;
    let maximum = source.max_rotation_deg.ok_or_else(|| CoreError::InvalidModel("max rotation is missing".into()))?;
    let (target_id, target, mode, base_value) = match source.mode {
        Mode::Slider => {
            let layer = source.action_layer_id.ok_or_else(|| CoreError::InvalidModel("action layer is missing".into()))?;
            let start = source.action_start_frame.ok_or_else(|| CoreError::InvalidModel("start frame is missing".into()))?;
            let end = source.action_end_frame.ok_or_else(|| CoreError::InvalidModel("end frame is missing".into()))?;
            (
                TargetId::new(format!("action-layer:{layer}"))?,
                TargetMapping::FrameRange {
                    action_layer_id: layer,
                    start_frame: start,
                    end_frame: end,
                    endpoint_policy: source.endpoint_policy.unwrap_or_default(),
                },
                BindingMode::AbsolutePose,
                None,
            )
        }
        Mode::Transform => {
            let target = source.target_bone_name.as_deref().ok_or_else(|| CoreError::InvalidModel("target bone is missing".into()))?;
            let property = source.property.as_deref().ok_or_else(|| CoreError::InvalidModel("property is missing".into()))?;
            let scale = source.scale.ok_or_else(|| CoreError::InvalidModel("scale is missing".into()))?;
            (
                TargetId::new(format!("bone:{target}/property:{property}"))?,
                TargetMapping::Property { target_id: target.to_owned(), property: property.to_owned(), scale, additive: source.additive },
                if source.additive { BindingMode::RelativeDelta } else { BindingMode::AbsolutePose },
                source.additive.then(|| TypedValue::scalar(0.0)).transpose()?,
            )
        }
    };
    let binding = ControllerBinding { id: binding_id, target_id, target, mode, base_value };
    let controller = Controller {
        id: controller_id,
        enabled: true,
        inputs: vec![input_id],
        dependencies: vec![],
        mapping: MappingConfig::DirectedRange1D { start: minimum, end: maximum },
        bindings: vec![binding],
        parent_scoped: false,
    };
    controller.validate_local()?;
    let document = empty_document(vec![input], vec![controller]);
    document.validate()?;

    let mut notices = vec!["Unversioned Params has no durable binding/controller identity; deterministic migration IDs were assigned and should be replaced by authored IDs on first save.".into()];
    let mut losses = Vec::new();
    if source.manual_value.is_some() {
        if source.driver_layer_id.is_some() || source.driver_bone_name.is_some() {
            losses.push("Driver layer/bone identity is unused when manual_value is present and was not carried into the canonical input.".into());
        }
    }
    match source.mode {
        Mode::Slider if source.target_bone_name.is_some() || source.property.is_some() || source.scale.is_some() || source.additive =>
            losses.push("Transform-only Params fields are not applicable to slider mode and were not carried forward.".into()),
        Mode::Transform if source.action_layer_id.is_some() || source.action_start_frame.is_some() || source.action_end_frame.is_some() || source.endpoint_policy.is_some() =>
            losses.push("Slider-only Params fields are not applicable to transform mode and were not carried forward.".into()),
        _ => {}
    }
    if source.mode == Mode::Slider {
        notices.push("Slider endpoint policy and frame range were preserved as a canonical FrameRange target.".into());
    }
    Ok(MigrationOutput {
        document,
        report: MigrationReport { source_format: "unversioned_params".into(), notices, losses },
    })
}
