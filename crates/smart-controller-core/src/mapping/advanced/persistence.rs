//! Separately versioned persistence contract for advanced pose mappings.
//!
//! This extension is stored beside (not inside) canonical Smart Controller
//! schema v1. Loading validates its identity, version, controller/input
//! references and mapping topology before producing the runtime registry.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    canonical::Controller,
    error::CoreError,
    ids::{ControllerId, InputId},
    pose::Pose2D,
};

use super::{
    controller_mapper::{AdvancedMappingRegistry, AdvancedPoseMapping},
    grid::{GridBoundaryPolicy, GridMapping},
    triangulated::{MeshBoundaryPolicy, Triangle, TriangulatedMapping, TriangulatedPose},
};

/// Stable reverse-DNS identity for this optional project extension.
pub const ADVANCED_MAPPING_EXTENSION_ID: &str =
    "io.github.mujshanan.forma-deform.smart-controller.advanced-mappings";
pub const ADVANCED_MAPPING_EXTENSION_VERSION: u32 = 1;

/// Standalone extension envelope. It is deliberately not a SchemaEnvelope and
/// does not alter canonical controller schema v1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedMappingExtension {
    pub extension_id: String,
    pub version: u32,
    pub mappings: Vec<PersistedAdvancedMapping>,
}

impl AdvancedMappingExtension {
    pub fn new(mappings: Vec<PersistedAdvancedMapping>) -> Self {
        Self {
            extension_id: ADVANCED_MAPPING_EXTENSION_ID.into(),
            version: ADVANCED_MAPPING_EXTENSION_VERSION,
            mappings,
        }
    }

    pub fn validate_identity(&self) -> Result<(), CoreError> {
        if self.extension_id != ADVANCED_MAPPING_EXTENSION_ID {
            return Err(CoreError::Serialization(format!(
                "unexpected advanced mapping extension ID: {}",
                self.extension_id
            )));
        }
        if self.version != ADVANCED_MAPPING_EXTENSION_VERSION {
            return Err(CoreError::UnsupportedSchemaVersion(self.version));
        }
        Ok(())
    }

    /// Decode and validate the extension envelope. Controller references are
    /// validated separately by `into_runtime_registry`, which needs the
    /// canonical controller document.
    pub fn from_json(json: &str) -> Result<Self, CoreError> {
        let extension: Self = serde_json::from_str(json)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        extension.validate_identity()?;
        Ok(extension)
    }

    pub fn to_json(&self) -> Result<String, CoreError> {
        self.validate_identity()?;
        serde_json::to_string_pretty(self)
            .map_err(|error| CoreError::Serialization(error.to_string()))
    }

    /// Validate all references and authored mapping data, then construct the
    /// runtime registry used by AdvancedControllerMapper.
    pub fn into_runtime_registry(
        &self,
        controllers: &[Controller],
    ) -> Result<AdvancedMappingRegistry, CoreError> {
        self.validate_identity()?;

        let mut controller_map = BTreeMap::new();
        for controller in controllers {
            controller.validate_local()?;
            if controller_map.insert(controller.id.clone(), controller).is_some() {
                return Err(CoreError::InvalidModel(format!(
                    "duplicate controller ID {} in extension load context",
                    controller.id
                )));
            }
        }

        let mut registry = AdvancedMappingRegistry::new();
        for record in &self.mappings {
            let controller = controller_map.get(&record.controller_id).ok_or_else(|| {
                CoreError::InvalidMapping(format!(
                    "advanced mapping references unknown controller {}",
                    record.controller_id
                ))
            })?;

            if !controller.inputs.contains(&record.input_id) {
                return Err(CoreError::InvalidMapping(format!(
                    "advanced mapping for controller {} references undeclared input {}",
                    record.controller_id, record.input_id
                )));
            }

            let mapping = record.mapping.build()?;
            let schema_pose = match &mapping {
                AdvancedPoseMapping::Grid(mapping) => mapping.poses().first(),
                AdvancedPoseMapping::Triangulated(mapping) => mapping.vertices().first().map(|vertex| &vertex.pose),
            }.ok_or_else(|| CoreError::InvalidMapping("advanced mapping has no pose schema".into()))?;
            validate_pose_bindings(controller, schema_pose)?;
            registry.register(
                record.controller_id.clone(),
                record.input_id.clone(),
                mapping,
            )?;
        }

        Ok(registry)
    }
}

/// One durable association between a canonical controller/input and its pose field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedAdvancedMapping {
    pub controller_id: ControllerId,
    pub input_id: InputId,
    pub mapping: AdvancedMappingSpec,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdvancedMappingSpec {
    Grid {
        x_coordinates: Vec<f64>,
        y_coordinates: Vec<f64>,
        /// Row-major: y_index * x_coordinates.len() + x_index.
        poses: Vec<Pose2D>,
        boundary: PersistedGridBoundaryPolicy,
    },
    Triangulated {
        vertices: Vec<PersistedTriangulatedPose>,
        /// Each triple is an explicit index into vertices; topology is not generated.
        triangles: Vec<[usize; 3]>,
        boundary: PersistedMeshBoundaryPolicy,
    },
}

impl AdvancedMappingSpec {
    fn build(&self) -> Result<AdvancedPoseMapping, CoreError> {
        match self {
            Self::Grid { x_coordinates, y_coordinates, poses, boundary } => {
                let mapping = GridMapping::new(
                    x_coordinates.clone(),
                    y_coordinates.clone(),
                    poses.clone(),
                    (*boundary).into(),
                )?;
                Ok(AdvancedPoseMapping::Grid(mapping))
            }
            Self::Triangulated { vertices, triangles, boundary } => {
                let vertices = vertices.iter().map(|vertex| {
                    Ok(TriangulatedPose::new(
                        vertex.position,
                        vertex.pose.clone(),
                    ))
                }).collect::<Result<Vec<_>, CoreError>>()?;
                let triangles = triangles.iter()
                    .map(|[a, b, c]| Triangle::new(*a, *b, *c))
                    .collect();
                let mapping = TriangulatedMapping::new(
                    vertices,
                    triangles,
                    (*boundary).into(),
                )?;
                Ok(AdvancedPoseMapping::Triangulated(mapping))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedTriangulatedPose {
    pub position: Vector2,
    pub pose: Pose2D,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedGridBoundaryPolicy {
    Reject,
    Clamp,
}

impl From<PersistedGridBoundaryPolicy> for GridBoundaryPolicy {
    fn from(value: PersistedGridBoundaryPolicy) -> Self {
        match value {
            PersistedGridBoundaryPolicy::Reject => Self::Reject,
            PersistedGridBoundaryPolicy::Clamp => Self::Clamp,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedMeshBoundaryPolicy {
    Reject,
    ClampToMesh,
}

impl From<PersistedMeshBoundaryPolicy> for MeshBoundaryPolicy {
    fn from(value: PersistedMeshBoundaryPolicy) -> Self {
        match value {
            PersistedMeshBoundaryPolicy::Reject => Self::Reject,
            PersistedMeshBoundaryPolicy::ClampToMesh => Self::ClampToMesh,
        }
    }
}

fn validate_pose_bindings(
    controller: &Controller,
    pose: &Pose2D,
) -> Result<(), CoreError> {
    let expected: BTreeSet<_> = controller.bindings.iter().map(|binding| &binding.id).collect();
    let actual: BTreeSet<_> = pose.iter().map(|(binding_id, _)| binding_id).collect();
    if expected != actual {
        return Err(CoreError::InvalidMapping(format!(
            "persisted pose binding set does not match controller {}",
            controller.id
        )));
    }
    Ok(())
}

