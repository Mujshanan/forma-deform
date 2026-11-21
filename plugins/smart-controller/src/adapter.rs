//! On-demand host input adaptation for the Smart Controller preview path.
//!
//! This module parses only the documented read-only `bone.info` response.
//! It does not infer freshness from wall-clock time or query the host itself;
//! the caller supplies the response obtained through the SDK.

use anicraft_plugin_sdk as ac;

use smart_controller_core::{
    composition::{evaluate_and_compose, CompositionContext, ComposedEvaluationResult},
    evaluator::ControllerMapper,
    graph::ControllerGraph,
    ids::TargetId,
    migration::CanonicalDocument,
    CoreError,
};

/// Pure preview plan produced by the host adapter. It contains calculated
/// target values and capability assessments, but performs no document writes.
#[derive(Clone, Debug, PartialEq)]
pub struct HostPreviewPlan {
    pub evaluation: ComposedEvaluationResult,
    pub targets: Vec<(TargetId, crate::targets::TargetCapability)>,
}

/// Complete read-only result for one host-driven canonical preview invocation.
#[derive(Clone, Debug, PartialEq)]
pub struct HostPreviewSession {
    pub evaluation: ComposedEvaluationResult,
    pub target_preview: crate::preview_plan::TargetPreviewPlan,
}

/// Validate and execute a canonical document against an already-collected
/// host input snapshot. Mapping remains injected: this adapter does not claim
/// that the not-yet-complete Pose2D/joystick mapper is implemented.
pub fn evaluate_document_preview<M: ControllerMapper>(
    document: &CanonicalDocument,
    snapshot: &InputSnapshot,
    mapper: &M,
    composition_context: &CompositionContext,
) -> Result<HostPreviewPlan, CoreError> {
    document.validate()?;
    let graph = ControllerGraph::new(document.controllers.clone())?;
    let evaluation = evaluate_and_compose(
        &graph,
        &document.inputs,
        snapshot,
        mapper,
        composition_context,
    )?;
    let mut targets = Vec::new();
    for controller in &document.controllers {
        for binding in &controller.bindings {
            targets.push((
                binding.target_id.clone(),
                crate::targets::assess_target(&binding.target),
            ));
        }
    }
    targets.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(HostPreviewPlan { evaluation, targets })
}


use serde_json::Value;
use smart_controller_core::{
    BoneTransformChannel, InputDefinition, InputSample, InputSnapshot, InputSource,
    TypedValue,
};

/// Collect host inputs, evaluate the canonical DAG, compose its outputs, and
/// build target-specific preview data in one invocation. The mapper remains
/// injected because the concrete Pose2D/joystick mapping implementation is a
/// separate core milestone. This function performs no target mutation.
pub fn evaluate_host_document_preview<M: ControllerMapper>(
    document: &CanonicalDocument,
    host_context: &crate::input_provider::HostInputContext,
    mapper: &M,
    composition_context: &CompositionContext,
) -> Result<HostPreviewSession, String> {
    let snapshot = crate::input_provider::collect_host_input_snapshot(
        &document.inputs,
        host_context,
    )?;
    let plan = evaluate_document_preview(document, &snapshot, mapper, composition_context)
        .map_err(|error| error.to_string())?;
    let target_preview = crate::preview_plan::build_target_preview_plan(
        document,
        &plan.evaluation,
    )?;
    Ok(HostPreviewSession {
        evaluation: plan.evaluation,
        target_preview,
    })
}

/// Whether the preview uses only the constrained keyed pose or explicitly
/// adds the separately reported spring swing. Missing swing is an error when
/// the latter policy is selected; it is never silently treated as zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoneAnglePolicy {
    KeyedPoseOnly,
    KeyedPosePlusSpringSwing,
}

/// Execute the verified, read-only bone query for a layer.
pub fn query_bone_info(layer_id: u64, frame: Option<u32>) -> Result<Value, String> {
    let mut params = serde_json::Map::new();
    params.insert("layer".into(), Value::from(layer_id));
    if let Some(frame) = frame {
        params.insert("frame".into(), Value::from(frame));
    }
    ac::command("bone.info", Value::Object(params))
}

