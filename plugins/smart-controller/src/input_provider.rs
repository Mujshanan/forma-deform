//! Per-invocation host input collection for canonical controller documents.
//!
//! Host identity resolution is explicit: callers supply the layer associated
//! with each canonical bone ID. No layer or bone identity is guessed.
use std::collections::BTreeMap;

use smart_controller_core::{
    BoneTransformChannel, InputDefinition, InputId, InputSample, InputSnapshot,
    InputSource, TypedValue,
    migration::CanonicalDocument,
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

/// Collect only external inputs referenced by enabled controllers in a
/// validated document. Disabled controller inputs are intentionally not
/// queried or required; ControllerOutput inputs remain DAG-owned.
pub fn collect_document_input_snapshot(
    document: &CanonicalDocument,
    context: &HostInputContext,
) -> Result<InputSnapshot, String> {
    document.validate().map_err(|error| error.to_string())?;
    let active: std::collections::BTreeSet<InputId> = document.controllers.iter()
        .filter(|controller| controller.enabled)
        .flat_map(|controller| controller.inputs.iter().cloned())
        .collect();
    let definitions: Vec<InputDefinition> = document.inputs.iter()
        .filter(|definition| active.contains(&definition.id))
        .cloned()
        .collect();
    collect_host_input_snapshot(&definitions, context)
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
                match context.manual_values.get(&definition.id).copied() {
                    Some(value) if definition.dimension.accepts(value) =>
                        InputSample::valid(definition.id.clone(), value, Some(0)),
                    Some(_) => InputSample::invalid(definition.id.clone()),
                    None => InputSample::missing(definition.id.clone()),
                }
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
                    if let Ok(response) = query_bone_info(layer_id, None) {
                        responses.insert(layer_id, response);
                    } else {
                        // A failed host read is explicitly unavailable; the
                        // core applies the authored fallback policy.
                        samples.push(InputSample::unavailable(definition.id.clone()));
                        continue;
                    }
                }
                match sample_bone_rotation(
                    definition,
                    responses.get(&layer_id).expect("response inserted"),
                    context.bone_angle_policy,
                    context.wrapped_angles,
                ) {
                    Ok(sample) => sample,
                    Err(_) => InputSample::invalid(definition.id.clone()),
                }
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
    fn missing_manual_input_is_reported_to_core_for_fallback_resolution() {
        let definition = InputDefinition {
            id: InputId::new("optional-manual").unwrap(),
            source: InputSource::Manual,
            dimension: InputDimension::Scalar,
            fallback: FallbackPolicy::UseValue(TypedValue::scalar(3.0).unwrap()),
        };
        let context = HostInputContext {
            manual_values: BTreeMap::new(),
            bone_layers: BTreeMap::new(),
            bone_angle_policy: BoneAnglePolicy::KeyedPoseOnly,
            wrapped_angles: false,
        };
        let snapshot = collect_host_input_snapshot(&[definition.clone()], &context).unwrap();
        let sample = snapshot.get(&definition.id).unwrap();
        assert_eq!(sample.validity, smart_controller_core::InputValidity::Missing);
        assert!(sample.value.is_none());
    }

    #[test]
    fn disabled_controller_inputs_are_not_required_or_queried() {
        let input_id = InputId::new("disabled-manual").unwrap();
        let document = CanonicalDocument {
            schema: smart_controller_core::schema::SchemaIdentity {
                name: "smart-controller".into(),
                version: 1,
            },
            inputs: vec![InputDefinition {
                id: input_id.clone(),
                source: InputSource::Manual,
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            }],
            controllers: vec![smart_controller_core::canonical::Controller {
                id: smart_controller_core::ControllerId::new("disabled").unwrap(),
                enabled: false,
                inputs: vec![input_id.clone()],
                dependencies: vec![],
                mapping: smart_controller_core::canonical::MappingConfig::Range1D {
                    minimum: 0.0,
                    neutral: 1.0,
                    maximum: 2.0,
                },
                bindings: vec![],
                parent_scoped: false,
            }],
        };
        let context = HostInputContext {
            manual_values: BTreeMap::new(),
            bone_layers: BTreeMap::new(),
            bone_angle_policy: BoneAnglePolicy::KeyedPoseOnly,
            wrapped_angles: false,
        };
        let snapshot = collect_document_input_snapshot(&document, &context).unwrap();
        assert!(snapshot.samples.is_empty());
        assert!(snapshot.get(&input_id).is_none());
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
