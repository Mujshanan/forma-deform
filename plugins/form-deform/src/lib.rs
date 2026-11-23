//! Bone-driven, deterministic AniCraft Asset Warp pin deformation command.

use anicraft_plugin_sdk as ac;
use form_deform_core::{DeformSpec, Vec2, evaluate_pin_position};
use serde_json::{Value, json};

const MANIFEST: &str = r##"{
  "id": "org.mujshanan.form-deform",
  "name": "Form Deform",
  "version": "0.1.0",
  "kind": "command",
  "author": "Mujshanan",
  "description": "Drive one Asset Warp pin from a bone angle using an authored neutral position and per-degree gains.",
  "params": {
    "bone_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Controller bone ID"},
    "element_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Warped shape element ID"},
    "pin_id": {"type": "int", "min": 1, "max": 1000000000000, "default": 1, "label": "Warp pin ID"},
    "neutral_angle": {"type": "number", "default": 0, "label": "Neutral bone angle (degrees)"},
    "neutral_x": {"type": "number", "default": 0, "label": "Neutral pin X (stage px)"},
    "neutral_y": {"type": "number", "default": 0, "label": "Neutral pin Y (stage px)"},
    "gain_x": {"type": "number", "default": 0, "label": "X displacement per degree (px/degree)"},
    "gain_y": {"type": "number", "default": 0, "label": "Y displacement per degree (px/degree)"},
    "include_spring_swing": {"type": "bool", "default": false, "label": "Include spring swing in bone angle"},
    "apply": {"type": "bool", "default": false, "label": "Apply pin position (creates an undoable document edit)"}
  }
}"##;

ac::plugin!(MANIFEST, run);

fn required_id(params: &Value, key: &str) -> Result<u64, String> {
    let id = params[key]
        .as_u64()
        .ok_or_else(|| format!("'{key}' must be a positive integer"))?;
    if id == 0 {
        return Err(format!("'{key}' must be greater than zero"));
    }
    Ok(id)
}
fn number(params: &Value, key: &str) -> Result<f64, String> {
    let value = params[key]
        .as_f64()
        .ok_or_else(|| format!("'{key}' must be a number"))?;
    if !value.is_finite() {
        return Err(format!("'{key}' must be finite"));
    }
    Ok(value)
}
fn run(params: &Value) -> Result<(), String> {
    let bone_id = required_id(params, "bone_id")?;
    let element_id = required_id(params, "element_id")?;
    let pin_id = required_id(params, "pin_id")?;
    let neutral_angle = number(params, "neutral_angle")?;
    let neutral_x = number(params, "neutral_x")?;
    let neutral_y = number(params, "neutral_y")?;
    let gain_x = number(params, "gain_x")?;
    let gain_y = number(params, "gain_y")?;
    let include_spring = params["include_spring_swing"].as_bool().unwrap_or(false);

    let bone_data = ac::command("bone.info", json!({}))?;
    let bone = find_bone(&bone_data, bone_id)
        .ok_or_else(|| format!("bone {bone_id} is not present in an armature at the playhead"))?;
    let keyed_angle = bone["angle"]
        .as_f64()
        .ok_or_else(|| format!("bone {bone_id} has no evaluated angle at the playhead"))?;
    let swing = bone["swing"].as_f64().unwrap_or(0.0);
    let input_angle = keyed_angle + if include_spring { swing } else { 0.0 };

    let warp = ac::command("warp.info", json!({"element": element_id}))?;
    if warp["warped"].as_bool() != Some(true) {
        return Err(format!(
            "element {element_id} has no usable Asset Warp mesh; add a warp and pin first"
        ));
    }
    if !find_pin(&warp, pin_id) {
        return Err(format!(
            "pin {pin_id} does not belong to warped element {element_id}"
        ));
    }

    let target = evaluate_pin_position(
        DeformSpec {
            neutral_position: Vec2 {
                x: neutral_x,
                y: neutral_y,
            },
            neutral_angle_degrees: neutral_angle,
            gain_per_degree: Vec2 {
                x: gain_x,
                y: gain_y,
            },
        },
        input_angle,
    )
    .map_err(|error| error.to_string())?;

    ac::log(&format!(
        "Form Deform — bone {bone_id}: keyed angle {keyed_angle:.3}°, spring swing {swing:.3}°, input {input_angle:.3}°"
    ));
    ac::log(&format!(
        "Pin {pin_id} on element {element_id}: neutral ({neutral_x:.2}, {neutral_y:.2}) → target ({:.2}, {:.2}) stage px",
        target.x, target.y
    ));

    if params["apply"].as_bool().unwrap_or(false) {
        let result = ac::command(
            "warp.pin_move",
            json!({"element": element_id, "pin": pin_id, "x": target.x, "y": target.y}),
        )?;
        ac::log(&format!(
            "Applied deformation at {:?}. This edit is undoable.",
            result["at"]
        ));
    } else {
        ac::log("Calculation only. Enable Apply pin position to write the result to the document.");
    }
    Ok(())
}
fn find_bone(result: &Value, id: u64) -> Option<&Value> {
    result["armatures"]
        .as_array()?
        .iter()
        .flat_map(|armature| armature["bones"].as_array().into_iter().flatten())
        .find(|bone| bone["id"].as_u64() == Some(id))
}
fn find_pin(result: &Value, id: u64) -> bool {
    result["pins"]
        .as_array()
        .is_some_and(|pins| pins.iter().any(|pin| pin["pin"].as_u64() == Some(id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bone_lookup_uses_stable_host_id_not_array_index() {
        let data = json!({"armatures":[{"bones":[{"id":42,"angle":12.5},{"id":91,"angle":-3.0}]}]});
        assert_eq!(find_bone(&data, 91).unwrap()["angle"], -3.0);
        assert!(find_bone(&data, 0).is_none());
    }
    #[test]
    fn pin_lookup_checks_warp_response() {
        let data = json!({"warped":true,"pins":[{"pin":7},{"pin":19}]});
        assert!(find_pin(&data, 19));
        assert!(!find_pin(&data, 2));
    }
    #[test]
    fn malformed_host_responses_are_safe() {
        assert!(find_bone(&json!({"armatures":null}), 1).is_none());
        assert!(!find_pin(&json!({"pins":null}), 1));
    }
}
