//! First-party mapper for the canonical Smart Controller document model.
//!
//! This module maps one explicitly declared driver input to typed binding
//! outputs. It is pure: it does not query the host, mutate document state,
//! compose contributions, or apply deformation.
//!
//! Scalar one-dimensional drivers use the authored numeric domain:
//! - Range1D / ThreeAnchorRange use their explicit minimum, neutral and maximum.
//! - DirectedRange1D maps its directed start/end interval to the signed range.
//! - Discrete and PoseSet1D use a normalized signed driver domain of [-1, 1].
//!
//! PoseSet1D pose data is supplied by the runtime PoseLibrary. The library is
//! deliberately separate from canonical schema v1 until durable pose storage
//! has an explicit, versioned document-extension contract.

use std::collections::{BTreeMap, HashSet};

use crate::{
    canonical::{BindingMode, Controller, MappingConfig, TargetMapping},
    error::CoreError,
    evaluator::{ControllerMapper, ResolvedInputs},
    ids::BindingId,
    mapping::{easing::Easing, frame, range::{normalize_directed, ThreePointRange}},
    pose::Pose2D,
    value::TypedValue,
};

/// Runtime lookup table for authored poses referenced by canonical pose IDs.
///
/// This is an evaluation dependency, not a persistence mechanism. Registration
/// rejects empty and duplicate identities so a controller cannot resolve a
/// pose ambiguously.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PoseLibrary {
    poses: BTreeMap<String, Pose2D>,
}

impl PoseLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        pose_id: impl Into<String>,
        pose: Pose2D,
    ) -> Result<(), CoreError> {
        let pose_id = pose_id.into();
        if pose_id.trim().is_empty() {
            return Err(CoreError::InvalidMapping("pose identity must not be empty".into()));
        }
        if self.poses.contains_key(&pose_id) {
            return Err(CoreError::InvalidMapping(format!("pose identity {pose_id} is already registered")));
        }
        self.poses.insert(pose_id, pose);
        Ok(())
    }

    pub fn get(&self, pose_id: &str) -> Option<&Pose2D> {
        self.poses.get(pose_id)
    }

    pub fn len(&self) -> usize {
        self.poses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.poses.is_empty()
    }
}

/// Built-in canonical mapper for one-dimensional controller configurations.
///
/// The mapper supports Range1D, ThreeAnchorRange, DirectedRange1D, Discrete
/// and PoseSet1D. JoystickFivePose remains an explicit unsupported capability
/// here; it requires a two-dimensional pose-field mapper and is not silently
/// coerced into a scalar mapping.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanonicalControllerMapper {
    poses: PoseLibrary,
}

impl CanonicalControllerMapper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pose_library(poses: PoseLibrary) -> Self {
        Self { poses }
    }

    pub fn pose_library(&self) -> &PoseLibrary {
        &self.poses
    }

    pub fn pose_library_mut(&mut self) -> &mut PoseLibrary {
        &mut self.poses
    }

    pub fn register_pose(
        &mut self,
        pose_id: impl Into<String>,
        pose: Pose2D,
    ) -> Result<(), CoreError> {
        self.poses.register(pose_id, pose)
    }
}

impl ControllerMapper for CanonicalControllerMapper {
    fn evaluate(
        &self,
        controller: &Controller,
        inputs: &ResolvedInputs,
    ) -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
        controller.validate_local()?;
        if controller.inputs.len() != 1 {
            return Err(CoreError::InvalidMapping(format!(
                "canonical mapping currently requires exactly one declared driver input; controller {} declares {}",
                controller.id,
                controller.inputs.len()
            )));
        }

        let input_id = &controller.inputs[0];
        let input = inputs.get(input_id).ok_or_else(|| {
            CoreError::InvalidMapping(format!("driver input {input_id} was not resolved"))
        })?;
        let value = input.value.ok_or_else(|| {
            CoreError::InvalidMapping(format!("driver input {input_id} is unavailable"))
        })?;