/// Parse a canonical bone-rotation input from a previously fetched bone.info
/// response. Bone IDs are stable numeric host IDs represented canonically as
/// decimal strings. Host angles and spring swing are reported in degrees.
/// Wrapped/unwrapped interpolation semantics are supplied explicitly by the caller.
pub fn sample_bone_rotation(
    definition: &InputDefinition,
    response: &Value,
    policy: BoneAnglePolicy,
    wrapped: bool,
) -> Result<InputSample, String> {
    let (bone_id, channel) = match &definition.source {
        InputSource::BoneTransform { bone_id, channel } => (bone_id, channel),
        _ => return Err(format!("input {} is not a bone-transform input", definition.id)),
    };
    if *channel != BoneTransformChannel::Rotation {
        return Err(format!("input {} is not a rotation input", definition.id));
    }
    if definition.dimension != smart_controller_core::InputDimension::Angle {
        return Err(format!("input {} must have Angle dimension", definition.id));
    }
    let expected_id = bone_id.parse::<u64>()
        .map_err(|_| format!("bone ID '{bone_id}' is not a numeric host bone ID"))?;

    let bones = response.get("armatures")
        .and_then(Value::as_array)
        .ok_or("bone.info response is missing armatures")?;
    let bone = bones.iter()
        .filter_map(|armature| armature.get("bones").and_then(Value::as_array))
        .flatten()
        .find(|bone| host_bone_id(bone.get("id")) == Some(expected_id))
        .ok_or_else(|| format!("bone ID {expected_id} was not found in bone.info response"))?;

    let keyed_degrees = bone.get("angle").and_then(Value::as_f64)
        .ok_or_else(|| format!("bone ID {expected_id} has no numeric keyed angle"))?;
    let degrees = match policy {
        BoneAnglePolicy::KeyedPoseOnly => keyed_degrees,
        BoneAnglePolicy::KeyedPosePlusSpringSwing => {
            let swing = bone.get("swing").and_then(Value::as_f64)
                .ok_or_else(|| format!("bone ID {expected_id} has no numeric spring swing"))?;
            keyed_degrees + swing
        }
    };
    if !degrees.is_finite() {
        return Err(format!("bone ID {expected_id} produced a non-finite angle"));
    }

    let radians = degrees.to_radians();
    let value = TypedValue::angle(radians, wrapped).map_err(|error| error.to_string())?;
    definition.validate().map_err(|error| error.to_string())?;
    Ok(InputSample::valid(definition.id.clone(), value, Some(0)))
}

/// Build a same-invocation snapshot from canonical bone-rotation definitions.
/// Tick age zero means the just-queried sample belongs to this invocation; it
/// does not claim any wall-clock freshness beyond that invocation.
pub fn snapshot_from_bone_info(
    definitions: &[InputDefinition],
    response: &Value,
    policy: BoneAnglePolicy,
    wrapped: bool,
) -> Result<InputSnapshot, String> {
    let samples = definitions.iter()
        .map(|definition| sample_bone_rotation(definition, response, policy, wrapped))
        .collect::<Result<Vec<_>, _>>()?;
    InputSnapshot::new(0, samples).map_err(|error| error.to_string())
}

