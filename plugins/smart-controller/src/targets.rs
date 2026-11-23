//! Host target capability assessment.
//!
//! This module deliberately plans and reports capabilities; it does not issue
//! mutating commands. A capability marked preview-only still requires the
//! host's command-dialog preview/undo transaction to be invoked by a later,
//! separately reviewed integration.

use anicraft_plugin_sdk as ac;
use serde_json::Value;
use smart_controller_core::canonical::TargetMapping;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetCapabilityStatus {
    /// The core can calculate the target value, but the inspected SDK does not
    /// establish a corresponding host application command.
    CalculationOnly,
    /// A documented read/write command pair may exist, but the exact target
    /// has not yet been resolved against the active host document.
    RequiresHostResolution,
    /// Exact host identity was verified; mutation remains preview/undo-only.
    CommandPreviewOnly,
}

/// Result of resolving a canonical target against the active host document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetResolution {
    /// Exact host identity was verified; application is still preview/undo-only.
    Supported(TargetCapability),
    /// Host confirms the selected object is not a supported target.
    Unsupported(String),
    /// The query failed, was malformed, or did not identify the requested object.
    Unavailable(String),
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
    let element = element_id.parse::<u64>()
        .map_err(|_| format!("element ID '{element_id}' is not a numeric host element ID"))?;
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
    let expected_element = element_id.parse::<u64>()
        .map_err(|_| format!("element ID '{element_id}' is not numeric"))?;
    let expected_pin = pin_id.parse::<u64>()
        .map_err(|_| format!("pin ID '{pin_id}' is not numeric"))?;
    let actual_element = response.get("element").and_then(Value::as_u64)
        .ok_or("warp.info response is missing numeric element ID")?;
    if actual_element != expected_element {
        return Err(format!("warp.info returned element {actual_element}, expected {expected_element}"));
    }
    if response.get("layer").and_then(Value::as_u64).is_none() {
        return Err("warp.info response is missing numeric layer ID".into());
    }
    if response.get("bitmap").and_then(Value::as_bool) == Some(true) {
        return Err("bitmap artwork is not a supported geometry deformation target".into());
    }
    if response.get("bitmap").and_then(Value::as_bool) != Some(false) {
        return Err("warp.info response is missing the bitmap capability flag".into());
    }
    if response.get("warped").and_then(Value::as_bool) != Some(true) {
        return Err("artwork has no live warp mesh and is not a supported geometry deformation target".into());
    }
    let pins = response.get("pins").and_then(Value::as_array)
        .ok_or("warp.info response is missing pins")?;
    // AniCraft's documented warp.info schema names this field "pin", not "id".
    if !pins.iter().any(|pin| pin.get("pin").and_then(Value::as_u64) == Some(expected_pin)) {
        return Err(format!("pin ID {expected_pin} was not found on element {expected_element}"));
    }
    Ok(())
}

