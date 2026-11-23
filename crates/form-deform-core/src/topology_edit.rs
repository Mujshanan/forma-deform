//! Transactional editing for irregular cage topology and control state.
//!
//! Structural edits and control-state edits use separate revision domains.
//! Failed edits leave the cage unchanged. Allocated IDs are consumed even when
//! a batch later fails, so a failed operation cannot cause an ID to be reused.

use crate::irregular_cage::{IrregularCage, IrregularCageError};
use crate::topology::{
    ControlPointId, ControlPointPosition, ControlPointState, ControlStateRevision,
    DeformationTopologyRevision, TopologyFace, TopologyContractError,
};

/// Monotonic allocator owned and persisted by the document/topology owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlPointIdAllocator {
    next_id: Option<u64>,
}

impl ControlPointIdAllocator {
    pub fn new(first_id: u64) -> Result<Self, TopologyContractError> {
        if first_id == 0 {
            return Err(TopologyContractError::ZeroIdentity);
        }
        Ok(Self { next_id: Some(first_id) })
    }

    /// Restore allocator state from durable owner metadata.
    pub fn from_next_id(next_id: Option<u64>) -> Result<Self, TopologyContractError> {
        if next_id == Some(0) {
            return Err(TopologyContractError::ZeroIdentity);
        }
        Ok(Self { next_id })
    }

    pub fn next_id(&self) -> Option<u64> {
        self.next_id
    }

    pub fn allocate(&mut self) -> Result<ControlPointId, TopologyEditError> {
        let value = self.next_id.ok_or(TopologyEditError::ControlPointIdExhausted)?;
        let id = ControlPointId::new(value).map_err(TopologyEditError::Topology)?;
        self.next_id = value.checked_add(1);
        Ok(id)
    }
}

/// A batch of structural operations. Apply dependent operations in order:
/// remove faces before removing their points; add points before faces that use them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IrregularCageTopologyEdit {
    AddPoint { position: ControlPointPosition, pinned: bool },
    RemovePoint { id: ControlPointId },
    AddFace { face: TopologyFace },
    RemoveFace { face: TopologyFace },
    ReplaceFace { old_face: TopologyFace, new_face: TopologyFace },
}

impl IrregularCage {
    /// Apply a structural batch atomically and increment topology revision once.
    ///
    /// The allocator is deliberately not rolled back on failure: any IDs handed
    /// out during an unsuccessful batch remain consumed and cannot be reused.
    pub fn apply_topology_edits(
        &mut self,
        allocator: &mut ControlPointIdAllocator,
        edits: &[IrregularCageTopologyEdit],
    ) -> Result<Vec<ControlPointId>, TopologyEditError> {
        if edits.is_empty() {
            return Err(TopologyEditError::EmptyEditBatch);
        }

        let mut candidate = self.clone();
        let mut added_ids = Vec::new();

        for edit in edits {
            match *edit {
                IrregularCageTopologyEdit::AddPoint { position, pinned } => {
                    position.validate().map_err(TopologyEditError::Topology)?;
                    let id = allocator.allocate()?;
                    candidate.topology.control_point_ids.push(id);
                    candidate.control_state.points.push(ControlPointState {
                        id,
                        position,
                        pinned,
                    });
                    added_ids.push(id);
                }
                IrregularCageTopologyEdit::RemovePoint { id } => {
                    if candidate.topology.faces.iter()
                        .any(|face| face.control_point_ids().contains(&id))
                    {
                        return Err(TopologyEditError::PointStillReferenced(id.get()));
                    }
                    let topology_index = candidate.topology.control_point_ids.iter()
                        .position(|candidate_id| *candidate_id == id)
                        .ok_or(TopologyEditError::UnknownControlPoint(id.get()))?;
                    candidate.topology.control_point_ids.remove(topology_index);
                    let state_index = candidate.control_state.points.iter()
                        .position(|point| point.id == id)
                        .ok_or(TopologyEditError::UnknownControlPoint(id.get()))?;
                    candidate.control_state.points.remove(state_index);
                }
                IrregularCageTopologyEdit::AddFace { face } => {
                    candidate.topology.faces.push(face);
                }
                IrregularCageTopologyEdit::RemoveFace { face } => {
                    let index = unique_face_index(&candidate, face)?;
                    candidate.topology.faces.remove(index);
                }
                IrregularCageTopologyEdit::ReplaceFace { old_face, new_face } => {
                    let index = unique_face_index(&candidate, old_face)?;
                    candidate.topology.faces[index] = new_face;
                }
            }
        }

        let next_revision = candidate.topology.revision.get()
            .checked_add(1)
            .ok_or(TopologyEditError::TopologyRevisionExhausted)?;
        candidate.topology.revision = DeformationTopologyRevision::new(next_revision)
            .map_err(TopologyEditError::Topology)?;
        candidate.control_state.topology_revision = candidate.topology.revision;
        candidate.control_state.validate_against(&candidate.topology)
            .map_err(TopologyEditError::Topology)?;
        candidate.validate_geometry().map_err(TopologyEditError::InvalidCage)?;

        *self = candidate;
        Ok(added_ids)
    }