fn host_bone_id(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use smart_controller_core::{InputDimension, InputId};

    fn definition() -> InputDefinition {
        InputDefinition {
            id: InputId::new("head-rotation").unwrap(),
            source: InputSource::BoneTransform {
                bone_id: "42".into(),
                channel: BoneTransformChannel::Rotation,
            },
            dimension: InputDimension::Angle,
            fallback: smart_controller_core::FallbackPolicy::Reject,
        }
    }


    #[test]
    fn canonical_preview_orchestration_accepts_empty_document_as_noop() {
        let document = smart_controller_core::migration::CanonicalDocument::from_json(
            r#"{"schema":{"name":"smart-controller","version":1},"inputs":[],"controllers":[]}"#,
        ).unwrap();
        let snapshot = InputSnapshot::new(0, vec![]).unwrap();
        let mapper = |_controller: &smart_controller_core::canonical::Controller,
                      _inputs: &smart_controller_core::evaluator::ResolvedInputs|
         -> Result<std::collections::BTreeMap<smart_controller_core::BindingId, TypedValue>, smart_controller_core::CoreError> {
            panic!("mapper must not run for an empty document");
        };
        let plan = evaluate_document_preview(
            &document,
            &snapshot,
            &mapper,
            &smart_controller_core::composition::CompositionContext::default(),
        ).unwrap();
        assert!(plan.evaluation.evaluation.contributions.is_empty());
        assert!(plan.evaluation.composition.outputs.is_empty());
        assert!(plan.targets.is_empty());
    }

    #[test]
    fn canonical_preview_orchestration_rejects_invalid_document_before_mapping() {
        let document = smart_controller_core::migration::CanonicalDocument::from_json(
            r#"{"schema":{"name":"smart-controller","version":1},"inputs":[],"controllers":[]}"#,
        ).unwrap();
        let snapshot = InputSnapshot::new(0, vec![]).unwrap();
        let mapper = |_controller: &smart_controller_core::canonical::Controller,
                      _inputs: &smart_controller_core::evaluator::ResolvedInputs|
         -> Result<std::collections::BTreeMap<smart_controller_core::BindingId, TypedValue>, smart_controller_core::CoreError> {
            panic!("mapper must not run for an invalid document");
        };
        let mut invalid = document;
        invalid.schema.version = 99;
        assert!(evaluate_document_preview(
            &invalid,
            &snapshot,
            &mapper,
            &smart_controller_core::composition::CompositionContext::default(),
        ).is_err());
    }

    #[test]
    fn parses_keyed_pose_angle_in_degrees_as_unwrapped_radians() {
        let response = json!({"armatures":[{"bones":[{"id":42,"name":"Head","angle":90.0,"swing":5.0}]}]});
        let sample = sample_bone_rotation(&definition(), &response, BoneAnglePolicy::KeyedPoseOnly, false).unwrap();
        let angle = sample.value.unwrap().as_angle().unwrap();
        assert!((angle.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!(!angle.is_wrapped());
        assert_eq!(sample.age_ticks, Some(0));
    }

    #[test]
    fn spring_swing_is_added_only_when_explicitly_selected() {
        let response = json!({"armatures":[{"bones":[{"id":"42","angle":30.0,"swing":-10.0}]}]});
        let sample = sample_bone_rotation(&definition(), &response, BoneAnglePolicy::KeyedPosePlusSpringSwing, false).unwrap();
        let angle = sample.value.unwrap().as_angle().unwrap();
        assert!((angle.radians() - 20.0_f64.to_radians()).abs() < 1e-12);
    }

    #[test]
    fn missing_spring_swing_does_not_default_to_zero() {
        let response = json!({"armatures":[{"bones":[{"id":42,"angle":30.0}]}]});
        assert!(sample_bone_rotation(&definition(), &response, BoneAnglePolicy::KeyedPosePlusSpringSwing, false).is_err());
    }


    #[test]
    fn caller_selects_wrapped_angle_semantics_and_snapshot_is_invocation_scoped() {
        let response = json!({"armatures":[{"bones":[{"id":42,"angle":270.0}]}]});
        let wrapped = sample_bone_rotation(&definition(), &response, BoneAnglePolicy::KeyedPoseOnly, true).unwrap();
        assert!(wrapped.value.unwrap().as_angle().unwrap().is_wrapped());

        let snapshot = snapshot_from_bone_info(
            &[definition()],
            &response,
            BoneAnglePolicy::KeyedPoseOnly,
            false,
        ).unwrap();
        assert_eq!(snapshot.max_age_ticks(), 0);
        assert_eq!(snapshot.get(&definition().id).unwrap().age_ticks, Some(0));
    }

    #[test]
    fn missing_bone_and_malformed_response_fail_closed() {
        assert!(sample_bone_rotation(&definition(), &json!({"armatures":[]}), BoneAnglePolicy::KeyedPoseOnly, false).is_err());
        assert!(sample_bone_rotation(&definition(), &json!({"bones":[]}), BoneAnglePolicy::KeyedPoseOnly, false).is_err());
    }
}
