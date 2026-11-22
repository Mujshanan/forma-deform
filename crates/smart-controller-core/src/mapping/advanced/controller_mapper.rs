//! Runtime bridge from advanced 2D pose fields into the controller mapper contract.
//!
//! This is deliberately an execution-time registry, not a canonical document
//! schema extension. V1 controller serialization remains unchanged. A caller
//! explicitly registers advanced mappings by stable ControllerId and identifies
//! the Vector2 input used to sample the pose field. Unregistered controllers
//! continue through the supplied mapper unchanged.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    canonical::Controller,
    error::CoreError,
    evaluator::{ControllerMapper, ResolvedInputs},
    ids::{BindingId, ControllerId, InputId},
    pose::Pose2D,
    value::{TypedValue, Vector2},
};

use super::{
    PoseMapping2D,
    grid::GridMapping,
    triangulated::TriangulatedMapping,
};

/// An owned advanced mapping implementation selected explicitly at runtime.
#[derive(Clone, Debug, PartialEq)]
pub enum AdvancedPoseMapping {
    Grid(GridMapping),
    Triangulated(TriangulatedMapping),
}

impl PoseMapping2D for AdvancedPoseMapping {
    fn map_pose(&self, point: Vector2) -> Result<Pose2D, CoreError> {
        match self {
            Self::Grid(mapping) => mapping.map_pose(point),
            Self::Triangulated(mapping) => mapping.map_pose(point),
        }
    }
}

/// Runtime association between one controller, its 2D driver input and a pose field.
#[derive(Clone, Debug, PartialEq)]
pub struct AdvancedMappingEntry {
    pub input_id: InputId,
    pub mapping: AdvancedPoseMapping,
}

/// Explicit runtime registrations for controllers using advanced 2D pose mapping.
///
/// This registry is intentionally not serialized into canonical schema v1.
/// Persistence of authored advanced mappings requires a separately versioned
/// extension contract and is not implied by this runtime bridge.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdvancedMappingRegistry {
    entries: BTreeMap<ControllerId, AdvancedMappingEntry>,
}

impl AdvancedMappingRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        controller_id: ControllerId,
        input_id: InputId,
        mapping: AdvancedPoseMapping,
    ) -> Result<(), CoreError> {
        if self.entries.contains_key(&controller_id) {
            return Err(CoreError::InvalidMapping(format!(
                "an advanced mapping is already registered for controller {controller_id}"
            )));
        }
        self.entries.insert(controller_id, AdvancedMappingEntry { input_id, mapping });
        Ok(())
    }

    pub fn register_grid(
        &mut self,
        controller_id: ControllerId,
        input_id: InputId,
        mapping: GridMapping,
    ) -> Result<(), CoreError> {
        self.register(controller_id, input_id, AdvancedPoseMapping::Grid(mapping))
    }

    pub fn register_triangulated(
        &mut self,
        controller_id: ControllerId,
        input_id: InputId,
        mapping: TriangulatedMapping,
    ) -> Result<(), CoreError> {
        self.register(controller_id, input_id, AdvancedPoseMapping::Triangulated(mapping))
    }

    pub fn get(&self, controller_id: &ControllerId) -> Option<&AdvancedMappingEntry> {
        self.entries.get(controller_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ControllerId, &AdvancedMappingEntry)> {
        self.entries.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Decorates an existing mapper with explicit advanced-pose registrations.
///
/// Registered controllers use the advanced pose field. Every other controller
/// is delegated to the fallback mapper, preserving existing mapping behaviour.
/// A registered controller never silently falls back if its advanced mapping
/// fails validation or evaluation.
pub struct AdvancedControllerMapper<M> {
    registry: AdvancedMappingRegistry,
    fallback: M,
}

impl<M> AdvancedControllerMapper<M> {
    pub fn new(fallback: M, registry: AdvancedMappingRegistry) -> Self {
        Self { registry, fallback }
    }

    pub fn registry(&self) -> &AdvancedMappingRegistry {
        &self.registry
    }

    pub fn fallback(&self) -> &M {
        &self.fallback
    }
}

impl<M: ControllerMapper> ControllerMapper for AdvancedControllerMapper<M> {
    fn evaluate(
        &self,
        controller: &Controller,
        inputs: &ResolvedInputs,
    ) -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
        let Some(entry) = self.registry.get(&controller.id) else {
            return self.fallback.evaluate(controller, inputs);
        };

        if !controller.inputs.contains(&entry.input_id) {
            return Err(CoreError::InvalidMapping(format!(
                "advanced mapping for controller {} references input {} that is not declared by the controller",
                controller.id, entry.input_id
            )));
        }

        let input = inputs.get(&entry.input_id).ok_or_else(|| {
            CoreError::InvalidMapping(format!(
                "advanced mapping driver input {} was not resolved",
                entry.input_id
            ))
        })?;
        let value = input.value.ok_or_else(|| {
            CoreError::InvalidMapping(format!(
                "advanced mapping driver input {} is unavailable",
                entry.input_id
            ))
        })?;
        let point = match value {
            TypedValue::Vector2(point) => point,
            other => {
                return Err(CoreError::InvalidMapping(format!(
                    "advanced mapping driver input {} must be vector2, received {}",
                    entry.input_id,
                    other.kind_name()
                )));
            }
        };

        let pose = entry.mapping.map_pose(point)?;
        validate_pose_bindings(controller, &pose)?;

        Ok(pose.iter().map(|(id, value)| (id.clone(), *value)).collect())
    }
}

fn validate_pose_bindings(controller: &Controller, pose: &Pose2D) -> Result<(), CoreError> {
    let expected: BTreeSet<&BindingId> =
        controller.bindings.iter().map(|binding| &binding.id).collect();
    let actual: BTreeSet<&BindingId> = pose.iter().map(|(binding_id, _)| binding_id).collect();

    if expected != actual {
        let missing: Vec<String> = expected
            .difference(&actual)
            .map(|id| id.to_string())
            .collect();
        let unexpected: Vec<String> = actual
            .difference(&expected)
            .map(|id| id.to_string())
            .collect();
        return Err(CoreError::InvalidMapping(format!(
            "advanced pose binding set does not match controller {} (missing: [{}]; unexpected: [{}])",
            controller.id,
            missing.join(", "),
            unexpected.join(", ")
        )));
    }
    Ok(())
}
