//! On-demand Smart Controller adapter for AniCraft.
//!
//! The core evaluates bindings; this adapter resolves a live bone input, validates an Asset
//! Warp pin, and optionally writes one absolute pin position through the host's undoable command.

use anicraft_plugin_sdk as ac;
use serde_json::{Value, json};
use smart_controller_core::{
    ControllerBinding, ControllerMode, ControllerTarget, EaseCurve, InputRange, Vec2,
    PreviewResult,
};

const MANIFEST: &str = r##"{
  "id": "org.mujshanan.smart-controller",
  "name": "Smart Controller",
  "version": "0.1.0",
  "kind": "command",
  "author": "Mujshanan",
  "description": "Map a bone angle through a validated three-anchor binding to an Asset Warp pin.",
  "params": {
    "use_bone_angle": {"type": "bool", "default": true, "label": "Read controller bone at playhead"},
    "controller_bone_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Controller bone ID"},
    "input_angle": {"type": "number", "default": 0, "label": "Explicit input angle (degrees, when bone query is off)"},
    "include_spring_swing": {"type": "bool", "default": false, "label": "Include spring swing in bone input"},
    "minimum": {"type": "number", "default": -45, "label": "Input minimum"},
    "neutral": {"type": "number", "default": 0, "label": "Input neutral"},
    "maximum": {"type": "number", "default": 45, "label": "Input maximum"},
    "smoothstep": {"type": "bool", "default": false, "label": "Use Smoothstep easing"},
    "relative_delta": {"type": "bool", "default": true, "label": "Output relative to neutral anchor"},
    "element_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Target shape element ID"},
    "pin_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Target warp pin ID"},
    "base_x": {"type": "number", "default": 0, "label": "Neutral pin base X (stage px)"},
    "base_y": {"type": "number", "default": 0, "label": "Neutral pin base Y (stage px)"},
    "negative_x": {"type": "number", "default": -10, "label": "Negative anchor X offset (px)"},
    "negative_y": {"type": "number", "default": 0, "label": "Negative anchor Y offset (px)"},
    "neutral_x": {"type": "number", "default": 0, "label": "Neutral anchor X offset (px)"},
    "neutral_y": {"type": "number", "default": 0, "label": "Neutral anchor Y offset (px)"},
    "positive_x": {"type": "number", "default": 10, "label": "Positive anchor X offset (px)"},
    "positive_y": {"type": "number", "default": 0, "label": "Positive anchor Y offset (px)"},
    "apply": {"type": "bool", "default": false, "label": "Apply target pin position (undoable document edit)"}
  }
}"##;

ac::plugin!(MANIFEST, run);

fn positive_id(params: &Value, key: &str) -> Result<u64, String> {
    let value = params[key].as_u64().ok_or_else(|| format!("'{key}' must be a positive integer"))?;
    if value == 0 { return Err(format!("'{key}' must be greater than zero")); }
    Ok(value)
}
fn finite_number(params: &Value, key: &str) -> Result<f64, String> {
    let value = params[key].as_f64().ok_or_else(|| format!("'{key}' must be a number"))?;
    if value.is_finite() { Ok(value) } else { Err(format!("'{key}' must be finite")) }
}
fn find_bone(result: &Value, id: u64) -> Option<&Value> {
    result["armatures"].as_array()?.iter()
        .flat_map(|armature| armature["bones"].as_array().into_iter().flatten())
        .find(|bone| bone["id"].as_u64() == Some(id))
}
fn has_pin(result: &Value, id: u64) -> bool {
    result["warped"].as_bool() == Some(true)
        && result["pins"].as_array().is_some_and(|pins| pins.iter().any(|pin| pin["pin"].as_u64() == Some(id)))
}

