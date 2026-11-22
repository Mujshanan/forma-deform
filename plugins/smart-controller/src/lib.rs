//! AniCraft Smart Controller command adapter.
//!
//! The native settings dialog authors a constrained canonical geometry controller
//! or runs the retained legacy Slider preview. The adapter reads verified host
//! inputs on demand and reports a deterministic, read-only target plan.
pub mod adapter;
pub mod compatibility;
pub mod input_provider;
pub mod preview_plan;
pub mod targets;

use std::collections::BTreeMap;

use anicraft_plugin_sdk as ac;
use serde_json::{Value, json};
use smart_controller_core::{
    InputId, TypedValue, Mode, Params, preview_slider, validation,
    canonical::{BindingMode, Controller, ControllerBinding, EasingConfig, MappingConfig, TargetMapping},
    ids::{BindingId, ControllerId, TargetId},
    input::{BoneTransformChannel, FallbackPolicy, InputDefinition, InputDimension, InputSource},
    schema::SchemaIdentity,
    composition::CompositionContext,
    mapping::PoseLibrary,
    migration::CanonicalDocument,
};

const MANIFEST: &str = r##"{
  "id": "org.mujshanan.smart-controller",
  "name": "Smart Controller",
  "version": "0.4.0",
  "kind": "command",
  "author": "Mujshanan",
  "description": "Native settings-dialog authoring and on-demand, read-only Smart Controller preview.",
  "params": {
    "workflow": {"type": "choice", "options": ["legacy_slider", "geometry_controller"], "default": "geometry_controller", "label": "Workflow"},
    "mode": {"type": "choice", "options": ["slider"], "default": "slider", "label": "Legacy mode"},
    "driver_layer_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Legacy driver layer ID"},
    "action_layer_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Legacy action layer ID"},
    "min_rotation_deg": {"type": "number", "min": -3600, "max": 3600, "default": -45, "label": "Legacy minimum angle"},
    "max_rotation_deg": {"type": "number", "min": -3600, "max": 3600, "default": 45, "label": "Legacy maximum angle"},
    "action_start_frame": {"type": "int", "min": 0, "max": 1000000000, "default": 0, "label": "Legacy start frame"},
    "action_end_frame": {"type": "int", "min": 1, "max": 1000000000, "default": 20, "label": "Legacy end frame"},
    "endpoint_policy": {"type": "choice", "options": ["exclusive", "inclusive"], "default": "exclusive", "label": "Legacy endpoint policy"},
    "legacy_input": {"type": "choice", "options": ["manual", "bone_rotation"], "default": "manual", "label": "Legacy input source"},
    "manual_value": {"type": "number", "min": -1000000, "max": 1000000, "default": 0, "label": "Manual input value"},
    "input_source": {"type": "choice", "options": ["manual", "bone_rotation"], "default": "manual", "label": "Controller input source"},
    "bone_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Stable bone ID"},
    "layer_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Armature layer ID"},
    "input_minimum": {"type": "number", "min": -1000000, "max": 1000000, "default": -0.7853981634, "label": "Input minimum (rad / scalar)"},
    "input_neutral": {"type": "number", "min": -1000000, "max": 1000000, "default": 0, "label": "Input neutral (rad / scalar)"},
    "input_maximum": {"type": "number", "min": -1000000, "max": 1000000, "default": 0.7853981634, "label": "Input maximum (rad / scalar)"},
    "easing": {"type": "choice", "options": ["linear", "smoothstep"], "default": "smoothstep", "label": "Mapping easing"},
    "bone_angle_policy": {"type": "choice", "options": ["keyed_pose_only", "keyed_pose_plus_spring_swing"], "default": "keyed_pose_only", "label": "Bone angle policy"},
    "angle_semantics": {"type": "choice", "options": ["wrapped", "unwrapped"], "default": "unwrapped", "label": "Angle semantics"},
    "element_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Geometry element ID"},
    "pin_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Geometry pin ID"},
    "negative_x": {"type": "number", "min": -1000000, "max": 1000000, "default": -10, "label": "Negative X anchor"},
    "negative_y": {"type": "number", "min": -1000000, "max": 1000000, "default": 0, "label": "Negative Y anchor"},
    "neutral_x": {"type": "number", "min": -1000000, "max": 1000000, "default": 0, "label": "Neutral X anchor"},
    "neutral_y": {"type": "number", "min": -1000000, "max": 1000000, "default": 0, "label": "Neutral Y anchor"},
    "positive_x": {"type": "number", "min": -1000000, "max": 1000000, "default": 10, "label": "Positive X anchor"},
    "positive_y": {"type": "number", "min": -1000000, "max": 1000000, "default": 0, "label": "Positive Y anchor"},
    "target_action": {"type": "choice", "options": ["preview", "apply_current_frame", "apply_all_current_frame"], "default": "preview", "label": "Target action (apply one or all verified pins at the playhead)"}
  }
}"##;