    /// Move one control point without changing topology revision.
    pub fn set_control_point_position(
        &mut self,
        id: ControlPointId,
        position: ControlPointPosition,
    ) -> Result<bool, TopologyEditError> {
        position.validate().map_err(TopologyEditError::Topology)?;
        let Some(index) = self.control_state.points.iter().position(|point| point.id == id) else {
            return Err(TopologyEditError::UnknownControlPoint(id.get()));
        };
        if self.control_state.points[index].position == position {
            return Ok(false);
        }

        let mut candidate = self.clone();
        candidate.control_state.points[index].position = position;
        candidate.increment_control_state_revision()?;
        candidate.validate_geometry().map_err(TopologyEditError::InvalidCage)?;
        *self = candidate;
        Ok(true)
    }

    /// Change pin state without changing topology revision.
    pub fn set_control_point_pinned(
        &mut self,
        id: ControlPointId,
        pinned: bool,
    ) -> Result<bool, TopologyEditError> {
        let Some(index) = self.control_state.points.iter().position(|point| point.id == id) else {
            return Err(TopologyEditError::UnknownControlPoint(id.get()));
        };
        if self.control_state.points[index].pinned == pinned {
            return Ok(false);
        }

        let mut candidate = self.clone();
        candidate.control_state.points[index].pinned = pinned;
        candidate.increment_control_state_revision()?;
        candidate.control_state.validate_against(&candidate.topology)
            .map_err(TopologyEditError::Topology)?;
        *self = candidate;
        Ok(true)
    }

    fn increment_control_state_revision(&mut self) -> Result<(), TopologyEditError> {
        let next_revision = self.control_state.revision.get()
            .checked_add(1)
            .ok_or(TopologyEditError::ControlStateRevisionExhausted)?;
        self.control_state.revision = ControlStateRevision::new(next_revision)
            .map_err(TopologyEditError::Topology)?;
        Ok(())
    }
}

fn unique_face_index(
    cage: &IrregularCage,
    face: TopologyFace,
) -> Result<usize, TopologyEditError> {
    let mut matches = cage.topology.faces.iter().enumerate()
        .filter(|(_, candidate)| **candidate == face)
        .map(|(index, _)| index);
    let index = matches.next().ok_or(TopologyEditError::FaceNotFound)?;
    if matches.next().is_some() {
        return Err(TopologyEditError::AmbiguousFace);
    }
    Ok(index)
}

#[derive(Clone, Debug, PartialEq)]
pub enum TopologyEditError {
    Topology(TopologyContractError),
    InvalidCage(IrregularCageError),
    EmptyEditBatch,
    ControlPointIdExhausted,
    TopologyRevisionExhausted,
    ControlStateRevisionExhausted,
    UnknownControlPoint(u64),
    PointStillReferenced(u64),
    FaceNotFound,
    AmbiguousFace,
}

impl std::fmt::Display for TopologyEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Topology(error) => write!(f, "topology edit failed: {error}"),
            Self::InvalidCage(error) => write!(f, "edit would produce an invalid cage: {error}"),
            Self::EmptyEditBatch => write!(f, "topology edit batch must contain at least one operation"),
            Self::ControlPointIdExhausted => write!(f, "control-point ID space is exhausted"),
            Self::TopologyRevisionExhausted => write!(f, "deformation topology revision is exhausted"),
            Self::ControlStateRevisionExhausted => write!(f, "control-state revision is exhausted"),
            Self::UnknownControlPoint(id) => write!(f, "control point {id} does not exist"),
            Self::PointStillReferenced(id) => write!(f, "control point {id} is still referenced by a face; remove or replace its faces first"),
            Self::FaceNotFound => write!(f, "the requested face does not exist"),
            Self::AmbiguousFace => write!(f, "the requested face is ambiguous; duplicate face records are not supported for removal or replacement"),
        }
    }
}
impl std::error::Error for TopologyEditError {}
