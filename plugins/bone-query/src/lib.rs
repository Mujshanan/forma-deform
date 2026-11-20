//! Read-only AniCraft armature and bone inspection command plugin.
//!
//! Follows the bone.info contract in Mujshanan/anicraft:main. Reports keyed pose data, spring
//! settings and swing, joint translation, armature membership, and optional shape skin weights.
//! It never edits the document.

use anicraft_plugin_sdk as ac;
use serde_json::{Value, json};

const MANIFEST: &str = r##"{
  "id": "org.mujshanan.bone-query",
  "name": "Bone Query",
  "version": "0.2.0",
  "kind": "command",
  "author": "Mujshanan",
  "description": "Inspect armatures, bone poses, joint limits, springs, translation and shape weights.",
  "params": {
    "use_frame": {"type": "bool", "default": false, "label": "Query a specific frame"},
    "frame": {"type": "int", "min": 0, "max": 1000000, "default": 0, "label": "Frame (0-based)"},
    "use_layer": {"type": "bool", "default": false, "label": "Filter by armature layer"},
    "layer": {"type": "int", "min": 0, "max": 1000000000000, "default": 0, "label": "Armature layer ID"},
    "points": {"type": "bool", "default": false, "label": "Include shape point weights"},
    "max_points": {"type": "int", "min": 1, "max": 100, "default": 30, "label": "Maximum points per shape"}
  }
}"##;

ac::plugin!(MANIFEST, run);

