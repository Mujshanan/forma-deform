//! Host target capability assessment.
//!
//! This module deliberately plans and reports capabilities; it does not issue
//! mutating commands. A capability marked preview-only still requires the
//! host's command-dialog preview/undo transaction to be invoked by a later,
//! separately reviewed integration.

use smart_controller_core::TargetMapping;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetCapabilityStatus {
    /// The core can calculate the target value, but the inspected SDK does not
    /// establish a corresponding host application command.
    CalculationOnly,
    /// A documented host command exists, but is mutating and must be invoked
    /// only through the host's preview/undo transaction.
    CommandPreviewOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetCapability {
    pub status: TargetCapabilityStatus,
    pub target_key: String,
    pub read_command: Option<&'static str>,
    pub apply_command: Option<&'static str>,
    pub explanation: &'static str,
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
            status: TargetCapabilityStatus::CommandPreviewOnly,
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
        assert_eq!(capability.status, TargetCapabilityStatus::CommandPreviewOnly);
        assert_eq!(capability.read_command, Some("warp.info"));
        assert_eq!(capability.apply_command, Some("warp.pin_move"));
    }
}
