//! AniCraft Smart Controller command adapter.
//!
//! V1 retains the original Slider preview. Canonical preview accepts a
//! versioned controller document as a command parameter, reads verified host
//! inputs on demand, and reports a deterministic read-only target plan.
//! It never applies target values or drives playback.
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
    composition::CompositionContext,
    mapping::PoseLibrary,
    migration::CanonicalDocument,
};

const MANIFEST: &str = r##"{
  "id": "org.mujshanan.smart-controller",
  "name": "Smart Controller",
  "version": "0.3.0",
  "kind": "command",
  "author": "Mujshanan",
  "description": "On-demand, read-only Smart Controller preview using AniCraft bone and warp queries.",
  "params": {
    "mode": {"type": "string", "default": "slider", "label": "Legacy V1 mode"},
    "driver_layer_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Legacy driver armature/layer ID"},
    "driver_bone_name": {"type": "string", "default": "Head", "label": "Legacy driver bone name"},
    "action_layer_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Legacy action layer ID"},
    "min_rotation_deg": {"type": "number", "default": -45, "label": "Legacy minimum driver angle (degrees)"},
    "max_rotation_deg": {"type": "number", "default": 45, "label": "Legacy maximum driver angle (degrees)"},
    "action_start_frame": {"type": "int", "min": 0, "max": 1000000000, "default": 0, "label": "Legacy action start frame"},
    "action_end_frame": {"type": "int", "min": 1, "max": 1000000000, "default": 20, "label": "Legacy action end frame"},
    "endpoint_policy": {"type": "string", "default": "exclusive", "label": "Legacy endpoint policy: exclusive or inclusive"},
    "manual_value": {"type": "number", "label": "Legacy manual driver angle"},
    "canonical_document_json": {"type": "string", "default": "", "label": "Canonical Smart Controller document JSON (enables canonical preview)"},
    "manual_values_json": {"type": "string", "default": "{}", "label": "Manual inputs as JSON object of InputId to TypedValue"},
    "bone_layers_json": {"type": "string", "default": "{}", "label": "Explicit bone ID to AniCraft layer ID JSON map"},
    "bone_angle_policy": {"type": "string", "default": "keyed_pose_only", "label": "Bone angle: keyed_pose_only or keyed_pose_plus_spring_swing"},
    "angle_semantics": {"type": "string", "default": "unwrapped", "label": "Angle semantics: wrapped or unwrapped"}
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
    let name = params.driver_bone_name.as_deref().ok_or("driver_bone_name is required for bone input")?;
    let response = ac::command("bone.info", json!({"layer": layer_id}))?;
    find_driver_bone(&response, name)
        .ok_or_else(|| format!("driver bone '{name}' was not found or has no evaluated angle"))
}

fn parse_json_object_parameter(p: &Value, key: &str) -> Result<Value, String> {
    let source = p.get(key).and_then(Value::as_str).unwrap_or("{}");
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

fn run_canonical_preview(p: &Value) -> Result<(), String> {
    let source = p.get("canonical_document_json").and_then(Value::as_str)
        .filter(|source| !source.trim().is_empty())
        .ok_or("canonical_document_json is required for canonical preview")?;
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
    ac::log("Read-only preview only: no target setter, keyframe write, timeline command, or document mutation was issued.");
    Ok(())
}

fn run(p: &Value) -> Result<(), String> {
    if p.get("canonical_document_json").and_then(Value::as_str)
        .is_some_and(|source| !source.trim().is_empty())
    {
        return run_canonical_preview(p);
    }

    // Preserve the original compact Slider workflow for existing users.
    let params: Params = serde_json::from_value(p.clone())
        .map_err(|error| format!("invalid parameters: {error}"))?;
    validation::validate(&params)?;
    if params.mode != Mode::Slider {
        return Err("Transform mode is reserved for V2; the legacy V1 plugin supports Slider preview only".into());
    }
    let driver_value = match params.manual_value {
        Some(value) => value,
        None => query_driver_bone(&params)?,
    };
    let output = preview_slider(&params, driver_value)?;
    let serialized = serde_json::to_string(&output)
        .map_err(|error| format!("could not serialize preview: {error}"))?;
    ac::log(&format!("Smart Controller Slider preview: {serialized}"));
    ac::log("Preview only: no timeline command or document mutation was issued.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        assert!(!json!({"canonical_document_json":""})["canonical_document_json"].as_str().unwrap().is_empty());
        assert!(json!({"canonical_document_json":"{}"})["canonical_document_json"].as_str().is_some());
    }
}
