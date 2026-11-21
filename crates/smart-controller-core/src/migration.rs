//! Explicit migrations from persisted V1 controller formats to the canonical document.
use crate::{
    canonical::{BindingMode, Controller, ControllerBinding, EasingConfig, MappingConfig, TargetMapping},
    error::CoreError,
    ids::{BindingId, ControllerId, InputId, TargetId},
    input::{FallbackPolicy, InputDefinition, InputDimension, InputSource},
    legacy::{BindingSet, ControllerBinding as LegacyBinding, ControllerMode, ControllerTarget, EaseCurve, InputRange, Vec2},
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

    pub fn to_json(&self) -> Result<String, CoreError> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    pub fn from_json(source: &str) -> Result<Self, CoreError> {
        let document: Self = serde_json::from_str(source).map_err(|e| CoreError::Serialization(e.to_string()))?;
        document.validate()?;
        Ok(document)
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
            mapping: MappingConfig::ThreeAnchorRange { minimum: binding.input_range.minimum, neutral: binding.input_range.neutral, maximum: binding.input_range.maximum, easing: match binding.easing { crate::legacy::EaseCurve::Linear => EasingConfig::Linear, crate::legacy::EaseCurve::Smoothstep => EasingConfig::Smoothstep } },
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


/// Detect and migrate either a versioned BindingSet V1 or the unversioned Params
/// object. The discriminator is structural; unknown fields remain rejected by
/// the selected strict deserializer.
pub fn migrate_json(source: &str) -> Result<MigrationOutput, CoreError> {
    let value: serde_json::Value = serde_json::from_str(source)
        .map_err(|e| CoreError::Serialization(e.to_string()))?;
    if value.get("schema_version").is_some() || value.get("bindings").is_some() {
        let set = BindingSet::from_json(source).map_err(|e| CoreError::Serialization(e.to_string()))?;
        migrate_binding_set_v1(&set)
    } else {
        let params: Params = serde_json::from_value(value)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        migrate_unversioned_params(&params)
    }
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyExportOutput {
    pub binding_set: BindingSet,
    pub report: MigrationReport,
}

/// Export only when every canonical field is representable by the V1 BindingSet.
/// Unsupported canonical structures fail explicitly rather than being silently dropped.
pub fn export_binding_set_v1(document: &CanonicalDocument) -> Result<LegacyExportOutput, CoreError> {
    document.validate()?;
    let mut bindings = Vec::with_capacity(document.controllers.len());
    for controller in &document.controllers {
        if controller.parent_scoped || !controller.dependencies.is_empty() || controller.inputs.len() != 1 || controller.bindings.len() != 1 {
            return Err(CoreError::InvalidModel(format!("controller {} is not representable as one V1 binding", controller.id)));
        }
        let (minimum, neutral, maximum, easing) = match &controller.mapping {
            MappingConfig::ThreeAnchorRange { minimum, neutral, maximum, easing } => (
                *minimum, *neutral, *maximum,
                match easing { EasingConfig::Linear => EaseCurve::Linear, EasingConfig::Smoothstep => EaseCurve::Smoothstep },
            ),
            _ => return Err(CoreError::InvalidModel(format!("controller {} uses a mapping unsupported by V1", controller.id))),
        };
        let input_id = &controller.inputs[0];
        let input = document.inputs.iter().find(|item| &item.id == input_id)
            .ok_or_else(|| CoreError::MissingInput(input_id.to_string()))?;
        let source_id = match &input.source {
            InputSource::LegacyScalar { source_id } if input.dimension == InputDimension::Scalar && input.fallback == FallbackPolicy::Reject => source_id.clone(),
            _ => return Err(CoreError::InvalidModel(format!("input {} is not a lossless V1 scalar source", input.id))),
        };
        let expected_input_id = format!("legacy-controller:{source_id}");
        if input.id.as_str() != expected_input_id.as_str() {
            return Err(CoreError::InvalidModel(format!("input {} has no reversible V1 identity", input.id)));
        }
        let canonical = &controller.bindings[0];
        let (mode, target, expected_target_id) = match &canonical.target {
            TargetMapping::SymbolFrame { symbol_instance_id, negative_frame, neutral_frame, positive_frame } => {
                if canonical.mode != BindingMode::AbsolutePose || canonical.base_value.is_some() {
                    return Err(CoreError::InvalidModel(format!("symbol binding {} has unsupported V1 mode/base", canonical.id)));
                }
                (ControllerMode::AbsolutePose, ControllerTarget::SymbolFrame {
                    symbol_instance_id: symbol_instance_id.clone(),
                    negative_frame: *negative_frame,
                    neutral_frame: *neutral_frame,
                    positive_frame: *positive_frame,
                }, TargetId::new(format!("symbol:{symbol_instance_id}"))?)
            }
            TargetMapping::GeometryDeform { element_id, pin_id, negative_offset, neutral_offset, positive_offset } => {
                let expected_base = TypedValue::vector2(neutral_offset[0], neutral_offset[1])?;
                match canonical.mode {
                    BindingMode::AbsolutePose if canonical.base_value.is_none() => {}
                    BindingMode::RelativeDelta if canonical.base_value == Some(expected_base) => {}
                    _ => return Err(CoreError::InvalidModel(format!("geometry binding {} has a non-round-trippable base/mode", canonical.id))),
                }
                (match canonical.mode { BindingMode::AbsolutePose => ControllerMode::AbsolutePose, BindingMode::RelativeDelta => ControllerMode::RelativeDelta },
                 ControllerTarget::GeometryDeform {
                    element_id: element_id.clone(), pin_id: pin_id.clone(),
                    negative_offset: Vec2 { x: negative_offset[0], y: negative_offset[1] },
                    neutral_offset: Vec2 { x: neutral_offset[0], y: neutral_offset[1] },
                    positive_offset: Vec2 { x: positive_offset[0], y: positive_offset[1] },
                 }, TargetId::new(format!("geometry:{element_id}:{pin_id}"))?)
            }
            _ => return Err(CoreError::InvalidModel(format!("target of binding {} is unsupported by V1", canonical.id))),
        };
        if canonical.target_id != expected_target_id {
            return Err(CoreError::InvalidModel(format!("binding {} has a non-reversible target identity", canonical.id)));
        }
        let expected_controller_id = format!("legacy-binding:{}", canonical.id);
        if controller.id.as_str() != expected_controller_id.as_str() {
            return Err(CoreError::InvalidModel(format!("controller {} has no reversible V1 identity", controller.id)));
        }
        bindings.push(LegacyBinding {
            id: canonical.id.to_string(),
            controller_id: source_id,
            input_range: InputRange { minimum, neutral, maximum },
            easing,
            mode,
            target,
            enabled: controller.enabled,
        });
    }
    let binding_set = BindingSet::new(bindings).map_err(|e| CoreError::InvalidModel(e.to_string()))?;
    Ok(LegacyExportOutput {
        binding_set,
        report: MigrationReport {
            source_format: "canonical_v1".into(),
            notices: vec!["All canonical fields were representable in the V1 BindingSet; export was lossless.".into()],
            losses: vec![],
        },
    })
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamsExportOutput {
    pub params: Params,
    pub report: MigrationReport,
}

/// Reverse the unversioned Params migration only when the canonical document
/// still has the exact deterministic shape and identities created by that migration.
pub fn export_unversioned_params(document: &CanonicalDocument) -> Result<ParamsExportOutput, CoreError> {
    document.validate()?;
    if document.controllers.len() != 1 || document.inputs.len() != 1 {
        return Err(CoreError::InvalidModel("document is not representable as one legacy Params object".into()));
    }
    let controller = &document.controllers[0];
    if controller.id.as_str() != "legacy-params-controller" || controller.enabled == false ||
        controller.parent_scoped || !controller.dependencies.is_empty() || controller.inputs.len() != 1 || controller.bindings.len() != 1 {
        return Err(CoreError::InvalidModel("controller identity or structure is not reversible to Params".into()));
    }
    let (min_rotation_deg, max_rotation_deg) = match &controller.mapping {
        MappingConfig::DirectedRange1D { start, end } => (*start, *end),
        _ => return Err(CoreError::InvalidModel("Params requires a directed scalar range".into())),
    };
    let input = &document.inputs[0];
    if input.id.as_str() != "legacy-params-input" || controller.inputs[0] != input.id {
        return Err(CoreError::InvalidModel("Params input identity is not reversible".into()));
    }
    let (driver_layer_id, driver_bone_name, manual_value) = match (&input.source, &input.fallback) {
        (InputSource::Manual, FallbackPolicy::UseValue(TypedValue::Scalar(value))) if input.dimension == InputDimension::Scalar =>
            (None, None, Some(value.get())),
        (InputSource::LegacyScalar { source_id }, FallbackPolicy::Reject) if input.dimension == InputDimension::Scalar => {
            if let Some(rest) = source_id.strip_prefix("layer:") {
                let (layer, bone) = rest.split_once("/bone:")
                    .ok_or_else(|| CoreError::InvalidModel("legacy driver source identity is malformed".into()))?;
                let layer_id = layer.parse::<u64>().map_err(|_| CoreError::InvalidModel("legacy driver layer ID is malformed".into()))?;
                if layer_id == 0 || bone.trim().is_empty() {
                    return Err(CoreError::InvalidModel("legacy driver layer/bone identity is invalid".into()));
                }
                (Some(layer_id), Some(bone.to_owned()), None)
            } else if !source_id.trim().is_empty() {
                (None, Some(source_id.clone()), None)
            } else {
                return Err(CoreError::InvalidModel("legacy driver source identity is empty".into()));
            }
        }
        _ => return Err(CoreError::InvalidModel("input source/fallback cannot be represented by Params".into())),
    };
    let binding = &controller.bindings[0];
    if binding.id.as_str() != "legacy-params-binding" {
        return Err(CoreError::InvalidModel("Params binding identity is not reversible".into()));
    }
    let (mode, action_layer_id, action_start_frame, action_end_frame, endpoint_policy, target_bone_name, property, scale, additive) = match &binding.target {
        TargetMapping::FrameRange { action_layer_id, start_frame, end_frame, endpoint_policy } => {
            if binding.mode != BindingMode::AbsolutePose || binding.base_value.is_some() ||
                binding.target_id.as_str() != format!("action-layer:{action_layer_id}").as_str() {
                return Err(CoreError::InvalidModel("frame target is not a reversible Params slider".into()));
            }
            (Mode::Slider, Some(*action_layer_id), Some(*start_frame), Some(*end_frame), Some(*endpoint_policy), None, None, None, false)
        }
        TargetMapping::Property { target_id, property, scale, additive } => {
            let expected_mode = if *additive { BindingMode::RelativeDelta } else { BindingMode::AbsolutePose };
            let expected_base = if *additive { Some(TypedValue::scalar(0.0)?) } else { None };
            if binding.mode != expected_mode || binding.base_value != expected_base ||
                binding.target_id.as_str() != format!("bone:{target_id}/property:{property}").as_str() {
                return Err(CoreError::InvalidModel("property target is not a reversible Params transform".into()));
            }
            (Mode::Transform, None, None, None, None, Some(target_id.clone()), Some(property.clone()), Some(*scale), *additive)
        }
        _ => return Err(CoreError::InvalidModel("target type is not representable by Params".into())),
    };
    let params = Params {
        mode, driver_layer_id, driver_bone_name, action_layer_id,
        min_rotation_deg: Some(min_rotation_deg), max_rotation_deg: Some(max_rotation_deg),
        action_start_frame, action_end_frame, endpoint_policy,
        target_bone_name, property, scale, additive, manual_value,
    };
    validation::validate(&params).map_err(CoreError::InvalidModel)?;
    Ok(ParamsExportOutput {
        params,
        report: MigrationReport {
            source_format: "canonical_v1".into(),
            notices: vec!["All canonical fields were representable in the unversioned Params format; export was lossless.".into()],
            losses: vec![],
        },
    })
}
