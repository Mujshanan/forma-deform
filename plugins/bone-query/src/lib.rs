//! Read-only AniCraft armature and bone inspection command plugin.
//!
//! The plugin queries the documented `bone.info` command at the current playhead frame.
//! It reports keyed/constrained angle and spring swing separately; it never edits the document.

use anicraft_plugin_sdk as ac;
use serde_json::{Value, json};

const MANIFEST: &str = r##"{
  "id": "org.mujshanan.bone-query",
  "name": "Bone Query",
  "version": "0.1.0",
  "kind": "command",
  "author": "Mujshanan",
  "description": "Inspect armatures, bones, pose angles, limits, springs and optional shape-point weights.",
  "params": {
    "points": {
      "type": "bool",
      "default": false,
      "label": "Include shape point weights"
    },
    "max_points": {
      "type": "int",
      "min": 1,
      "max": 100,
      "default": 30,
      "label": "Maximum points per shape"
    }
  }
}"##;

ac::plugin!(MANIFEST, run);

fn run(params: &Value) -> Result<(), String> {
    let document = ac::document()?;
    let frame = document["frame"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);
    let include_points = params["points"].as_bool().unwrap_or(false);
    let max_points = params["max_points"].as_u64().unwrap_or(30).clamp(1, 100) as usize;

    let result = ac::command(
        "bone.info",
        json!({
            "frame": frame,
            "points": include_points
        }),
    )?;

    report_armatures(&result, frame, include_points, max_points);
    Ok(())
}

fn report_armatures(result: &Value, frame: u32, include_points: bool, max_points: usize) {
    let armatures = result["armatures"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    ac::log(&format!(
        "Bone Query — frame {} — {} armature(s)",
        frame + 1,
        armatures.len()
    ));

    if armatures.is_empty() {
        ac::log("No armatures found at the current frame.");
        return;
    }

    for armature in armatures {
        let layer = display_u64(armature.get("layer"));
        let name = armature["name"].as_str().unwrap_or("(unnamed)");
        let springs = if armature["springs"].as_bool().unwrap_or(false) {
            "on"
        } else {
            "off"
        };
        ac::log(&format!(
            "Armature layer {layer}: {name} | springs {springs} | mixed {}",
            armature["mixed"].as_bool().unwrap_or(false)
        ));

        if let Some(bones) = armature["bones"].as_array() {
            for bone in bones {
                report_bone(bone);
            }
        }

        if include_points {
            report_shapes(armature, max_points);
        }
    }
}

fn report_bone(bone: &Value) {
    let id = display_u64(bone.get("id"));
    let name = bone["name"].as_str().unwrap_or("(unnamed)");
    let parent = bone["parent_bone"]
        .as_u64()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "root".to_string());
    let angle = display_number(bone.get("angle"));
    let swing = display_number(bone.get("swing"));
    let displayed = match (bone["angle"].as_f64(), bone["swing"].as_f64()) {
        (Some(angle), Some(swing)) => format!("{:.2}", angle + swing),
        _ => "n/a".to_string(),
    };
    let limits = match (bone["min"].as_f64(), bone["max"].as_f64()) {
        (Some(min), Some(max)) => format!("{min:.2}°..{max:.2}°"),
        _ => "unconstrained".to_string(),
    };
    let pinned = bone["pinned"].as_bool().unwrap_or(false);
    let head = display_point(bone.get("head"));
    let tail = display_point(bone.get("tail"));
    let shape = bone["shape"]
        .as_u64()
        .map(|value| format!(" | shape {value}"))
        .unwrap_or_default();
    let bound_points = bone["points"]
        .as_u64()
        .map(|value| format!(" | bound points {value}"))
        .unwrap_or_default();

    ac::log(&format!(
        "Bone {id} '{name}' parent {parent}: angle {angle}°, swing {swing}°, displayed {displayed}°; limits {limits}; pinned {pinned}; {head} -> {tail}{shape}{bound_points}"
    ));
}

fn report_shapes(armature: &Value, max_points: usize) {
    let Some(shapes) = armature["shapes"].as_array() else {
        return;
    };

    for shape in shapes {
        let shape_id = display_u64(shape.get("shape"));
        let count = shape["count"].as_u64().unwrap_or(0);
        ac::log(&format!("Shape {shape_id}: {count} deformation point(s)"));

        let Some(points) = shape["points"].as_array() else {
            continue;
        };

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
                            Some(format!("{id}:{:.3}", weight))
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
}

fn display_u64(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_u64)
        .map(|number| number.to_string())
        .unwrap_or_else(|| "n/a".to_string())
}

fn display_number(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_f64)
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn display_point(value: Option<&Value>) -> String {
    let Some(pair) = value.and_then(Value::as_array) else {
        return "(n/a)".to_string();
    };
    match (pair.first().and_then(Value::as_f64), pair.get(1).and_then(Value::as_f64)) {
        (Some(x), Some(y)) => format!("({x:.2}, {y:.2})"),
        _ => "(n/a)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_armature_and_bone_data_without_mutating_input() {
        let result = json!({
            "armatures": [{
                "layer": 7,
                "name": "Armature",
                "springs": true,
                "mixed": false,
                "bones": [{
                    "id": 12,
                    "name": "Forearm",
                    "parent_bone": 11,
                    "angle": 30.0,
                    "swing": 5.0,
                    "min": -90.0,
                    "max": 90.0,
                    "pinned": false,
                    "head": [1.0, 2.0],
                    "tail": [3.0, 4.0],
                    "shape": 20,
                    "points": 8
                }],
                "shapes": [{
                    "shape": 20,
                    "count": 2,
                    "points": [
                        {"at": [1.0, 2.0], "bones": [[12, 1.0]], "manual": true},
                        {"at": [3.0, 4.0], "bones": [[12, 0.5], [13, 0.5]], "manual": false}
                    ]
                }]
            }]
        });
        let original = result.clone();
        report_armatures(&result, 4, true, 1);
        assert_eq!(result, original);
        assert_eq!(display_number(Some(&result["armatures"][0]["bones"][0]["angle"])), "30.00");
        assert_eq!(display_point(Some(&result["armatures"][0]["bones"][0]["head"])), "(1.00, 2.00)");
    }

    #[test]
    fn empty_armature_result_is_supported() {
        let result = json!({"armatures": []});
        report_armatures(&result, 0, false, 30);
        assert_eq!(result["armatures"].as_array().unwrap().len(), 0);
    }
}
