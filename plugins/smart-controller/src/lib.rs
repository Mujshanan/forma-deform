//! Thin AniCraft adapter. V1 calculates and logs a deterministic Slider preview;
//! it does not drive the timeline during playback or mutate the document.
pub mod adapter;
pub mod compatibility;
pub mod input_provider;
pub mod preview_plan;
pub mod targets;

use anicraft_plugin_sdk as ac;
use serde_json::{Value, json};
use smart_controller_core::{Mode, Params, preview_slider, validation};

const MANIFEST: &str = r##"{
  "id": "org.mujshanan.smart-controller",
  "name": "Smart Controller",
  "version": "0.2.0",
  "kind": "command",
  "author": "Mujshanan",
  "description": "Calculate a deterministic bone-angle-to-action-frame Slider preview.",
  "params": {
    "mode": {"type": "choice", "options": ["slider"], "default": "slider", "label": "Controller mode (V1: slider)"},
    "driver_layer_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Driver armature/layer ID"},
    "driver_bone_name": {"type": "choice", "options": ["Head"], "default": "Head", "label": "Driver bone name"},
    "action_layer_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Action layer ID"},
    "min_rotation_deg": {"type": "number", "min": -360, "max": 360, "default": -45, "label": "Minimum driver angle (degrees)"},
    "max_rotation_deg": {"type": "number", "min": -360, "max": 360, "default": 45, "label": "Maximum driver angle (degrees)"},
    "action_start_frame": {"type": "int", "min": 0, "max": 1000000000, "default": 0, "label": "Action start frame"},
    "action_end_frame": {"type": "int", "min": 1, "max": 1000000000, "default": 20, "label": "Action end frame"},
    "endpoint_policy": {"type": "choice", "options": ["exclusive", "inclusive"], "default": "exclusive", "label": "Endpoint policy"},
    "manual_value": {"type": "number", "min": -360, "max": 360, "default": 0, "label": "Manual driver angle (degrees)"}
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
fn run(p: &Value) -> Result<(), String> {
    let params: Params = serde_json::from_value(p.clone())
        .map_err(|error| format!("invalid parameters: {error}"))?;
    validation::validate(&params)?;
    if params.mode != Mode::Slider {
        return Err("Transform mode is reserved for V2; the V1 plugin supports Slider preview only".into());
    }
    let driver_value = match params.manual_value {
        Some(value) => value,
        None => query_driver_bone(&params)?,
    };
    let output = preview_slider(&params, driver_value)?;
    let serialized = serde_json::to_string(&output)
        .map_err(|error| format!("could not serialize preview: {error}"))?;
    ac::log(&format!("Smart Controller preview: {serialized}"));
    ac::log("V1 is preview-only: no timeline command or document mutation was issued.");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn manifest_uses_only_anicraft_supported_parameter_types() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        let params = manifest["params"].as_object().unwrap();
        for (name, spec) in params {
            let kind = spec["type"].as_str().unwrap();
            assert!(
                matches!(kind, "number" | "int" | "bool" | "choice" | "color"),
                "unsupported AniCraft parameter type {kind:?} for {name}"
            );
            if kind == "choice" {
                assert!(!spec["options"].as_array().unwrap().is_empty());
            }
        }
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
}
