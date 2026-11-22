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
    value::{Angle, TypedValue},
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
        let required_inputs = match &controller.mapping {
            MappingConfig::JoystickFivePose { .. } => 2,
            MappingConfig::PoseGrid { axes, .. } => axes.len(),
            _ => 1,
        };
        if controller.inputs.len() != required_inputs {
            return Err(CoreError::InvalidMapping(format!(
                "mapping requires exactly {required_inputs} declared driver input(s); controller {} declares {}",
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
            MappingConfig::JoystickFivePose { center, up, right, down, left } => {
                let y_input_id = &controller.inputs[1];
                let y_input = inputs.get(y_input_id).ok_or_else(|| {
                    CoreError::InvalidMapping(format!("joystick Y input {y_input_id} was not resolved"))
                })?;
                let y_value = y_input.value.ok_or_else(|| {
                    CoreError::InvalidMapping(format!("joystick Y input {y_input_id} is unavailable"))
                })?;
                let x = scalar_driver(value)?;
                let y = scalar_driver(y_value)?;
                let pose = interpolate_joystick_five_pose(
                    [
                        self.poses.get(center),
                        self.poses.get(up),
                        self.poses.get(right),
                        self.poses.get(down),
                        self.poses.get(left),
                    ],
                    x,
                    y,
                )?;
                validate_pose_bindings(controller, &pose)?;
                Ok(pose.iter().map(|(id, value)| (id.clone(), *value)).collect())
            },
            MappingConfig::PoseGrid { axes, pose_ids } => {
                let mut driver_values = Vec::with_capacity(axes.len());
                for input_id in &controller.inputs {
                    let resolved = inputs.get(input_id).ok_or_else(|| {
                        CoreError::InvalidMapping(format!("PoseGrid input {input_id} was not resolved"))
                    })?;
                    let value = resolved.value.ok_or_else(|| {
                        CoreError::InvalidMapping(format!("PoseGrid input {input_id} is unavailable"))
                    })?;
                    driver_values.push(scalar_driver(value)?);
                }
                let key_axes: Vec<&[f64]> = axes.iter().map(|axis| axis.keys.as_slice()).collect();
                let pose = interpolate_pose_grid(&key_axes, pose_ids, &self.poses, &driver_values)?;
                validate_pose_bindings(controller, &pose)?;
                Ok(pose.iter().map(|(id, value)| (id.clone(), *value)).collect())
            },
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



/// Interpolate a complete 2D/3D Cartesian pose grid. Axis zero is the
/// fastest-varying dimension in pose_ids. Only positive-weight cell corners
/// are resolved, so exact keyforms do not depend on neighboring poses.
fn interpolate_pose_grid(
    axes: &[&[f64]],
    pose_ids: &[String],
    library: &PoseLibrary,
    inputs: &[f64],
) -> Result<Pose2D, CoreError> {
    if !(2..=3).contains(&axes.len()) || axes.len() != inputs.len() {
        return Err(CoreError::InvalidMapping("PoseGrid requires two or three scalar inputs".into()));
    }
    let mut segments = Vec::with_capacity(axes.len());
    for (keys, input) in axes.iter().zip(inputs) {
        if !input.is_finite() {
            return Err(CoreError::NonFiniteValue { context: "PoseGrid driver input" });
        }
        let value = input.clamp(keys[0], keys[keys.len() - 1]);
        let mut lower = keys.len() - 2;
        for index in 0..keys.len() - 1 {
            if value <= keys[index + 1] {
                lower = index;
                break;
            }
        }
        let amount = ((value - keys[lower]) / (keys[lower + 1] - keys[lower])).clamp(0.0, 1.0);
        segments.push((lower, amount));
    }

    let mut corners: Vec<(&Pose2D, f64)> = Vec::with_capacity(1 << axes.len());
    for mask in 0..(1usize << axes.len()) {
        let mut flat_index = 0usize;
        let mut stride = 1usize;
        let mut weight = 1.0;
        for (axis_index, (keys, (lower, amount))) in axes.iter().zip(&segments).enumerate() {
            let upper = (mask & (1 << axis_index)) != 0;
            let index = lower + usize::from(upper);
            weight *= if upper { *amount } else { 1.0 - *amount };
            flat_index += index * stride;
            stride *= keys.len();
        }
        if weight <= 0.0 { continue; }
        let pose_id = pose_ids.get(flat_index).ok_or_else(|| {
            CoreError::InvalidMapping("PoseGrid flattened index is out of range".into())
        })?;
        let pose = library.get(pose_id).ok_or_else(|| {
            CoreError::InvalidMapping(format!("PoseGrid references unregistered pose {pose_id}"))
        })?;
        if let Some((first, _)) = corners.first() {
            first.validate_compatible(pose)?;
        }
        corners.push((pose, weight));
    }
    blend_weighted_poses(&corners)
}

/// Blend typed values from a single cell using nonnegative tensor-product
/// weights. Wrapped angles use a circular mean; an undefined resultant fails.
fn blend_weighted_poses(corners: &[(&Pose2D, f64)]) -> Result<Pose2D, CoreError> {
    let (first, _) = corners.first().ok_or_else(|| {
        CoreError::InvalidMapping("PoseGrid produced no active interpolation corners".into())
    })?;
    let mut values = Vec::with_capacity(first.values().len());
    for (binding_id, first_value) in first.iter() {
        let samples: Vec<(TypedValue, f64)> = corners.iter().map(|(pose, weight)| {
            let value = pose.get(binding_id).copied().ok_or_else(|| {
                CoreError::PoseBindingMissing { binding_id: binding_id.to_string() }
            })?;
            Ok((value, *weight))
        }).collect::<Result<_, CoreError>>()?;
        let value = match first_value {
            TypedValue::Scalar(_) => {
                let result = samples.iter().map(|(value, weight)| {
                    value.as_scalar().expect("compatible pose type") * weight
                }).sum();
                TypedValue::scalar(result)?
            }
            TypedValue::Vector2(_) => {
                let x = samples.iter().map(|(value, weight)| value.as_vector2().expect("compatible pose type").x() * weight).sum();
                let y = samples.iter().map(|(value, weight)| value.as_vector2().expect("compatible pose type").y() * weight).sum();
                TypedValue::vector2(x, y)?
            }
            TypedValue::Angle(angle) if angle.is_wrapped() => {
                let sin_sum: f64 = samples.iter().map(|(value, weight)| value.as_angle().expect("compatible pose type").radians().sin() * weight).sum();
                let cos_sum: f64 = samples.iter().map(|(value, weight)| value.as_angle().expect("compatible pose type").radians().cos() * weight).sum();
                if sin_sum.hypot(cos_sum) <= 1e-12 {
                    return Err(CoreError::InvalidMapping(format!("PoseGrid has an ambiguous wrapped-angle blend for binding {binding_id}")));
                }
                TypedValue::angle(sin_sum.atan2(cos_sum), true)?
            }
            TypedValue::Angle(_) => {
                let result = samples.iter().map(|(value, weight)| value.as_angle().expect("compatible pose type").radians() * weight).sum();
                TypedValue::angle(result, false)?
            }
        };
        values.push((binding_id.clone(), value));
    }
    Pose2D::new(values)
}

/// Interpolate a five-pose joystick field using inverse-distance weighting.
///
/// Anchor order is center, up, right, down, left. Each axis is clamped to
/// [-1, 1]. Exact anchor hits return the authored pose without numerical
/// blending; elsewhere all five poses contribute with inverse squared
/// Euclidean distance. This gives deterministic, continuous interpolation
/// without inventing diagonal corner poses.
fn interpolate_joystick_five_pose(
    poses: [Option<&Pose2D>; 5],
    x: f64,
    y: f64,
) -> Result<Pose2D, CoreError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(CoreError::NonFiniteValue { context: "joystick input" });
    }
    let poses = poses.map(|pose| {
        pose.ok_or_else(|| CoreError::InvalidMapping("JoystickFivePose references an unregistered pose".into()))
    });
    let [center, up, right, down, left] = poses;
    let poses = [center?, up?, right?, down?, left?];
    for pose in poses.iter().skip(1) {
        poses[0].validate_compatible(pose)?;
    }

    let point = [x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0)];
    let anchors = [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [0.0, -1.0], [-1.0, 0.0]];
    let mut weights = [0.0; 5];
    for (index, anchor) in anchors.iter().enumerate() {
        let dx = point[0] - anchor[0];
        let dy = point[1] - anchor[1];
        let distance_squared = dx * dx + dy * dy;
        if distance_squared <= f64::EPSILON {
            return Ok(poses[index].clone());
        }
        weights[index] = 1.0 / distance_squared;
    }
    let total: f64 = weights.iter().sum();
    if !total.is_finite() || total <= 0.0 {
        return Err(CoreError::InvalidMapping("joystick pose weights are not finite".into()));
    }
    for weight in &mut weights {
        *weight /= total;
    }

    let mut values = Vec::with_capacity(poses[0].values().len());
    for (binding_id, first_value) in poses[0].iter() {
        let mut samples = Vec::with_capacity(5);
        samples.push(*first_value);
        for pose in poses.iter().skip(1) {
            samples.push(*pose.get(binding_id).ok_or_else(|| {
                CoreError::PoseBindingMissing { binding_id: binding_id.to_string() }
            })?);
        }
        let value = match samples.as_slice() {
            [TypedValue::Scalar(_), ..] => {
                let mut result = 0.0;
                for (sample, weight) in samples.iter().zip(weights) {
                    let TypedValue::Scalar(scalar) = sample else { unreachable!() };
                    result += scalar.get() * weight;
                }
                TypedValue::scalar(result)?
            }
            [TypedValue::Vector2(_), ..] => {
                let (mut x, mut y) = (0.0, 0.0);
                for (sample, weight) in samples.iter().zip(weights) {
                    let TypedValue::Vector2(vector) = sample else { unreachable!() };
                    x += vector.x() * weight;
                    y += vector.y() * weight;
                }
                TypedValue::vector2(x, y)?
            }
            [TypedValue::Angle(first), ..] if first.is_wrapped() => {
                let (mut sin_sum, mut cos_sum) = (0.0, 0.0);
                for (sample, weight) in samples.iter().zip(weights) {
                    let TypedValue::Angle(angle) = sample else { unreachable!() };
                    sin_sum += angle.radians().sin() * weight;
                    cos_sum += angle.radians().cos() * weight;
                }
                if sin_sum.hypot(cos_sum) <= 1e-12 {
                    return Err(CoreError::InvalidMapping(format!(
                        "joystick field has an ambiguous wrapped-angle blend for binding {binding_id}"
                    )));
                }
                TypedValue::Angle(Angle::new(sin_sum.atan2(cos_sum), true)?)
            }
            [TypedValue::Angle(_), ..] => {
                let mut result = 0.0;
                for (sample, weight) in samples.iter().zip(weights) {
                    let TypedValue::Angle(angle) = sample else { unreachable!() };
                    result += angle.radians() * weight;
                }
                TypedValue::angle(result, false)?
            }
            _ => return Err(CoreError::InvalidMapping("joystick poses contain incompatible value types".into())),
        };
        values.push((binding_id.clone(), value));
    }
    Pose2D::new(values)
}

#[cfg(test)]
mod joystick_field_tests {
    use super::*;
    use crate::ids::BindingId;

    fn scalar_pose(value: f64) -> Pose2D {
        Pose2D::new(vec![(
            BindingId::new("output").unwrap(),
            TypedValue::scalar(value).unwrap(),
        )]).unwrap()
    }

    #[test]
    fn joystick_returns_exact_authored_cardinal_poses() {
        let poses = [
            scalar_pose(0.0),
            scalar_pose(10.0),
            scalar_pose(20.0),
            scalar_pose(-10.0),
            scalar_pose(-20.0),
        ];
        for (x, y, expected) in [
            (0.0, 0.0, 0.0),
            (0.0, 1.0, 10.0),
            (1.0, 0.0, 20.0),
            (0.0, -1.0, -10.0),
            (-1.0, 0.0, -20.0),
        ] {
            let refs = poses.each_ref().map(Some);
            let result = interpolate_joystick_five_pose(refs, x, y).unwrap();
            assert_eq!(result.get(&BindingId::new("output").unwrap()).unwrap().as_scalar(), Some(expected));
        }
    }

    #[test]
    fn joystick_clamps_axes_and_interpolates_diagonal_values() {
        let poses = [
            scalar_pose(0.0),
            scalar_pose(10.0),
            scalar_pose(20.0),
            scalar_pose(-10.0),
            scalar_pose(-20.0),
        ];
        let refs = poses.each_ref().map(Some);
        let clamped = interpolate_joystick_five_pose(refs, 2.0, 0.0).unwrap();
        assert_eq!(clamped.get(&BindingId::new("output").unwrap()).unwrap().as_scalar(), Some(20.0));

        let refs = poses.each_ref().map(Some);
        let diagonal = interpolate_joystick_five_pose(refs, 0.5, 0.5).unwrap();
        let value = diagonal.get(&BindingId::new("output").unwrap()).unwrap().as_scalar().unwrap();
        assert!(value.is_finite());
        assert!(value > 0.0 && value < 20.0);
    }

    #[test]
    fn joystick_interpolates_vector_outputs_and_rejects_ambiguous_wrapped_angles() {
        let vector_pose = |x, y| Pose2D::new(vec![(
            BindingId::new("vector").unwrap(),
            TypedValue::vector2(x, y).unwrap(),
        )]).unwrap();
        let poses = [
            vector_pose(0.0, 0.0),
            vector_pose(0.0, 10.0),
            vector_pose(10.0, 0.0),
            vector_pose(0.0, -10.0),
            vector_pose(-10.0, 0.0),
        ];
        let refs = poses.each_ref().map(Some);
        let result = interpolate_joystick_five_pose(refs, 0.5, 0.5).unwrap();
        let value = result.get(&BindingId::new("vector").unwrap()).unwrap().as_vector2().unwrap();
        assert!(value.x().is_finite() && value.y().is_finite());

        let angle_pose = |radians| Pose2D::new(vec![(
            BindingId::new("angle").unwrap(),
            TypedValue::angle(radians, true).unwrap(),
        )]).unwrap();
        let near_pi = std::f64::consts::PI - 0.05;
        let angles = [
            angle_pose(near_pi),
            angle_pose(-near_pi),
            angle_pose(near_pi),
            angle_pose(near_pi),
            angle_pose(near_pi),
        ];
        let refs = angles.each_ref().map(Some);
        let result = interpolate_joystick_five_pose(refs, 0.5, 0.0).unwrap();
        let angle = result.get(&BindingId::new("angle").unwrap()).unwrap().as_angle().unwrap();
        assert!(angle.radians().abs() > 3.0);
    }
}

#[cfg(test)]
mod pose_grid_tests {
    use super::*;
    use crate::ids::BindingId;

    fn scalar_pose(value: f64) -> Pose2D {
        Pose2D::new(vec![(BindingId::new("out").unwrap(), TypedValue::scalar(value).unwrap())]).unwrap()
    }

    fn scalar(pose: &Pose2D) -> f64 {
        pose.get(&BindingId::new("out").unwrap()).unwrap().as_scalar().unwrap()
    }

    #[test]
    fn bilinear_grid_uses_nonuniform_spacing_and_axis_zero_fastest() {
        let mut library = PoseLibrary::new();
        for (id, value) in [("a",30.0),("b",34.0),("c",60.0),("d",64.0)] {
            library.register(id, scalar_pose(value)).unwrap();
        }
        let result = interpolate_pose_grid(
            &[&[0.0, 2.0], &[10.0, 20.0]],
            &["a".into(),"b".into(),"c".into(),"d".into()],
            &library,
            &[1.0, 15.0],
        ).unwrap();
        assert!((scalar(&result) - 47.0).abs() < 1e-12);
        let clamped = interpolate_pose_grid(
            &[&[0.0, 2.0], &[10.0, 20.0]],
            &["a".into(),"b".into(),"c".into(),"d".into()],
            &library,
            &[9.0, 15.0],
        ).unwrap();
        assert!((scalar(&clamped) - 49.0).abs() < 1e-12);
    }

    #[test]
    fn trilinear_grid_uses_axis_zero_fastest_flattening() {
        let mut library = PoseLibrary::new();
        for (id, value) in [("p0",0.0),("p1",1.0),("p2",10.0),("p3",11.0),("p4",100.0),("p5",101.0),("p6",110.0),("p7",111.0)] {
            library.register(id, scalar_pose(value)).unwrap();
        }
        let ids = ["p0","p1","p2","p3","p4","p5","p6","p7"].map(str::to_owned);
        let result = interpolate_pose_grid(&[&[0.0,1.0],&[0.0,1.0],&[0.0,1.0]], &ids, &library, &[0.5,0.5,0.5]).unwrap();
        assert!((scalar(&result) - 55.5).abs() < 1e-12);
    }

    #[test]
    fn exact_keyform_does_not_resolve_zero_weight_neighbor_poses() {
        let mut library = PoseLibrary::new();
        library.register("only", scalar_pose(42.0)).unwrap();
        let ids = vec!["only".into(),"missing-x".into(),"missing-y".into(),"missing-xy".into()];
        let result = interpolate_pose_grid(&[&[0.0,1.0],&[0.0,1.0]], &ids, &library, &[0.0,0.0]).unwrap();
        assert_eq!(scalar(&result), 42.0);
    }

    #[test]
    fn wrapped_angles_blend_circularly_across_the_pi_seam() {
        let angle_pose = |radians| Pose2D::new(vec![(
            BindingId::new("angle").unwrap(),
            TypedValue::angle(radians, true).unwrap(),
        )]).unwrap();
        let mut library = PoseLibrary::new();
        let near_pi = std::f64::consts::PI - 0.05;
        library.register("a", angle_pose(near_pi)).unwrap();
        library.register("b", angle_pose(-near_pi)).unwrap();
        library.register("c", angle_pose(near_pi)).unwrap();
        library.register("d", angle_pose(-near_pi)).unwrap();
        let result = interpolate_pose_grid(&[&[0.0,1.0],&[0.0,1.0]], &["a".into(),"b".into(),"c".into(),"d".into()], &library, &[0.5,0.5]).unwrap();
        let angle = result.get(&BindingId::new("angle").unwrap()).unwrap().as_angle().unwrap();
        assert!(angle.radians().abs() > 3.0);
    }
}