fn run(params: &Value) -> Result<(), String> {
    let document = ac::document()?;
    let playhead_frame = document["frame"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);
    let query = build_query(params, playhead_frame);
    let frame = query["frame"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(playhead_frame);
    let include_points = query["points"].as_bool().unwrap_or(false);
    let max_points = params["max_points"].as_u64().unwrap_or(30).clamp(1, 100) as usize;

    let result = ac::command("bone.info", query)?;
    report_armatures(&result, frame, include_points, max_points);
    Ok(())
}

fn build_query(params: &Value, playhead_frame: u32) -> Value {
    // Every declared manifest parameter receives a default from AniCraft. Explicit enable
    // toggles are therefore needed to distinguish "use frame/layer 0" from "use the default".
    let frame = if params["use_frame"].as_bool().unwrap_or(false) {
        params["frame"]
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(playhead_frame)
    } else {
        playhead_frame
    };
    let include_points = params["points"].as_bool().unwrap_or(false);
    let mut query = json!({"frame": frame, "points": include_points});
    if params["use_layer"].as_bool().unwrap_or(false)
        && let Some(layer) = params["layer"].as_u64()
    {
        query["layer"] = json!(layer);
    }
    query
}

fn report_armatures(result: &Value, frame: u32, include_points: bool, max_points: usize) {
    let armatures = result["armatures"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let reported_frame = result["frame"].as_u64().unwrap_or(u64::from(frame));
    ac::log(&format!(
        "Bone Query — frame {} (0-based {}) — {} armature(s)",
        reported_frame + 1,
        reported_frame,
        armatures.len()
    ));

    if armatures.is_empty() {
        ac::log("No armatures found for this frame/filter.");
        return;
    }
    for armature in armatures {
        report_armature(armature, include_points, max_points);
    }
}

fn report_armature(armature: &Value, include_points: bool, max_points: usize) {
    let layer = display_u64(armature.get("layer"));
    let name = armature["name"].as_str().unwrap_or("(unnamed)");
    let springs = if armature["springs"].as_bool().unwrap_or(false) {
        "on"
    } else {
        "off"
    };
    let mixed = armature["mixed"].as_bool().unwrap_or(false);
    let roots = display_id_list(armature.get("roots"));
    let members = display_id_list(armature.get("members"));
    let poses = display_u64(armature.get("poses"));
    let first_shape = display_optional_u64(armature.get("shape"));

    ac::log(&format!(
        "Armature layer {layer}: {name} | poses {poses} | springs {springs} | mixed {mixed}"
    ));
    ac::log(&format!(
        "  Roots (root element IDs): [{roots}] | members (element IDs): [{members}] | first shape: {first_shape}"
    ));

    if let Some(bones) = armature["bones"].as_array() {
        for bone in bones {
            ac::log(&format!("  {}", format_bone(bone)));
        }
    }

    if let Some(shapes) = armature["shapes"].as_array() {
        for shape in shapes {
            report_shape(shape, include_points, max_points);
        }
    } else if include_points {
        report_shape_alias(armature, max_points);
    }
}

fn format_bone(bone: &Value) -> String {
    let id = display_u64(bone.get("id"));
    let name = bone["name"].as_str().unwrap_or("(unnamed)");
    let from = display_u64(bone.get("from"));
    let to = display_u64(bone.get("to"));
    let parent = display_optional_u64(bone.get("parent_bone"));
    let angle = display_number(bone.get("angle"));
    let swing = display_number(bone.get("swing"));
    let displayed = match (bone["angle"].as_f64(), bone["swing"].as_f64()) {
        (Some(angle), Some(swing)) => format!("{:.2}", angle + swing),
        _ => "n/a".to_string(),
    };
    let rotation_limits = match (bone["min"].as_f64(), bone["max"].as_f64()) {
        (Some(min), Some(max)) => format!("{min:.2}°..{max:.2}°"),
        _ => "unconstrained".to_string(),
    };
    let pinned = bone["pinned"].as_bool().unwrap_or(false);
    let head = display_point(bone.get("head"));
    let tail = display_point(bone.get("tail"));
    let slide = display_point(bone.get("slide"));
    let slide_x = display_translation_limits(bone.get("slide_x"));
    let slide_y = display_translation_limits(bone.get("slide_y"));
    let spring = display_spring(bone.get("spring"));
    let shape = display_optional_u64(bone.get("shape"));
    let bound_points = display_optional_u64(bone.get("points"));

    format!(
        "Bone {id} '{name}' ({from} -> {to}; parent bone {parent}): angle {angle}°, spring swing {swing}°, angle + swing {displayed}°; rotation limits {rotation_limits}; pinned {pinned}; head {head}, tail {tail}; slide {slide}; translation X {slide_x}, translation Y {slide_y}; spring {spring}; shape {shape}, bound points {bound_points}"
    )
}

fn report_shape(shape: &Value, include_points: bool, max_points: usize) {
    let shape_id = display_u64(shape.get("shape"));
    let count = shape["count"].as_u64().unwrap_or(0);
    let bones = display_id_list(shape.get("bones"));
    ac::log(&format!(
        "Shape {shape_id}: {count} skin point(s); bones [{bones}]"
    ));
    if include_points {
        if let Some(points) = shape["points"].as_array() {
            report_points(points, max_points);
        } else {
            ac::log("  Point details were requested but are absent from the host response.");
        }
    }
}

fn report_shape_alias(armature: &Value, max_points: usize) {
    let Some(points) = armature["points"].as_array() else {
        return;
    };
    let shape_id = display_optional_u64(armature.get("shape"));
    ac::log(&format!("Shape {shape_id}: {} skin point(s)", points.len()));
    report_points(points, max_points);
}

fn report_points(points: &[Value], max_points: usize) {
    for (index, point) in points.iter().take(max_points).enumerate() {
        let at = display_point(point.get("at"));
        let manual = point["manual"].as_bool().unwrap_or(false);
        let weights = point["bones"]
            .as_array()
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| {
                        let pair = entry.as_array()?;
                        let id = pair.first()?.as_u64()?;
                        let weight = pair.get(1)?.as_f64()?;
                        Some(format!("{id}:{weight:.3}"))
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        ac::log(&format!(
            "  Point {index} at {at}; weights [{weights}]; manual {manual}"
        ));
    }
    if points.len() > max_points {
        ac::log(&format!(
            "  … {} more point(s) omitted; increase Maximum points per shape to inspect more",
            points.len() - max_points
        ));
    }
}

fn display_u64(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_u64)
        .map(|n| n.to_string())
        .unwrap_or_else(|| "n/a".to_string())
}

fn display_optional_u64(value: Option<&Value>) -> String {
    match value {
        Some(Value::Null) | None => "none".to_string(),
        Some(value) => display_u64(Some(value)),
    }
}

fn display_id_list(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_u64)
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn display_number(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_f64)
        .map(|n| format!("{n:.2}"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn display_point(value: Option<&Value>) -> String {
    let Some(pair) = value.and_then(Value::as_array) else {
        return "(not evaluated)".to_string();
    };
    match (
        pair.first().and_then(Value::as_f64),
        pair.get(1).and_then(Value::as_f64),
    ) {
        (Some(x), Some(y)) => format!("({x:.2}, {y:.2})"),
        _ => "(not evaluated)".to_string(),
    }
}

fn display_translation_limits(value: Option<&Value>) -> String {
    let Some(value) = value.filter(|v| !v.is_null()) else {
        return "disabled".to_string();
    };
    if value["constrain"].as_bool() == Some(false) {
        return "free".to_string();
    }
    match (value["min"].as_f64(), value["max"].as_f64()) {
        (Some(min), Some(max)) => format!("{min:.2}..{max:.2} px"),
        _ => "enabled (limits unavailable)".to_string(),
    }
}

fn display_spring(value: Option<&Value>) -> String {
    let Some(value) = value.filter(|v| !v.is_null()) else {
        return "none".to_string();
    };
    match (value["strength"].as_f64(), value["damping"].as_f64()) {
        (Some(strength), Some(damping)) => {
            format!("strength {strength:.1}%, damping {damping:.1}%")
        }
        _ => "configured (details unavailable)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_fixture() -> Value {
        json!({
            "frame": 4,
            "armatures": [{
                "layer": 7, "name": "Character Rig", "roots": [101],
                "members": [101, 102, 103], "poses": 3, "springs": true,
                "mixed": true, "shape": 201,
                "bones": [{
                    "id": 12, "name": "Forearm", "from": 101, "to": 102,
                    "parent_bone": 11, "head": [1.0, 2.0], "tail": [3.0, 4.0],
                    "angle": 30.0, "min": -90.0, "max": 90.0, "pinned": false,
                    "slide": [2.0, 0.0],
                    "slide_x": {"min": -20.0, "max": 20.0, "constrain": true},
                    "slide_y": {"min": -10000.0, "max": 10000.0, "constrain": false},
                    "spring": {"strength": 60.0, "damping": 25.0}, "swing": 5.0,
                    "shape": 201, "points": 8
                }, {
                    "id": 13, "name": "Unavailable", "from": 102, "to": 103,
                    "parent_bone": 12, "head": null, "tail": null, "angle": 0.0,
                    "min": null, "max": null, "pinned": true, "slide": null,
                    "slide_x": null, "slide_y": null, "spring": null, "swing": 0.0,
                    "shape": null, "points": null
                }],
                "shapes": [{
                    "shape": 201, "bones": [12], "count": 2,
                    "points": [
                        {"at": [1.0, 2.0], "bones": [[12, 1.0]], "manual": true},
                        {"at": [3.0, 4.0], "bones": [[12, 0.5], [13, 0.5]], "manual": false}
                    ]
                }]
            }]
        })
    }

    #[test]
    fn query_defaults_to_playhead_and_all_armatures() {
        assert_eq!(
            build_query(
                &json!({
                    "use_frame": false, "frame": 0,
                    "use_layer": false, "layer": 0, "points": true
                }),
                9
            ),
            json!({"frame": 9, "points": true})
        );
    }

    #[test]
    fn query_can_explicitly_select_frame_zero_and_layer_zero() {
        assert_eq!(
            build_query(
                &json!({
                    "use_frame": true, "frame": 0,
                    "use_layer": true, "layer": 0, "points": false
                }),
                9
            ),
            json!({"frame": 0, "layer": 0, "points": false})
        );
    }

    #[test]
    fn parses_and_formats_the_fork_bone_info_contract() {
        let result = host_fixture();
        let armature = &result["armatures"][0];
        let line = format_bone(&armature["bones"][0]);
        assert_eq!(result["frame"], 4);
        assert_eq!(armature["roots"][0], 101);
        assert_eq!(armature["members"].as_array().unwrap().len(), 3);
        assert!(line.contains("angle 30.00°"));
        assert!(line.contains("spring swing 5.00°"));
        assert!(line.contains("angle + swing 35.00°"));
        assert!(line.contains("slide (2.00, 0.00)"));
        assert!(line.contains("translation X -20.00..20.00 px"));
        assert!(line.contains("translation Y free"));
        assert!(line.contains("strength 60.0%, damping 25.0%"));
        assert!(line.contains("shape 201, bound points 8"));
    }

    #[test]
    fn null_evaluated_geometry_and_optional_properties_are_safe() {
        let result = host_fixture();
        let line = format_bone(&result["armatures"][0]["bones"][1]);
        assert!(line.contains("head (not evaluated)"));
        assert!(line.contains("tail (not evaluated)"));
        assert!(line.contains("rotation limits unconstrained"));
        assert!(line.contains("translation X disabled"));
        assert!(line.contains("spring none"));
        assert!(line.contains("shape none, bound points none"));
    }

    #[test]
    fn shape_points_are_parsed_without_mutating_the_response() {
        let result = host_fixture();
        let points = result["armatures"][0]["shapes"][0]["points"]
            .as_array()
            .unwrap();
        let original = result.clone();
        assert_eq!(points.len(), 2);
        assert_eq!(display_point(points[0].get("at")), "(1.00, 2.00)");
        assert_eq!(points[1]["bones"][1][1], 0.5);
        assert_eq!(result, original);
    }

    #[test]
    fn empty_armature_result_is_supported() {
        let result = json!({"frame": 0, "armatures": []});
        assert!(result["armatures"].as_array().unwrap().is_empty());
        assert_eq!(display_id_list(result.get("missing")), "");
    }
}