ac::plugin!(MANIFEST, run);

fn find_driver_bone(result: &Value, name: &str) -> Option<f64> {
    result["armatures"].as_array()?.iter()
        .flat_map(|armature| armature["bones"].as_array().into_iter().flatten())
        .find(|bone| bone["name"].as_str() == Some(name))
        .and_then(|bone| bone["angle"].as_f64())
}

fn query_driver_bone(params: &Params) -> Result<f64, String> {
    let layer_id = params.driver_layer_id.ok_or("driver_layer_id is required for bone input")?;
    // The host's native plug-in settings schema has no free-form string field.
    // Preserve the legacy workflow with its documented default bone label.
    let name = params.driver_bone_name.as_deref().unwrap_or("Head");
    let response = ac::command("bone.info", json!({"layer": layer_id}))?;
    find_driver_bone(&response, name)
        .ok_or_else(|| format!("driver bone '{name}' was not found or has no evaluated angle"))
}

fn parse_json_object_parameter(p: &Value, key: &str) -> Result<Value, String> {
    let source = match p.get(key) {
        None | Some(Value::Null) => "{}",
        Some(Value::String(source)) => source.as_str(),
        Some(_) => return Err(format!("{key} must be a JSON-encoded string")),
    };
    let value: Value = serde_json::from_str(source)
        .map_err(|error| format!("{key} must contain valid JSON: {error}"))?;
    if !value.is_object() {
        return Err(format!("{key} must be a JSON object"));
    }
    Ok(value)
}

fn parse_manual_values(p: &Value) -> Result<BTreeMap<InputId, TypedValue>, String> {
    let object = parse_json_object_parameter(p, "manual_values_json")?;
    object.as_object().expect("object checked above").iter().map(|(id, value)| {
        let id = InputId::new(id.clone()).map_err(|error| format!("invalid manual InputId: {error}"))?;
        let value: TypedValue = serde_json::from_value(value.clone())
            .map_err(|error| format!("invalid TypedValue for input {id}: {error}"))?;
        Ok((id, value))
    }).collect()
}

fn parse_bone_layers(p: &Value) -> Result<BTreeMap<String, u64>, String> {
    let object = parse_json_object_parameter(p, "bone_layers_json")?;
    object.as_object().expect("object checked above").iter().map(|(bone_id, layer)| {
        let layer_id = layer.as_u64()
            .ok_or_else(|| format!("bone_layers_json entry '{bone_id}' must be a non-negative integer layer ID"))?;
        if bone_id.trim().is_empty() {
            return Err("bone_layers_json contains an empty bone ID".into());
        }
        Ok((bone_id.clone(), layer_id))
    }).collect()
}