        match &controller.mapping {
            MappingConfig::Range1D { minimum, neutral, maximum } => {
                let (raw, angle_wrapped) = numeric_driver(value)?;
                let range = ThreePointRange::new(*minimum, *neutral, *maximum)?;
                let signed = range.normalize_signed(raw)?;
                map_anchored_bindings(controller, signed, Easing::Linear, angle_wrapped)
            }
            MappingConfig::ThreeAnchorRange { minimum, neutral, maximum, easing } => {
                let (raw, angle_wrapped) = numeric_driver(value)?;
                let range = ThreePointRange::new(*minimum, *neutral, *maximum)?;
                let signed = range.normalize_signed(raw)?;
                let easing = match easing {
                    Easing::Linear => Easing::Linear,
                    Easing::Smoothstep => Easing::Smoothstep,
                };
                map_anchored_bindings(controller, signed, easing, angle_wrapped)
            }
            MappingConfig::DirectedRange1D { start, end } => {
                let (raw, angle_wrapped) = numeric_driver(value)?;
                let unit = normalize_directed(raw, *start, *end)?;
                let signed = unit * 2.0 - 1.0;
                map_anchored_bindings_with_unit(controller, signed, unit, Easing::Linear, angle_wrapped)
            }
            MappingConfig::Discrete { values } => {
                let raw = scalar_driver(value)?;
                if controller.bindings.len() != 1 {
                    return Err(CoreError::InvalidMapping(
                        "Discrete mapping requires exactly one output binding".into(),
                    ));
                }
                let unit = signed_to_unit(raw);
                let index = nearest_discrete_index(unit, values.len());
                Ok(BTreeMap::from([(
                    controller.bindings[0].id.clone(),
                    values[index],
                )]))
            }
            MappingConfig::PoseSet1D { pose_ids } => {
                let raw = scalar_driver(value)?;
                let unit = signed_to_unit(raw);
                let poses = pose_ids
                    .iter()
                    .map(|pose_id| {
                        self.poses.get(pose_id).ok_or_else(|| {
                            CoreError::InvalidMapping(format!(
                                "PoseSet1D references unregistered pose {pose_id}"
                            ))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let pose = interpolate_pose_set(&poses, unit)?;
                validate_pose_bindings(controller, &pose)?;
                Ok(pose.iter().map(|(id, value)| (id.clone(), *value)).collect())
            }
            MappingConfig::JoystickFivePose { .. } => Err(CoreError::InvalidMapping(
                "JoystickFivePose requires the dedicated two-dimensional pose mapper; scalar canonical mapping does not approximate it".into(),
            )),
        }
    }
}

fn scalar_driver(value: TypedValue) -> Result<f64, CoreError> {
    value.as_scalar().ok_or_else(|| {
        CoreError::InvalidMapping(format!(
            "mapping requires a scalar driver input, received {}",
            value.kind_name()
        ))
    })
}

fn numeric_driver(value: TypedValue) -> Result<(f64, Option<bool>), CoreError> {
    match value {
        TypedValue::Scalar(value) => Ok((value.get(), None)),
        TypedValue::Angle(value) => Ok((value.radians(), Some(value.is_wrapped()))),
        other => Err(CoreError::InvalidMapping(format!(
            "one-dimensional range mapping requires scalar or angle input, received {}",
            other.kind_name()
        ))),
    }
}

fn signed_to_unit(signed: f64) -> f64 {
    (signed.clamp(-1.0, 1.0) + 1.0) * 0.5
}

fn nearest_discrete_index(unit: f64, count: usize) -> usize {
    debug_assert!(count > 0);
    if count <= 1 {
        return 0;
    }
    ((unit.clamp(0.0, 1.0) * (count - 1) as f64).round() as usize).min(count - 1)
}

fn map_anchored_bindings(
    controller: &Controller,
    signed: f64,
    easing: Easing,
    angle_wrapped: Option<bool>,
) -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
    let unit = signed_to_unit(signed);
    map_anchored_bindings_with_unit(controller, signed, unit, easing, angle_wrapped)
}

fn map_anchored_bindings_with_unit(
    controller: &Controller,
    signed: f64,
    unit: f64,
    easing: Easing,
    angle_wrapped: Option<bool>,
) -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
    let eased = ease_signed(signed, easing)?;
    let mut outputs = BTreeMap::new();
    for binding in &controller.bindings {
        let value = match &binding.target {
            TargetMapping::SymbolFrame {
                negative_frame,
                neutral_frame,
                positive_frame,
                ..
            } => {
                let frame_value = interpolate_anchors(
                    *negative_frame as f64,
                    *neutral_frame as f64,
                    *positive_frame as f64,
                    eased,
                )
                .round();
                TypedValue::scalar(frame_value)?
            }
            TargetMapping::FrameRange {
                start_frame,
                end_frame,
                endpoint_policy,
                ..
            } => {
                let policy = endpoint_policy.unwrap_or_default();
                let frame_value = frame::map_to_frame(unit, *start_frame, *end_frame, policy)?;
                TypedValue::scalar(frame_value as f64)?
            }
            TargetMapping::GeometryDeform {
                negative_offset,
                neutral_offset,
                positive_offset,
                ..
            } => {
                let x = interpolate_anchors(
                    negative_offset[0],
                    neutral_offset[0],
                    positive_offset[0],
                    eased,
                );
                let y = interpolate_anchors(
                    negative_offset[1],
                    neutral_offset[1],
                    positive_offset[1],
                    eased,
                );
                let (x, y) = if binding.mode == BindingMode::RelativeDelta {
                    (x - neutral_offset[0], y - neutral_offset[1])
                } else {
                    (x, y)
                };
                TypedValue::vector2(x, y)?
            }
            TargetMapping::Property { scale, .. } => {
                let mapped = eased * *scale;
                match angle_wrapped {
                    Some(wrapped) => TypedValue::angle(mapped, wrapped)?,
                    None => TypedValue::scalar(mapped)?,
                }
            }
        };
        outputs.insert(binding.id.clone(), value);
    }
    Ok(outputs)
}

/// Interpolate neutral toward the negative or positive anchor. Easing is
/// applied symmetrically so the neutral point remains exactly unchanged.
fn ease_signed(value: f64, easing: Easing) -> Result<f64, CoreError> {
    if !value.is_finite() {
        return Err(CoreError::NonFiniteValue { context: "signed mapping input" });
    }
    if value < 0.0 {
        Ok(-easing.apply(-value)?)
    } else {
        easing.apply(value)
    }
}

fn interpolate_anchors(negative: f64, neutral: f64, positive: f64, signed: f64) -> f64 {
    if signed < 0.0 {
        negative * -signed + neutral * (1.0 + signed)
    } else {
        neutral * (1.0 - signed) + positive * signed
    }
}

fn interpolate_pose_set(poses: &[&Pose2D], unit: f64) -> Result<Pose2D, CoreError> {
    if poses.is_empty() {
        return Err(CoreError::InvalidMapping("PoseSet1D requires at least one registered pose".into()));
    }
    for pose in poses.iter().skip(1) {
        poses[0].validate_compatible(pose)?;
    }
    if poses.len() == 1 {
        return Ok(poses[0].clone());
    }

    let position = unit.clamp(0.0, 1.0) * (poses.len() - 1) as f64;
    let lower_index = (position.floor() as usize).min(poses.len() - 1);
    let upper_index = (lower_index + 1).min(poses.len() - 1);
    let amount = (position - lower_index as f64).clamp(0.0, 1.0);
    let lower = poses[lower_index];
    let upper = poses[upper_index];

    let mut values = Vec::with_capacity(lower.values().len());
    for (binding_id, lower_value) in lower.iter() {
        let upper_value = upper.get(binding_id).ok_or_else(|| {
            CoreError::PoseBindingMissing { binding_id: binding_id.to_string() }
        })?;
        values.push((binding_id.clone(), lower_value.interpolate(*upper_value, amount)?));
    }
    Pose2D::new(values)
}

fn validate_pose_bindings(controller: &Controller, pose: &Pose2D) -> Result<(), CoreError> {
    let expected: HashSet<&BindingId> = controller.bindings.iter().map(|binding| &binding.id).collect();
    let actual: HashSet<&BindingId> = pose.iter().map(|(binding_id, _)| binding_id).collect();
    if expected != actual {
        let missing: Vec<String> = expected.difference(&actual).map(|id| id.to_string()).collect();
        let unexpected: Vec<String> = actual.difference(&expected).map(|id| id.to_string()).collect();
        return Err(CoreError::InvalidMapping(format!(
            "pose binding set does not match controller {} (missing: [{}]; unexpected: [{}])",
            controller.id,
            missing.join(", "),
            unexpected.join(", ")
        )));
    }
    Ok(())
}
