//! Per-invocation host input collection for canonical controller documents.
//!
//! Host identity resolution is explicit: callers supply the layer associated
//! with each canonical bone ID. No layer or bone identity is guessed.
use std::collections::BTreeMap;

use smart_controller_core::{
    BoneTransformChannel, InputDefinition, InputId, InputSample, InputSnapshot,
    InputSource, TypedValue,
};

use crate::adapter::{query_bone_info, sample_bone_rotation, BoneAnglePolicy};

/// Explicit host-side values and identity resolution for one preview invocation.
#[derive(Clone, Debug)]
pub struct HostInputContext {
    /// Values supplied by the user/UI for Manual inputs.
    pub manual_values: BTreeMap<InputId, TypedValue>,
    /// Canonical bone ID -> AniCraft layer ID. Required for BoneTransform inputs.
    pub bone_layers: BTreeMap<String, u64>,
    pub bone_angle_policy: BoneAnglePolicy,
    /// Angle wrapping is authored semantic metadata, never inferred from a host angle.
    pub wrapped_angles: bool,
}

/// Collect all external inputs before evaluation. ControllerOutput inputs are
/// deliberately omitted: the core evaluator resolves them from the DAG.
pub fn collect_host_input_snapshot(
    definitions: &[InputDefinition],
    context: &HostInputContext,
) -> Result<InputSnapshot, String> {
    let mut responses: BTreeMap<u64, serde_json::Value> = BTreeMap::new();
    let mut samples = Vec::new();

    for definition in definitions {
        definition.validate().map_err(|error| error.to_string())?;
        let sample = match &definition.source {
            InputSource::Manual => {
                let value = context.manual_values.get(&definition.id).copied()
                    .ok_or_else(|| format!("manual input {} has no supplied value", definition.id))?;
                InputSample::valid(definition.id.clone(), value, Some(0))
            }
            InputSource::BoneTransform { bone_id, channel } => {
                if *channel != BoneTransformChannel::Rotation {
                    return Err(format!(
                        "bone input {} uses {:?}; only Rotation is currently supported by the host adapter",
                        definition.id, channel
                    ));
                }
                let layer_id = *context.bone_layers.get(bone_id)
                    .ok_or_else(|| format!("bone input {} has no explicit layer mapping for bone {}", definition.id, bone_id))?;
                if !responses.contains_key(&layer_id) {
                    responses.insert(layer_id, query_bone_info(layer_id, None)?);
                }
                sample_bone_rotation(
                    definition,
                    responses.get(&layer_id).expect("response inserted"),
                    context.bone_angle_policy,
                    context.wrapped_angles,
                )?
            }
            InputSource::ControllerOutput { .. } => continue,
            InputSource::LegacyScalar { source_id } => {
                return Err(format!(
                    "legacy scalar input {} ({}) requires an explicit migration/provider mapping",
                    definition.id, source_id
                ));
            }
        };
        definition.validate_sample(&sample).map_err(|error| error.to_string())?;
        samples.push(sample);
    }

    // Every sample was queried or supplied during this invocation. This does
    // not imply freshness outside it and intentionally uses a zero-age window.
    InputSnapshot::new(0, samples).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_controller_core::{FallbackPolicy, InputDimension, InputId};

    #[test]
    fn manual_collection_is_invocation_scoped_and_omits_controller_outputs() {
        let manual_id = InputId::new("manual").unwrap();
        let derived_id = InputId::new("derived").unwrap();
        let definitions = vec![
            InputDefinition {
                id: manual_id.clone(),
                source: InputSource::Manual,
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            },
            InputDefinition {
                id: derived_id,
                source: InputSource::ControllerOutput {
                    controller_id: smart_controller_core::ControllerId::new("upstream").unwrap(),
                    binding_id: smart_controller_core::BindingId::new("out").unwrap(),
                },
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            },
        ];
        let context = HostInputContext {
            manual_values: BTreeMap::from([(manual_id.clone(), TypedValue::scalar(2.5).unwrap())]),
            bone_layers: BTreeMap::new(),
            bone_angle_policy: BoneAnglePolicy::KeyedPoseOnly,
            wrapped_angles: false,
        };
        let snapshot = collect_host_input_snapshot(&definitions, &context).unwrap();
        assert_eq!(snapshot.samples.len(), 1);
        assert_eq!(snapshot.get(&manual_id).unwrap().age_ticks, Some(0));
        assert_eq!(snapshot.max_age_ticks(), 0);
    }

    #[test]
    fn missing_manual_value_fails_closed() {
        let definition = InputDefinition {
            id: InputId::new("manual").unwrap(),
            source: InputSource::Manual,
            dimension: InputDimension::Scalar,
            fallback: FallbackPolicy::Reject,
        };
        assert!(collect_host_input_snapshot(&[definition], &HostInputContext { manual_values: BTreeMap::new(), bone_layers: BTreeMap::new(), bone_angle_policy: BoneAnglePolicy::KeyedPoseOnly, wrapped_angles: false }).is_err());
    }
}