fn run_canonical_preview(p: &Value, source: &str) -> Result<(), String> {
    let document = CanonicalDocument::from_json(source)
        .map_err(|error| format!("invalid canonical controller document: {error}"))?;
    let manual_values = parse_manual_values(p)?;
    let bone_layers = parse_bone_layers(p)?;

    let bone_angle_policy = match p.get("bone_angle_policy").and_then(Value::as_str).unwrap_or("keyed_pose_only") {
        "keyed_pose_only" => adapter::BoneAnglePolicy::KeyedPoseOnly,
        "keyed_pose_plus_spring_swing" => adapter::BoneAnglePolicy::KeyedPosePlusSpringSwing,
        other => return Err(format!("unsupported bone_angle_policy '{other}'")),
    };
    let wrapped_angles = match p.get("angle_semantics").and_then(Value::as_str).unwrap_or("unwrapped") {
        "wrapped" => true,
        "unwrapped" => false,
        other => return Err(format!("unsupported angle_semantics '{other}'; expected wrapped or unwrapped")),
    };

    let context = input_provider::HostInputContext {
        manual_values,
        bone_layers,
        bone_angle_policy,
        wrapped_angles,
    };
    let session = adapter::evaluate_host_document_preview_with_canonical_mapper(
        &document,
        &context,
        &PoseLibrary::new(),
        &CompositionContext::default(),
    )?;

    ac::log(&format!(
        "Canonical Smart Controller preview: {} target(s), {} unresolved target(s), {} diagnostic(s).",
        session.target_preview.items.len(),
        session.target_preview.unresolved_targets.len(),
        session.evaluation.evaluation.diagnostics.len(),
    ));
    for item in &session.target_preview.items {
        let value = serde_json::to_string(&item.value)
            .map_err(|error| format!("could not serialize preview value: {error}"))?;
        ac::log(&format!(
            "Target {} => {}; capability={:?}; host_verification={:?}; {}",
            item.target_id,
            value,
            item.capability.status,
            item.host_verification,
            item.capability.explanation,
        ));
    }
    for target in &session.target_preview.unresolved_targets {
        ac::log(&format!("Unresolved output target: {target}"));
    }
    for diagnostic in &session.evaluation.evaluation.diagnostics {
        ac::log(&format!("Controller diagnostic: {diagnostic:?}"));
    }

    match p.get("target_action").and_then(Value::as_str).unwrap_or("preview") {
        "preview" => {
            ac::log("Preview only: no target setter, keyframe write, timeline command, or document mutation was issued.");
        }
        "apply_current_frame" => {
            let item = single_apply_candidate(&session.target_preview)?;
            let result = targets::apply_geometry_target(&item.target, item.value)?;
            ac::log(&format!(
                "Applied {} at the current playhead via warp.pin_move. The host command owns the undo step; result: {}",
                item.target_id, result
            ));
        }
        "apply_all_current_frame" => {
            let items = multi_apply_candidates(&session.target_preview)?;
            // Phase 1: re-read and validate every target. No mutation occurs here.
            let prepared = items.iter()
                .map(|item| targets::prepare_geometry_target(&item.target, item.value))
                .collect::<Result<Vec<_>, _>>()?;
            // Phase 2: commit all writes. AniCraft executes the command plug-in
            // inside one plugin.run transaction, so any command error rolls back
            // the complete batch and the successful batch is one undo step.
            for params in prepared {
                targets::apply_prepared_geometry_target(params)?;
            }
            ac::log(&format!(
                "Applied {} verified geometry target(s) at the current playhead in one plugin.run transaction.",
                items.len()
            ));
        }
        other => return Err(format!("unsupported target_action '{other}'")),
    }
    Ok(())
}

fn single_apply_candidate(
    plan: &preview_plan::TargetPreviewPlan,
) -> Result<&preview_plan::TargetPreviewItem, String> {
    if !plan.unresolved_targets.is_empty() {
        return Err("cannot apply while the preview contains unresolved output targets".into());
    }
    if plan.items.len() != 1 {
        return Err(format!(
            "current-frame apply requires exactly one resolved target; found {}",
            plan.items.len()
        ));
    }
    let item = &plan.items[0];
    if item.host_verification != preview_plan::HostTargetVerification::Verified {
        return Err("target identity was not verified by the host; refusing to apply".into());
    }
    if !matches!(&item.target, TargetMapping::GeometryDeform { .. }) {
        return Err("current-frame apply supports only verified GeometryDeform targets".into());
    }
    Ok(item)
}