/// Resolve a GeometryDeform target against an actual warp.info response.
/// A successful result is still only CommandPreviewOnly: this function does
/// not invoke warp.pin_move or mutate the document.
pub fn resolve_geometry_pin(
    response: &Value,
    element_id: &str,
    pin_id: &str,
) -> TargetResolution {
    let expected_element = match element_id.parse::<u64>() {
        Ok(value) => value,
        Err(_) => return TargetResolution::Unavailable("element ID is not a numeric host ID".into()),
    };
    if response.get("element").and_then(Value::as_u64) != Some(expected_element) {
        return TargetResolution::Unavailable("warp.info response does not match the requested element ID".into());
    }
    if response.get("layer").and_then(Value::as_u64).is_none() {
        return TargetResolution::Unavailable("warp.info response is missing numeric layer ID".into());
    }
    if response.get("bitmap").and_then(Value::as_bool) == Some(true) {
        return TargetResolution::Unsupported("bitmap artwork cannot be targeted by geometry pin deformation".into());
    }
    if response.get("bitmap").and_then(Value::as_bool) != Some(false) {
        return TargetResolution::Unavailable("warp.info response is missing the bitmap capability flag".into());
    }
    match response.get("warped").and_then(Value::as_bool) {
        Some(true) => {}
        Some(false) => return TargetResolution::Unsupported("artwork has no live warp mesh".into()),
        None => return TargetResolution::Unavailable("warp.info response is missing the warped capability flag".into()),
    }
    match verify_geometry_pin(response, element_id, pin_id) {
        Ok(()) => TargetResolution::Supported(TargetCapability {
            status: TargetCapabilityStatus::CommandPreviewOnly,
            target_key: format!("geometry:{element_id}:{pin_id}"),
            read_command: Some("warp.info"),
            apply_command: Some("warp.pin_move"),
            explanation: "Exact element and pin IDs were verified; application remains restricted to the host preview/undo transaction.",
        }),
        Err(error) => {
            if error.contains("pin ID") && error.contains("not found") {
                TargetResolution::Unsupported(error)
            } else {
                TargetResolution::Unavailable(error)
            }
        }
    }
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
            status: TargetCapabilityStatus::RequiresHostResolution,
            target_key: format!("geometry:{element_id}:{pin_id}"),
            read_command: Some("warp.info"),
            apply_command: Some("warp.pin_move"),
            explanation: "The mapping names a possible geometry pin; query warp.info and verify the exact live element/pin before treating it as supported.",
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
        let response = serde_json::json!({"element":17,"layer":4,"warped":true,"bitmap":false,"pins":[{"pin":90,"at":[1.0,2.0]},{"pin":12,"at":[3.0,4.0]}]});
        assert!(verify_geometry_pin(&response, "17", "12").is_ok());
        assert!(verify_geometry_pin(&response, "17", "1").is_err());
        assert!(verify_geometry_pin(&response, "18", "12").is_err());
    }

    #[test]
    fn malformed_geometry_identity_fails_closed() {
        assert!(verify_geometry_pin(&serde_json::json!({"pins":[]}), "17", "12").is_err());
        assert!(verify_geometry_pin(&serde_json::json!({"element":17,"layer":4,"warped":true,"bitmap":false,"pins":[]}), "17", "not-a-pin").is_err());
        assert!(verify_geometry_pin(&serde_json::json!({"element":17,"warped":true,"pins":[{"pin":12}]}), "17", "12").is_err());
        assert!(verify_geometry_pin(&serde_json::json!({"element":17,"warped":false,"bitmap":false}), "17", "12").is_err());
    }

    #[test]
    fn geometry_resolution_distinguishes_supported_unsupported_and_unavailable() {
        let supported = serde_json::json!({"element":17,"layer":4,"warped":true,"bitmap":false,"pins":[{"pin":12,"at":[1.0,2.0]}]});
        assert!(matches!(resolve_geometry_pin(&supported, "17", "12"), TargetResolution::Supported(_)));

        let unwarped = serde_json::json!({"element":17,"layer":4,"warped":false,"bitmap":false,"pins":[]});
        assert!(matches!(resolve_geometry_pin(&unwarped, "17", "12"), TargetResolution::Unsupported(_)));

        let bitmap = serde_json::json!({"element":17,"layer":4,"warped":true,"bitmap":true,"pins":[{"pin":12}]});
        assert!(matches!(resolve_geometry_pin(&bitmap, "17", "12"), TargetResolution::Unsupported(_)));

        assert!(matches!(resolve_geometry_pin(&supported, "18", "12"), TargetResolution::Unavailable(_)));
        assert!(matches!(resolve_geometry_pin(&supported, "17", "99"), TargetResolution::Unsupported(_)));
        assert!(matches!(resolve_geometry_pin(&serde_json::json!({"element":17,"layer":4,"bitmap":false}), "17", "12"), TargetResolution::Unavailable(_)));
        assert!(matches!(resolve_geometry_pin(&serde_json::json!({"element":17,"bitmap":false,"warped":true,"pins":[{"pin":12}]}), "17", "12"), TargetResolution::Unavailable(_)));
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
    fn geometry_pin_exposes_only_the_documented_preview_command_path() {
        let capability = assess_target(&TargetMapping::GeometryDeform {
            element_id: "element-1".into(),
            pin_id: "pin-2".into(),
            negative_offset: [0.0, 0.0],
            neutral_offset: [1.0, 1.0],
            positive_offset: [2.0, 2.0],
        });
        assert_eq!(capability.status, TargetCapabilityStatus::RequiresHostResolution);
        assert_eq!(capability.read_command, Some("warp.info"));
        assert_eq!(capability.apply_command, Some("warp.pin_move"));
    }
}