fn run(params: &Value) -> Result<(), String> {
    let use_bone = params["use_bone_angle"].as_bool().unwrap_or(true);
    let include_spring = params["include_spring_swing"].as_bool().unwrap_or(false);
    let controller_id = if use_bone { positive_id(params, "controller_bone_id")? } else { 0 };
    let input = if use_bone {
        let response = ac::command("bone.info", json!({}))?;
        let bone = find_bone(&response, controller_id)
            .ok_or_else(|| format!("controller bone {controller_id} was not found at the playhead"))?;
        let angle = bone["angle"].as_f64()
            .ok_or_else(|| format!("controller bone {controller_id} has no evaluated angle"))?;
        angle + if include_spring { bone["swing"].as_f64().unwrap_or(0.0) } else { 0.0 }
    } else {
        finite_number(params, "input_angle")?
    };
    if !input.is_finite() { return Err("controller input is not finite".into()); }

    let minimum = finite_number(params, "minimum")?;
    let neutral = finite_number(params, "neutral")?;
    let maximum = finite_number(params, "maximum")?;
    let element_id = positive_id(params, "element_id")?;
    let pin_id = positive_id(params, "pin_id")?;
    let base_x = finite_number(params, "base_x")?;
    let base_y = finite_number(params, "base_y")?;

    let binding = ControllerBinding {
        id: "invocation-binding".into(),
        controller_id: if use_bone { controller_id.to_string() } else { "explicit-input".into() },
        input_range: InputRange { minimum, neutral, maximum },
        easing: if params["smoothstep"].as_bool().unwrap_or(false) { EaseCurve::Smoothstep } else { EaseCurve::Linear },
        mode: if params["relative_delta"].as_bool().unwrap_or(true) { ControllerMode::RelativeDelta } else { ControllerMode::AbsolutePose },
        target: ControllerTarget::GeometryDeform {
            element_id: element_id.to_string(),
            pin_id: pin_id.to_string(),
            negative_offset: Vec2 { x: finite_number(params, "negative_x")?, y: finite_number(params, "negative_y")? },
            neutral_offset: Vec2 { x: finite_number(params, "neutral_x")?, y: finite_number(params, "neutral_y")? },
            positive_offset: Vec2 { x: finite_number(params, "positive_x")?, y: finite_number(params, "positive_y")? },
        },
        enabled: true,
    };

    let evaluated = binding.evaluate(input)
        .map_err(|error| format!("invalid controller binding: {error}"))?
        .ok_or_else(|| "controller binding is disabled".to_string())?;

    let warp = ac::command("warp.info", json!({"element": element_id}))?;
    if !has_pin(&warp, pin_id) {
        return Err(format!("target pin {pin_id} was not found on warped element {element_id}"));
    }

    let (target_x, target_y) = match evaluated {
        PreviewResult::GeometryDeform { offset, .. } => (base_x + offset.x, base_y + offset.y),
        PreviewResult::SymbolFrame { .. } => return Err("symbol-frame visual application is not exposed by the current AniCraft command API".into()),
    };
    if !(target_x.is_finite() && target_y.is_finite()) {
        return Err("calculated target position is not finite".into());
    }

    ac::log(&format!("Smart Controller — input {input:.3}; bone input: {use_bone}; spring swing included: {}", use_bone && include_spring));
    ac::log(&format!("Target element {element_id}, pin {pin_id}: ({target_x:.2}, {target_y:.2}) stage px"));
    if params["apply"].as_bool().unwrap_or(false) {
        let result = ac::command("warp.pin_move", json!({
            "element": element_id, "pin": pin_id, "x": target_x, "y": target_y
        }))?;
        ac::log(&format!("Applied target at {:?}. This edit is undoable.", result["at"]));
    } else {
        ac::log("Calculation only. Enable Apply target pin position to write the result.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bone_resolution_uses_host_identity() {
        let data = json!({"armatures":[{"bones":[{"id":8,"angle":10.0},{"id":21,"angle":-12.0}]}]});
        assert_eq!(find_bone(&data, 21).unwrap()["angle"], -12.0);
        assert!(find_bone(&data, 1).is_none());
    }

    #[test]
    fn pin_validation_requires_the_requested_pin() {
        let data = json!({"warped":true,"pins":[{"pin":3},{"pin":5}]});
        assert!(has_pin(&data, 5));
        assert!(!has_pin(&data, 6));
        assert!(!has_pin(&json!({"warped":false,"pins":[{"pin":5}]}), 5));
    }

    #[test]
    fn malformed_bone_and_warp_responses_fail_closed() {
        assert!(find_bone(&json!({"armatures":[]}), 7).is_none());
        assert!(!has_pin(&json!({"pins":null}), 7));
    }
}
