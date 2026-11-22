//! Host target capability assessment.
//!
//! This module verifies target capabilities, builds deterministic command
//! payloads, and exposes the single-target current-playhead apply path. AniCraft
//! remains authoritative for mutation, document revision, and undo/redo.

use anicraft_plugin_sdk as ac;
use serde_json::Value;
use smart_controller_core::{canonical::TargetMapping, value::TypedValue};
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetCapabilityStatus {
    /// The core can calculate the target value, but the inspected SDK does not
    /// establish a corresponding host application command.
    CalculationOnly,
    /// A documented host mutation command exists and is invoked only inside
    /// AniCraft's atomic plugin.run transaction.
    CommandApply,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetCapability {
    pub status: TargetCapabilityStatus,
    pub target_key: String,
    pub read_command: Option<&'static str>,
    pub apply_command: Option<&'static str>,
    pub explanation: &'static str,
}


/// Query the documented read-only warp.info endpoint for one numeric element ID.
pub fn query_geometry_info(element_id: &str, frame: Option<u32>) -> Result<Value, String> {
    let element = element_id.parse::<u64>().ok().filter(|id| *id > 0)
        .ok_or_else(|| format!("element ID '{element_id}' is not a positive numeric host element ID"))?;
    let mut params = serde_json::Map::new();
    params.insert("element".into(), Value::from(element));
    if let Some(frame) = frame {
        params.insert("frame".into(), Value::from(frame));
    }
    ac::command("warp.info", Value::Object(params))
}

/// Verify that a canonical geometry target resolves to the exact host pin.
/// IDs are numeric host identities, never array positions or selection indices.
pub fn verify_geometry_pin(
    response: &Value,
    element_id: &str,
    pin_id: &str,
) -> Result<(), String> {
    let expected_element = element_id.parse::<u64>().ok().filter(|id| *id > 0)
        .ok_or_else(|| format!("element ID '{element_id}' is not a positive numeric host ID"))?;
    let expected_pin = pin_id.parse::<u64>().ok().filter(|id| *id > 0)
        .ok_or_else(|| format!("pin ID '{pin_id}' is not a positive numeric host ID"))?;
    let actual_element = response.get("element").and_then(Value::as_u64)
        .ok_or("warp.info response is missing numeric element ID")?;
    if actual_element != expected_element {
        return Err(format!("warp.info returned element {actual_element}, expected {expected_element}"));
    }
    let pins = response.get("pins").and_then(Value::as_array)
        .ok_or("warp.info response is missing pins")?;
    if !pins.iter().any(|pin| pin.get("pin").and_then(Value::as_u64) == Some(expected_pin)) {
        return Err(format!("pin ID {expected_pin} was not found on element {expected_element}"));
    }
    Ok(())
}


/// Build the exact host command payload for applying one evaluated geometry pin.
/// This is pure and can be tested without an AniCraft host.
pub fn geometry_pin_move_params_at_frame(target: &TargetMapping, value: TypedValue, frame: Option<u32>) -> Result<Value, String> {
    let (element_id, pin_id) = match target {
        TargetMapping::GeometryDeform { element_id, pin_id, .. } => (element_id, pin_id),
        _ => return Err("only GeometryDeform targets can be applied through warp.pin_move".into()),
    };
    let element = element_id.parse::<u64>()
        .ok().filter(|id| *id > 0)
        .ok_or_else(|| format!("element ID '{element_id}' is not a positive numeric host ID"))?;
    let pin = pin_id.parse::<u64>()
        .ok().filter(|id| *id > 0)
        .ok_or_else(|| format!("pin ID '{pin_id}' is not a positive numeric host ID"))?;
    let vector = value.as_vector2()
        .ok_or_else(|| format!("geometry target requires Vector2 output, received {}", value.kind_name()))?;
    let mut params = json!({ "element": element, "pin": pin, "x": vector.x(), "y": vector.y() });
    if let Some(frame) = frame { params["frame"] = Value::from(frame); }
    Ok(params)
}

/// Build a current-playhead payload (legacy convenience wrapper).
pub fn geometry_pin_move_params(target: &TargetMapping, value: TypedValue) -> Result<Value, String> {
    geometry_pin_move_params_at_frame(target, value, None)
}

/// Prepare one geometry write by freshly verifying its host identity.
/// This function is read-only: callers can prepare every write before committing any.
pub fn prepare_geometry_target_at_frame(target: &TargetMapping, value: TypedValue, frame: Option<u32>) -> Result<Value, String> {
    let (element_id, pin_id) = match target {
        TargetMapping::GeometryDeform { element_id, pin_id, .. } => (element_id, pin_id),
        _ => return Err("only GeometryDeform targets can be prepared for apply".into()),
    };
    let response = query_geometry_info(element_id, frame)?;
    verify_geometry_pin(&response, element_id, pin_id)?;
    geometry_pin_move_params_at_frame(target, value, frame)
}

/// Prepare a current-playhead write (legacy convenience wrapper).
pub fn prepare_geometry_target(target: &TargetMapping, value: TypedValue) -> Result<Value, String> {
    prepare_geometry_target_at_frame(target, value, None)
}

/// Execute one already-prepared pin write. The caller must run this inside the
/// enclosing AniCraft plugin.run transaction and must prepare the full batch first.
pub fn apply_prepared_geometry_target(params: Value) -> Result<Value, String> {
    ac::command("warp.pin_move", params)
}

/// Apply one verified geometry output at AniCraft's current playhead.
pub fn apply_geometry_target(target: &TargetMapping, value: TypedValue) -> Result<Value, String> {
    let params = prepare_geometry_target(target, value)?;
    apply_prepared_geometry_target(params)
}

/// Resolve target identity and its verified capability boundary without
/// issuing host commands or assuming that calculation implies application.
pub fn assess_target(target: &TargetMapping) -> TargetCapability {
    match target {
        TargetMapping::SymbolFrame { symbol_instance_id, .. } => TargetCapability {
            status: TargetCapabilityStatus::CalculationOnly,
            target_key: format!("symbol:{symbol_instance_id}"),
            read_command: None,
            apply_command: None,
            explanation: "Symbol-frame calculation is supported; no displayed-frame setter is verified.",
        },
        TargetMapping::FrameRange { action_layer_id, .. } => TargetCapability {
            status: TargetCapabilityStatus::CalculationOnly,
            target_key: format!("action-layer:{action_layer_id}"),
            read_command: None,
            apply_command: None,
            explanation: "Frame-range calculation is supported; a host command that applies this calculated frame is not established.",
        },
        TargetMapping::GeometryDeform { element_id, pin_id, .. } => TargetCapability {
            status: TargetCapabilityStatus::CommandApply,
            target_key: format!("geometry:{element_id}:{pin_id}"),
            read_command: Some("warp.info"),
            apply_command: Some("warp.pin_move"),
            explanation: "Pin inspection and mutation commands are documented; mutation is permitted only through the host preview/undo transaction.",
        },
        TargetMapping::Property { target_id, property, .. } => TargetCapability {
            status: TargetCapabilityStatus::CalculationOnly,
            target_key: format!("property:{target_id}:{property}"),
            read_command: None,
            apply_command: None,
            explanation: "Property calculation is supported by core; a matching generic host property setter is not verified for this adapter.",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn geometry_pin_identity_is_verified_by_host_ids_not_array_position() {
        let response = serde_json::json!({"element":17,"pins":[{"pin":90,"at":[1.0,2.0],"rest":[0.0,0.0]},{"pin":12,"at":[3.0,4.0],"rest":[2.0,2.0]}]});
        assert!(verify_geometry_pin(&response, "17", "12").is_ok());
        assert!(verify_geometry_pin(&response, "17", "1").is_err());
        assert!(verify_geometry_pin(&response, "18", "12").is_err());
    }

    #[test]
    fn malformed_geometry_identity_fails_closed() {
        assert!(verify_geometry_pin(&serde_json::json!({"pins":[]}), "17", "12").is_err());
        assert!(verify_geometry_pin(&serde_json::json!({"element":17,"pins":[]}), "17", "not-a-pin").is_err());
        assert!(verify_geometry_pin(&serde_json::json!({"element":0,"pins":[{"pin":12}]}), "0", "12").is_err());
        assert!(verify_geometry_pin(&serde_json::json!({"element":17,"pins":[{"id":12}]}), "17", "12").is_err(),
            "AniCraft returns pin, not id");
    }

    #[test]
    fn symbol_frame_is_calculation_only() {
        let capability = assess_target(&TargetMapping::SymbolFrame {
            symbol_instance_id: "symbol-1".into(),
            negative_frame: 0,
            neutral_frame: 5,
            positive_frame: 10,
        });
        assert_eq!(capability.status, TargetCapabilityStatus::CalculationOnly);
        assert!(capability.apply_command.is_none());
    }

    #[test]
    fn geometry_pin_exposes_the_documented_apply_command_path() {
        let capability = assess_target(&TargetMapping::GeometryDeform {
            element_id: "element-1".into(),
            pin_id: "pin-2".into(),
            negative_offset: [0.0, 0.0],
            neutral_offset: [1.0, 1.0],
            positive_offset: [2.0, 2.0],
        });
        assert_eq!(capability.status, TargetCapabilityStatus::CommandApply);
        assert_eq!(capability.read_command, Some("warp.info"));
        assert_eq!(capability.apply_command, Some("warp.pin_move"));
    }
}