/// Validate a complete current-frame geometry batch before any host write.
fn multi_apply_candidates(
    plan: &preview_plan::TargetPreviewPlan,
) -> Result<&[preview_plan::TargetPreviewItem], String> {
    if !plan.unresolved_targets.is_empty() {
        return Err("cannot apply batch while preview contains unresolved output targets".into());
    }
    if plan.items.is_empty() || plan.items.len() > 512 {
        return Err(format!(
            "current-frame batch requires 1–512 resolved targets; found {}",
            plan.items.len()
        ));
    }
    let mut identities = std::collections::BTreeSet::new();
    for item in &plan.items {
        if item.host_verification != preview_plan::HostTargetVerification::Verified {
            return Err(format!("target {} was not verified by the host; refusing entire batch", item.target_id));
        }
        let (element, pin) = match &item.target {
            TargetMapping::GeometryDeform { element_id, pin_id, .. } => (element_id, pin_id),
            _ => return Err(format!("target {} is not a GeometryDeform target; refusing entire batch", item.target_id)),
        };
        if !identities.insert((element, pin)) {
            return Err(format!("duplicate geometry target element={element}, pin={pin}; refusing entire batch"));
        }
    }
    Ok(&plan.items)
}

fn number_param(p: &Value, key: &str) -> Result<f64, String> {
    p.get(key).and_then(Value::as_f64).filter(|value| value.is_finite())
        .ok_or_else(|| format!("{key} must be a finite number"))
}

