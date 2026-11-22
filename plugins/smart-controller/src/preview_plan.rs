//! Read-only, target-specific preview planning.
//!
//! Plans preserve the canonical target mapping and composed typed value. They
//! are data for a host preview surface, not host commands or mutations.
use smart_controller_core::{
    canonical::TargetMapping,
    migration::CanonicalDocument,
    composition::ComposedEvaluationResult,
    ids::TargetId,
    value::TypedValue,
};

use crate::targets::{assess_target, query_geometry_info, verify_geometry_pin, TargetCapability};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostTargetVerification {
    NotAttempted,
    Verified,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetPreviewItem {
    pub target_id: TargetId,
    pub target: TargetMapping,
    pub value: TypedValue,
    pub capability: TargetCapability,
    /// True only after a read-only host identity query confirms the exact pin.
    pub host_verification: HostTargetVerification,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TargetPreviewPlan {
    /// Items are sorted by canonical TargetId.
    pub items: Vec<TargetPreviewItem>,
    /// Composed outputs that have no corresponding target in the document.
    pub unresolved_targets: Vec<TargetId>,
}

/// Build a deterministic preview description from the exact composed outputs.
/// No output is re-mapped, rounded, applied, or converted into a host command.
pub fn build_target_preview_plan(
    document: &CanonicalDocument,
    result: &ComposedEvaluationResult,
) -> Result<TargetPreviewPlan, String> {
    document.validate().map_err(|error| error.to_string())?;

    let mut target_by_id = std::collections::BTreeMap::new();
    for controller in &document.controllers {
        for binding in &controller.bindings {
            if let Some(existing) = target_by_id.get(&binding.target_id) {
                if existing != &binding.target {
                    return Err(format!(
                        "target ID {} maps to inconsistent target definitions",
                        binding.target_id
                    ));
                }
            } else {
                target_by_id.insert(binding.target_id.clone(), binding.target.clone());
            }
        }
    }

    let mut plan = TargetPreviewPlan::default();
    for (target_id, value) in &result.composition.outputs {
        let target = match target_by_id.get(target_id) {
            Some(target) => target.clone(),
            None => {
                plan.unresolved_targets.push(target_id.clone());
                continue;
            }
        };
        validate_target_value(&target, *value)?;
        plan.items.push(TargetPreviewItem {
            target_id: target_id.clone(),
            capability: assess_target(&target),
            host_verification: HostTargetVerification::NotAttempted,
            target,
            value: *value,
        });
    }
    plan.items.sort_by(|left, right| left.target_id.cmp(&right.target_id));
    plan.unresolved_targets.sort();
    Ok(plan)
}

/// Build the preview plan and verify every GeometryDeform target against
/// the host's read-only warp.info response. The exact element/pin IDs are
/// checked; array order is never treated as identity. No mutation is issued.
pub fn build_host_target_preview_plan_at_frame(
    document: &CanonicalDocument,
    result: &ComposedEvaluationResult,
    frame: Option<u32>,
) -> Result<TargetPreviewPlan, String> {
    let mut plan = build_target_preview_plan(document, result)?;
    let mut element_responses: std::collections::BTreeMap<u64, serde_json::Value> = std::collections::BTreeMap::new();

    for item in &mut plan.items {
        if let TargetMapping::GeometryDeform { element_id, pin_id, .. } = &item.target {
            let element_numeric = element_id.parse::<u64>()
                .map_err(|_| format!("geometry target element ID '{element_id}' is not numeric"))?;
            let response = if let Some(response) = element_responses.get(&element_numeric) {
                response.clone()
            } else {
                let response = query_geometry_info(element_id, frame)?;
                element_responses.insert(element_numeric, response.clone());
                response
            };
            verify_geometry_pin(&response, element_id, pin_id)?;
            item.host_verification = HostTargetVerification::Verified;
        }
    }
    Ok(plan)
}

pub fn build_host_target_preview_plan(
    document: &CanonicalDocument,
    result: &ComposedEvaluationResult,
) -> Result<TargetPreviewPlan, String> {
    build_host_target_preview_plan_at_frame(document, result, None)
}

fn validate_target_value(target: &TargetMapping, value: TypedValue) -> Result<(), String> {
    let valid = match target {
        TargetMapping::SymbolFrame { .. } | TargetMapping::FrameRange { .. } =>
            matches!(value, TypedValue::Scalar(_)),
        TargetMapping::GeometryDeform { .. } =>
            matches!(value, TypedValue::Vector2(_)),
        TargetMapping::Property { .. } =>
            matches!(value, TypedValue::Scalar(_) | TypedValue::Angle(_)),
    };
    if valid {
        Ok(())
    } else {
        Err(format!(
            "target preview value type {} is incompatible with target mapping",
            value.kind_name()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_controller_core::{
        canonical::{Controller, ControllerBinding, BindingMode},
        composition::{CompositionResult, EvaluationResult},
        ids::{BindingId, ControllerId},
        schema::SchemaIdentity,
        input::{FallbackPolicy, InputDefinition, InputDimension, InputSource},
    };

    #[test]
    fn preview_plan_preserves_geometry_mapping_and_composed_vector() {
        let target_id = TargetId::new("geometry:17:12").unwrap();
        let binding = ControllerBinding {
            id: BindingId::new("pin").unwrap(),
            target_id: target_id.clone(),
            target: TargetMapping::GeometryDeform {
                element_id: "17".into(),
                pin_id: "12".into(),
                negative_offset: [0.0, 0.0],
                neutral_offset: [1.0, 1.0],
                positive_offset: [2.0, 2.0],
            },
            mode: BindingMode::AbsolutePose,
            base_value: None,
            composition: None,
        };
        let document = CanonicalDocument {
            schema: SchemaIdentity { name: "smart-controller".into(), version: 1 },
            inputs: vec![InputDefinition {
                id: smart_controller_core::InputId::new("input").unwrap(),
                source: InputSource::Manual,
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            }],
            controllers: vec![Controller {
                id: ControllerId::new("controller").unwrap(),
                enabled: true,
                inputs: vec![smart_controller_core::InputId::new("input").unwrap()],
                dependencies: vec![],
                mapping: smart_controller_core::canonical::MappingConfig::Range1D {
                    minimum: 0.0, neutral: 1.0, maximum: 2.0,
                },
                bindings: vec![binding],
                parent_scoped: false,
            }],
        };
        let result = ComposedEvaluationResult {
            evaluation: EvaluationResult::default(),
            composition: CompositionResult {
                outputs: [(target_id.clone(), TypedValue::vector2(4.0, -2.0).unwrap())].into(),
                ..CompositionResult::default()
            },
        };
        let plan = build_target_preview_plan(&document, &result).unwrap();
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].target_id, target_id);
        assert_eq!(plan.items[0].value, TypedValue::vector2(4.0, -2.0).unwrap());
        assert!(plan.unresolved_targets.is_empty());
    }

    #[test]
    fn preview_plan_rejects_target_value_type_mismatch() {
        let target = TargetMapping::GeometryDeform {
            element_id: "17".into(), pin_id: "12".into(),
            negative_offset: [0.0, 0.0], neutral_offset: [0.0, 0.0], positive_offset: [1.0, 1.0],
        };
        assert!(validate_target_value(&target, TypedValue::scalar(1.0).unwrap()).is_err());
    }
}
