//! Stable identities and typed topology contracts for FDM-authored deformation cages.
//!
//! This module deliberately separates topology identity/revision from mutable
//! control-point state. It defines structural contracts only; geometric
//! validity (self-intersection, winding, area, connectedness, and tolerances)
//! is validated by the regular-lattice and irregular-cage implementation steps.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::num::NonZeroU64;

/// Stable FDM-owned identity. Never use a vector index as a control-point ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ControlPointId(NonZeroU64);

impl ControlPointId {
    pub fn new(value: u64) -> Result<Self, TopologyContractError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(TopologyContractError::ZeroIdentity)
    }

    pub const fn get(self) -> u64 { self.0.get() }
}

/// Stable identity for one authored deformation topology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeformationTopologyId(NonZeroU64);

impl DeformationTopologyId {
    pub fn new(value: u64) -> Result<Self, TopologyContractError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(TopologyContractError::ZeroIdentity)
    }

    pub const fn get(self) -> u64 { self.0.get() }
}

/// Revision of structural topology only: membership, dimensions, or connectivity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeformationTopologyRevision(NonZeroU64);

impl DeformationTopologyRevision {
    pub fn new(value: u64) -> Result<Self, TopologyContractError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(TopologyContractError::ZeroRevision)
    }

    pub const fn get(self) -> u64 { self.0.get() }
}

/// Revision of mutable control state only: positions and pin state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ControlStateRevision(NonZeroU64);

impl ControlStateRevision {
    pub fn new(value: u64) -> Result<Self, TopologyContractError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(TopologyContractError::ZeroRevision)
    }

    pub const fn get(self) -> u64 { self.0.get() }
}

/// FDM-local point coordinates. These are not host Path2D point identities.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPointPosition {
    pub x: f64,
    pub y: f64,
}

impl ControlPointPosition {
    pub fn new(x: f64, y: f64) -> Result<Self, TopologyContractError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(TopologyContractError::NonFinitePosition);
        }
        Ok(Self { x, y })
    }

    pub fn validate(&self) -> Result<(), TopologyContractError> {
        if self.x.is_finite() && self.y.is_finite() {
            Ok(())
        } else {
            Err(TopologyContractError::NonFinitePosition)
        }
    }
}

/// One point's mutable authored state. Identity is stable across movement.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPointState {
    pub id: ControlPointId,
    pub position: ControlPointPosition,
    pub pinned: bool,
}

impl ControlPointState {
    pub fn validate(&self) -> Result<(), TopologyContractError> {
        self.position.validate()
    }
}

/// A face has three or four vertices. Vertex order is authored and significant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyFace {
    Triangle([ControlPointId; 3]),
    Quad([ControlPointId; 4]),
}

impl TopologyFace {
    pub fn control_point_ids(&self) -> &[ControlPointId] {
        match self {
            Self::Triangle(ids) => ids,
            Self::Quad(ids) => ids,
        }
    }
}

/// Common read-only topology interface. Faces are returned in deterministic order.
pub trait TopologyView {
    fn topology_id(&self) -> DeformationTopologyId;
    fn topology_revision(&self) -> DeformationTopologyRevision;
    fn control_point_ids(&self) -> &[ControlPointId];
    fn face_count(&self) -> usize;
    fn face_at(&self, index: usize) -> Option<TopologyFace>;
}

/// Rectangular lattice. Connectivity is implicit; each cell is exposed as a quad.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegularLatticeTopology {
    pub id: DeformationTopologyId,
    pub revision: DeformationTopologyRevision,
    pub rows: u32,
    pub columns: u32,
    /// Row-major stable IDs. Reordering storage does not redefine identity.
    pub control_point_ids: Vec<ControlPointId>,
}

impl RegularLatticeTopology {
    pub fn validate_structure(&self) -> Result<(), TopologyContractError> {
        if self.rows < 2 || self.columns < 2 {
            return Err(TopologyContractError::InvalidLatticeDimensions);
        }
        let expected = (self.rows as usize)
            .checked_mul(self.columns as usize)
            .ok_or(TopologyContractError::PointCountOverflow)?;
        if self.control_point_ids.len() != expected {
            return Err(TopologyContractError::ControlPointCountMismatch {
                expected,
                actual: self.control_point_ids.len(),
            });
        }
        validate_unique_ids(&self.control_point_ids)
    }
}

impl TopologyView for RegularLatticeTopology {
    fn topology_id(&self) -> DeformationTopologyId { self.id }
    fn topology_revision(&self) -> DeformationTopologyRevision { self.revision }
    fn control_point_ids(&self) -> &[ControlPointId] { &self.control_point_ids }

    fn face_count(&self) -> usize {
        (self.rows.saturating_sub(1) as usize)
            .saturating_mul(self.columns.saturating_sub(1) as usize)
    }

    fn face_at(&self, index: usize) -> Option<TopologyFace> {
        if index >= self.face_count() { return None; }
        let cell_columns = (self.columns - 1) as usize;
        let row = index / cell_columns;
        let column = index % cell_columns;
        let top_left = row * self.columns as usize + column;
        let top_right = top_left + 1;
        let bottom_left = top_left + self.columns as usize;
        let bottom_right = bottom_left + 1;
        Some(TopologyFace::Quad([
            self.control_point_ids[top_left],
            self.control_point_ids[top_right],
            self.control_point_ids[bottom_right],
            self.control_point_ids[bottom_left],
        ]))
    }
}

/// Irregular cage with explicit, authored face connectivity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IrregularCageTopology {
    pub id: DeformationTopologyId,
    pub revision: DeformationTopologyRevision,
    pub control_point_ids: Vec<ControlPointId>,
    pub faces: Vec<TopologyFace>,
}