fn integer_param(p: &Value, key: &str) -> Result<u64, String> {
    p.get(key).and_then(Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{key} must be a positive integer"))
}

/// Pure result of translating native dialog values into the canonical preview inputs.
#[derive(Debug)]
struct GeometryAuthoringBuild {
    document: CanonicalDocument,
    manual_values: Value,
    bone_layers: Value,
}

/// Build a canonical one-input/one-binding document without issuing host commands.
/// Keeping this separate makes dialog validation and serialization independently testable.
fn build_geometry_authoring_document(p: &Value) -> Result<GeometryAuthoringBuild, String> {
    let input_source = p.get("input_source").and_then(Value::as_str).unwrap_or("manual");
    let (source, dimension, manual_values, bone_layers) = match input_source {
        "manual" => (
            InputSource::Manual,
            InputDimension::Scalar,
            json!({"fd-m3-input": {"kind": "scalar", "value": number_param(p, "manual_value")?}}),
            json!({}),
        ),
        "bone_rotation" => {
            let bone_id = integer_param(p, "bone_id")?;
            let layer_id = integer_param(p, "layer_id")?;
            (
                InputSource::BoneTransform {
                    bone_id: bone_id.to_string(),
                    channel: BoneTransformChannel::Rotation,
                },
                InputDimension::Angle,
                json!({}),
                json!({(bone_id.to_string()): layer_id}),
            )
        }
        other => return Err(format!("unsupported input_source '{other}'")),
    };

    let input = InputDefinition {
        id: InputId::new("fd-m3-input").map_err(|error| error.to_string())?,
        source,
        dimension,
        fallback: FallbackPolicy::Reject,
    };
    input.validate().map_err(|error| error.to_string())?;

    let easing = match p.get("easing").and_then(Value::as_str).unwrap_or("smoothstep") {
        "linear" => EasingConfig::Linear,
        "smoothstep" => EasingConfig::Smoothstep,
        other => return Err(format!("unsupported easing '{other}'")),
    };
    let minimum = number_param(p, "input_minimum")?;
    let neutral = number_param(p, "input_neutral")?;
    let maximum = number_param(p, "input_maximum")?;
    if !(minimum < neutral && neutral < maximum) {
        return Err("input range must satisfy minimum < neutral < maximum".into());
    }

    let element_id = integer_param(p, "element_id")?;
    let pin_id = integer_param(p, "pin_id")?;
    let controller = Controller {
        id: ControllerId::new("fd-m3-controller").map_err(|error| error.to_string())?,
        enabled: true,
        inputs: vec![input.id.clone()],
        dependencies: vec![],
        mapping: MappingConfig::ThreeAnchorRange {
            minimum,
            neutral,
            maximum,
            easing,
        },
        bindings: vec![ControllerBinding {
            id: BindingId::new("fd-m3-geometry-output").map_err(|error| error.to_string())?,
            target_id: TargetId::new(format!("geometry:{element_id}:{pin_id}"))
                .map_err(|error| error.to_string())?,
            target: TargetMapping::GeometryDeform {
                element_id: element_id.to_string(),
                pin_id: pin_id.to_string(),
                negative_offset: [number_param(p, "negative_x")?, number_param(p, "negative_y")?],
                neutral_offset: [number_param(p, "neutral_x")?, number_param(p, "neutral_y")?],
                positive_offset: [number_param(p, "positive_x")?, number_param(p, "positive_y")?],
            },
            mode: BindingMode::AbsolutePose,
            base_value: None,
            composition: None,
        }],
        parent_scoped: false,
    };
    let document = CanonicalDocument {
        schema: SchemaIdentity { name: "smart-controller".into(), version: 1 },
        inputs: vec![input],
        controllers: vec![controller],
    };
    // Round-trip through the canonical serializer/parser so invalid documents fail
    // before any host input is queried.
    let serialized = document.to_json().map_err(|error| error.to_string())?;
    let document = CanonicalDocument::from_json(&serialized)
        .map_err(|error| format!("built canonical document failed validation: {error}"))?;

    Ok(GeometryAuthoringBuild { document, manual_values, bone_layers })
}

fn run_geometry_authoring_preview(p: &Value) -> Result<(), String> {
    let built = build_geometry_authoring_document(p)?;
    let source = built.document.to_json().map_err(|error| error.to_string())?;
    let mut preview_params = p.clone();
    preview_params["manual_values_json"] = Value::String(built.manual_values.to_string());
    preview_params["bone_layers_json"] = Value::String(built.bone_layers.to_string());
    run_canonical_preview(&preview_params, &source)
}

fn run(p: &Value) -> Result<(), String> {
    match p.get("workflow").and_then(Value::as_str).unwrap_or("geometry_controller") {
        "geometry_controller" => run_geometry_authoring_preview(p),
        "legacy_slider" => {
            // Keep the legacy settings path compatible with Params' strict schema.
            let mut legacy_params = p.clone();
            if let Some(object) = legacy_params.as_object_mut() {
                for key in [
                    "workflow", "legacy_input", "input_source", "bone_id", "layer_id",
                    "input_minimum", "input_neutral", "input_maximum", "easing",
                    "bone_angle_policy", "angle_semantics", "element_id", "pin_id",
                    "negative_x", "negative_y", "neutral_x", "neutral_y", "positive_x", "positive_y", "target_action",
                ] {
                    object.remove(key);
                }
            }
            let params: Params = serde_json::from_value(legacy_params)
                .map_err(|error| format!("invalid parameters: {error}"))?;
            validation::validate(&params)?;
            if params.mode != Mode::Slider {
                return Err("the legacy plugin supports Slider preview only".into());
            }
            let driver_value = if p.get("legacy_input").and_then(Value::as_str) == Some("bone_rotation") {
                query_driver_bone(&params)?
            } else {
                params.manual_value.unwrap_or(0.0)
            };
            let output = preview_slider(&params, driver_value)?;
            let serialized = serde_json::to_string(&output).map_err(|error| error.to_string())?;
            ac::log(&format!("Smart Controller Slider preview: {serialized}"));
            ac::log("Preview only: no timeline command or document mutation was issued.");
            Ok(())
        }
        other => Err(format!("unsupported workflow '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn manifest_uses_only_native_supported_parameter_types() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        let params = manifest["params"].as_object().unwrap();
        assert!(params.len() <= 32);
        assert_eq!(params["target_action"]["default"], "preview");
        assert_eq!(params["target_action"]["options"], json!(["preview", "apply_current_frame", "apply_all_current_frame"]));
        for spec in params.values() {
            assert!(matches!(
                spec["type"].as_str(),
                Some("number" | "int" | "bool" | "choice" | "color")
            ));
        }
    }

    fn batch_item(element: &str, pin: &str, verified: bool) -> preview_plan::TargetPreviewItem {
        let target = TargetMapping::GeometryDeform {
            element_id: element.into(),
            pin_id: pin.into(),
            negative_offset: [0.0, 0.0],
            neutral_offset: [0.0, 0.0],
            positive_offset: [1.0, 1.0],
        };
        preview_plan::TargetPreviewItem {
            target_id: TargetId::new(format!("geometry:{element}:{pin}")).unwrap(),
            target,
            value: TypedValue::vector2(1.0, 2.0).unwrap(),
            capability: targets::TargetCapability {
                status: targets::TargetCapabilityStatus::CommandApply,
                target_key: format!("geometry:{element}:{pin}"),
                read_command: Some("warp.info"),
                apply_command: Some("warp.pin_move"),
                explanation: "test fixture",
            },
            host_verification: if verified {
                preview_plan::HostTargetVerification::Verified
            } else {
                preview_plan::HostTargetVerification::NotAttempted
            },
        }
    }

    #[test]
    fn multi_apply_accepts_multiple_unique_verified_geometry_targets() {
        let plan = preview_plan::TargetPreviewPlan {
            items: vec![batch_item("17", "12", true), batch_item("17", "13", true)],
            unresolved_targets: vec![],
        };
        assert_eq!(multi_apply_candidates(&plan).unwrap().len(), 2);
    }

    #[test]
    fn multi_apply_rejects_duplicate_unverified_unresolved_and_unsupported_targets() {
        let duplicate = preview_plan::TargetPreviewPlan {
            items: vec![batch_item("17", "12", true), batch_item("17", "12", true)],
            unresolved_targets: vec![],
        };
        assert!(multi_apply_candidates(&duplicate).unwrap_err().contains("duplicate"));

        let unverified = preview_plan::TargetPreviewPlan {
            items: vec![batch_item("17", "12", false)],
            unresolved_targets: vec![],
        };
        assert!(multi_apply_candidates(&unverified).unwrap_err().contains("not verified"));

        let unresolved = preview_plan::TargetPreviewPlan {
            items: vec![batch_item("17", "12", true)],
            unresolved_targets: vec![TargetId::new("missing").unwrap()],
        };
        assert!(multi_apply_candidates(&unresolved).unwrap_err().contains("unresolved"));
    }

    #[test]
    fn multi_apply_enforces_the_host_batch_limit() {
        let items = (1..=513)
            .map(|pin| batch_item("17", &pin.to_string(), true))
            .collect();
        let plan = preview_plan::TargetPreviewPlan { items, unresolved_targets: vec![] };
        assert!(multi_apply_candidates(&plan).unwrap_err().contains("1–512"));
    }

    fn valid_geometry_params() -> Value {
        json!({
            "input_source": "manual",
            "manual_value": 0.4,
            "bone_id": 42,
            "layer_id": 7,
            "input_minimum": -1.0,
            "input_neutral": 0.0,
            "input_maximum": 1.0,
            "easing": "smoothstep",
            "element_id": 12,
            "pin_id": 3,
            "negative_x": -8.0, "negative_y": 1.0,
            "neutral_x": 0.0, "neutral_y": 0.0,
            "positive_x": 8.0, "positive_y": -1.0
        })
    }

    #[test]
    fn manual_authoring_builds_a_valid_canonical_geometry_document() {
        let built = build_geometry_authoring_document(&valid_geometry_params()).unwrap();
        assert_eq!(built.document.inputs.len(), 1);
        assert_eq!(built.document.controllers.len(), 1);
        assert_eq!(built.document.controllers[0].bindings.len(), 1);
        assert_eq!(built.manual_values["fd-m3-input"]["value"], 0.4);
        assert_eq!(built.bone_layers, json!({}));
        let target = &built.document.controllers[0].bindings[0].target;
        match target {
            TargetMapping::GeometryDeform { element_id, pin_id, negative_offset, positive_offset, .. } => {
                assert_eq!(element_id, "12");
                assert_eq!(pin_id, "3");
                assert_eq!(*negative_offset, [-8.0, 1.0]);
                assert_eq!(*positive_offset, [8.0, -1.0]);
            }
            _ => panic!("expected GeometryDeform target"),
        }
    }

    #[test]
    fn bone_rotation_authoring_keeps_stable_bone_and_host_layer_ids_separate() {
        let mut params = valid_geometry_params();
        params["input_source"] = json!("bone_rotation");
        let built = build_geometry_authoring_document(&params).unwrap();
        assert_eq!(built.manual_values, json!({}));
        assert_eq!(built.bone_layers, json!({"42": 7}));
        match &built.document.inputs[0].source {
            InputSource::BoneTransform { bone_id, channel } => {
                assert_eq!(bone_id, "42");
                assert!(matches!(channel, BoneTransformChannel::Rotation));
            }
            _ => panic!("expected BoneTransform input"),
        }
    }

    #[test]
    fn authoring_rejects_non_ordered_ranges_and_invalid_ids() {
        let mut params = valid_geometry_params();
        params["input_neutral"] = json!(2.0);
        assert!(build_geometry_authoring_document(&params).err().unwrap().contains("minimum < neutral < maximum"));
        let mut params = valid_geometry_params();
        params["element_id"] = json!(0);
        assert!(build_geometry_authoring_document(&params).is_err());
    }

    #[test]
    fn authoring_rejects_non_finite_or_missing_numeric_values() {
        let mut params = valid_geometry_params();
        params["positive_x"] = json!("not-a-number");
        assert!(build_geometry_authoring_document(&params).is_err());
        let mut params = valid_geometry_params();
        params.as_object_mut().unwrap().remove("pin_id");
        assert!(build_geometry_authoring_document(&params).is_err());
    }

    #[test]
    fn authoring_rejects_unknown_input_and_easing_modes() {
        let mut params = valid_geometry_params();
        params["input_source"] = json!("translation");
        assert!(build_geometry_authoring_document(&params).is_err());
        let mut params = valid_geometry_params();
        params["easing"] = json!("cubic");
        assert!(build_geometry_authoring_document(&params).is_err());
    }

    #[test]
    fn bone_resolution_uses_bone_info_response_shape() {
        let data = json!({"armatures":[{"bones":[{"name":"Head","angle":12.5},{"name":"Jaw","angle":-4.0}]}]});
        assert_eq!(find_driver_bone(&data, "Jaw"), Some(-4.0));
        assert_eq!(find_driver_bone(&data, "Missing"), None);
    }

    #[test]
    fn malformed_bone_info_fails_closed() {
        assert_eq!(find_driver_bone(&json!({"armatures":[]}), "Head"), None);
    }

    #[test]
    fn manual_values_parse_as_typed_values_keyed_by_stable_input_id() {
        let params = json!({
            "manual_values_json": "{\"jaw\":{\"kind\":\"scalar\",\"value\":0.75},\"head\":{\"kind\":\"angle\",\"value\":{\"radians\":1.0,\"wrapped\":false}}}"
        });
        let values = parse_manual_values(&params).unwrap();
        assert_eq!(values[&InputId::new("jaw").unwrap()].as_scalar(), Some(0.75));
        assert_eq!(values[&InputId::new("head").unwrap()].as_angle().unwrap().radians(), 1.0);
    }

    #[test]
    fn bone_layer_map_requires_explicit_numeric_host_layer_ids() {
        let params = json!({"bone_layers_json":"{\"42\":7}"});
        assert_eq!(parse_bone_layers(&params).unwrap().get("42"), Some(&7));
        let invalid = json!({"bone_layers_json":"{\"42\":\"seven\"}"});
        assert!(parse_bone_layers(&invalid).is_err());
    }

    #[test]
    fn canonical_mode_is_selected_only_for_a_non_empty_document() {
        let empty = json!({"canonical_document_json":""});
        let supplied = json!({"canonical_document_json":"{}"});
        assert!(empty["canonical_document_json"].as_str().unwrap().is_empty());
        assert!(supplied["canonical_document_json"].as_str().is_some());
    }

    #[test]
    fn malformed_json_parameter_types_fail_closed() {
        assert!(parse_manual_values(&json!({"manual_values_json":[]})).is_err());
        assert!(parse_bone_layers(&json!({"bone_layers_json":"[]"})).is_err());
    }
}