impl IrregularCageTopology {
    pub fn validate_structure(&self) -> Result<(), TopologyContractError> {
        if self.control_point_ids.len() < 3 || self.faces.is_empty() {
            return Err(TopologyContractError::EmptyCage);
        }
        validate_unique_ids(&self.control_point_ids)?;
        let known_ids: HashSet<_> = self.control_point_ids.iter().copied().collect();
        for face in &self.faces {
            let ids = face.control_point_ids();
            let mut local = ids.to_vec();
            local.sort_unstable();
            local.dedup();
            if local.len() != ids.len() {
                return Err(TopologyContractError::RepeatedVertexInFace);
            }
            for id in ids {
                if !known_ids.contains(id) {
                    return Err(TopologyContractError::FaceReferencesUnknownPoint);
                }
            }
        }
        Ok(())
    }
}

impl TopologyView for IrregularCageTopology {
    fn topology_id(&self) -> DeformationTopologyId { self.id }
    fn topology_revision(&self) -> DeformationTopologyRevision { self.revision }
    fn control_point_ids(&self) -> &[ControlPointId] { &self.control_point_ids }
    fn face_count(&self) -> usize { self.faces.len() }
    fn face_at(&self, index: usize) -> Option<TopologyFace> { self.faces.get(index).copied() }
}

/// Strongly typed topology alternatives; no implicit conversion between them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeformationTopology {
    RegularLattice(RegularLatticeTopology),
    IrregularCage(IrregularCageTopology),
}

impl TopologyView for DeformationTopology {
    fn topology_id(&self) -> DeformationTopologyId {
        match self {
            Self::RegularLattice(value) => value.topology_id(),
            Self::IrregularCage(value) => value.topology_id(),
        }
    }
    fn topology_revision(&self) -> DeformationTopologyRevision {
        match self {
            Self::RegularLattice(value) => value.topology_revision(),
            Self::IrregularCage(value) => value.topology_revision(),
        }
    }
    fn control_point_ids(&self) -> &[ControlPointId] {
        match self {
            Self::RegularLattice(value) => value.control_point_ids(),
            Self::IrregularCage(value) => value.control_point_ids(),
        }
    }
    fn face_count(&self) -> usize {
        match self {
            Self::RegularLattice(value) => value.face_count(),
            Self::IrregularCage(value) => value.face_count(),
        }
    }
    fn face_at(&self, index: usize) -> Option<TopologyFace> {
        match self {
            Self::RegularLattice(value) => value.face_at(index),
            Self::IrregularCage(value) => value.face_at(index),
        }
    }
}

/// Mutable control state is revisioned independently of topology.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeformationControlState {
    pub topology_id: DeformationTopologyId,
    pub topology_revision: DeformationTopologyRevision,
    pub revision: ControlStateRevision,
    /// Values are keyed by stable ID semantically; vector order is not identity.
    pub points: Vec<ControlPointState>,
}

impl DeformationControlState {
    pub fn validate_against(
        &self,
        topology: &impl TopologyView,
    ) -> Result<(), TopologyContractError> {
        if self.topology_id != topology.topology_id()
            || self.topology_revision != topology.topology_revision()
        {
            return Err(TopologyContractError::TopologyReferenceMismatch);
        }
        let expected = topology.control_point_ids();
        if self.points.len() != expected.len() {
            return Err(TopologyContractError::ControlStatePointSetMismatch);
        }
        let mut actual: Vec<_> = self.points.iter().map(|point| point.id).collect();
        actual.sort_unstable();
        let mut expected_sorted = expected.to_vec();
        expected_sorted.sort_unstable();
        if actual != expected_sorted {
            return Err(TopologyContractError::ControlStatePointSetMismatch);
        }
        for point in &self.points {
            point.validate()?;
        }
        Ok(())
    }
}

fn validate_unique_ids(ids: &[ControlPointId]) -> Result<(), TopologyContractError> {
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != ids.len() {
        return Err(TopologyContractError::DuplicateControlPointId);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TopologyContractError {
    ZeroIdentity,
    ZeroRevision,
    NonFinitePosition,
    InvalidLatticeDimensions,
    PointCountOverflow,
    ControlPointCountMismatch { expected: usize, actual: usize },
    DuplicateControlPointId,
    EmptyCage,
    RepeatedVertexInFace,
    FaceReferencesUnknownPoint,
    TopologyReferenceMismatch,
    ControlStatePointSetMismatch,
}

impl std::fmt::Display for TopologyContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroIdentity => write!(f, "stable identities must be non-zero"),
            Self::ZeroRevision => write!(f, "topology and control-state revisions must be non-zero"),
            Self::NonFinitePosition => write!(f, "control-point coordinates must be finite"),
            Self::InvalidLatticeDimensions => write!(f, "regular lattice requires at least two rows and columns"),
            Self::PointCountOverflow => write!(f, "regular lattice point count overflows addressable size"),
            Self::ControlPointCountMismatch { expected, actual } => write!(f, "lattice expects {expected} control points, received {actual}"),
            Self::DuplicateControlPointId => write!(f, "control-point identities must be unique within a topology"),
            Self::EmptyCage => write!(f, "irregular cage requires control points and faces"),
            Self::RepeatedVertexInFace => write!(f, "a face must not repeat a control-point identity"),
            Self::FaceReferencesUnknownPoint => write!(f, "a face references a control point outside the cage"),
            Self::TopologyReferenceMismatch => write!(f, "control state references a different topology identity or revision"),
            Self::ControlStatePointSetMismatch => write!(f, "control state must contain exactly one state for every topology control point"),
        }
    }
}

impl std::error::Error for TopologyContractError {}
