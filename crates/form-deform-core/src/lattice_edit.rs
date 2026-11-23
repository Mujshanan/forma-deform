//! Row/column editing for regular lattices and a unified cage-editing facade.
//!
//! Row and column insertion split existing grid intervals at their midpoint.
//! Removed rows/columns are deleted structurally; all surviving control-point
//! identities are preserved. Structural batches are atomic and consume allocated
//! IDs even when validation fails.

use crate::irregular_cage::IrregularCage;
use crate::regular_lattice::{RegularLattice, RegularLatticeError, MAX_LATTICE_CONTROL_POINTS};
use crate::topology::{
    ControlPointId, ControlPointPosition, ControlPointState, ControlStateRevision,
    DeformationTopologyRevision, TopologyView,
};
use crate::topology_edit::{
    ControlPointIdAllocator, IrregularCageTopologyEdit, TopologyEditError,
};

/// A row is inserted between this row and the following row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegularLatticeEdit {
    InsertRow { after_row: u32 },
    RemoveRow { row: u32 },
    InsertColumn { after_column: u32 },
    RemoveColumn { column: u32 },
}

impl RegularLattice {
    /// Apply a structural batch atomically. Indices in each operation refer to
    /// the candidate lattice as modified by earlier operations in the batch.
    pub fn apply_lattice_edits(
        &mut self,
        allocator: &mut ControlPointIdAllocator,
        edits: &[RegularLatticeEdit],
    ) -> Result<Vec<ControlPointId>, RegularLatticeEditError> {
        if edits.is_empty() {
            return Err(RegularLatticeEditError::EmptyEditBatch);
        }

        let mut candidate = self.clone();
        let mut added_ids = Vec::new();

        for edit in edits {
            match *edit {
                RegularLatticeEdit::InsertRow { after_row } => {
                    if after_row >= candidate.topology.rows.saturating_sub(1) {
                        return Err(RegularLatticeEditError::InvalidInsertionIndex);
                    }
                    let old_rows = candidate.topology.rows as usize;
                    let columns = candidate.topology.columns as usize;
                    let insert_at = after_row as usize + 1;
                    let positions = candidate.positions_by_id()?;
                    let mut new_ids = Vec::with_capacity(columns);
                    let mut new_points = Vec::with_capacity(columns);
                    for column in 0..columns {
                        let upper_id = candidate.topology.control_point_ids[
                            after_row as usize * columns + column
                        ];
                        let lower_id = candidate.topology.control_point_ids[
                            (after_row as usize + 1) * columns + column
                        ];
                        let upper = positions[&upper_id];
                        let lower = positions[&lower_id];
                        let id = allocator.allocate()?;
                        let position = midpoint(upper, lower)?;
                        new_ids.push(id);
                        new_points.push(ControlPointState { id, position, pinned: false });
                        added_ids.push(id);
                    }
                    candidate.topology.control_point_ids.splice(insert_at * columns..insert_at * columns, new_ids);
                    candidate.control_state.points.extend(new_points);
                    candidate.topology.rows = u32::try_from(old_rows + 1)
                        .map_err(|_| RegularLatticeEditError::DimensionOverflow)?;
                }
                RegularLatticeEdit::RemoveRow { row } => {
                    if candidate.topology.rows <= 2 || row >= candidate.topology.rows {
                        return Err(RegularLatticeEditError::InvalidRemoval);
                    }
                    let columns = candidate.topology.columns as usize;
                    let start = row as usize * columns;
                    let removed: Vec<_> = candidate.topology.control_point_ids
                        .drain(start..start + columns).collect();
                    candidate.control_state.points.retain(|point| !removed.contains(&point.id));
                    candidate.topology.rows -= 1;
                }
                RegularLatticeEdit::InsertColumn { after_column } => {
                    if after_column >= candidate.topology.columns.saturating_sub(1) {
                        return Err(RegularLatticeEditError::InvalidInsertionIndex);
                    }
                    let rows = candidate.topology.rows as usize;
                    let columns = candidate.topology.columns as usize;
                    let positions = candidate.positions_by_id()?;
                    let mut inserted = Vec::with_capacity(rows);
                    let mut new_points = Vec::with_capacity(rows);
                    for row in 0..rows {
                        let left_id = candidate.topology.control_point_ids[row * columns + after_column as usize];
                        let right_id = candidate.topology.control_point_ids[row * columns + after_column as usize + 1];
                        let left = positions[&left_id];
                        let right = positions[&right_id];
                        let id = allocator.allocate()?;
                        let position = midpoint(left, right)?;
                        inserted.push(id);
                        new_points.push(ControlPointState { id, position, pinned: false });
                        added_ids.push(id);
                    }
                    let mut ids = Vec::with_capacity(rows * (columns + 1));
                    for row in 0..rows {
                        let start = row * columns;
                        ids.extend_from_slice(&candidate.topology.control_point_ids[start..start + after_column as usize + 1]);
                        ids.push(inserted[row]);
                        ids.extend_from_slice(&candidate.topology.control_point_ids[start + after_column as usize + 1..start + columns]);
                    }
                    candidate.topology.control_point_ids = ids;
                    candidate.control_state.points.extend(new_points);
                    candidate.topology.columns = u32::try_from(columns + 1)
                        .map_err(|_| RegularLatticeEditError::DimensionOverflow)?;
                }
                RegularLatticeEdit::RemoveColumn { column } => {
                    if candidate.topology.columns <= 2 || column >= candidate.topology.columns {
                        return Err(RegularLatticeEditError::InvalidRemoval);
                    }
                    let rows = candidate.topology.rows as usize;
                    let columns = candidate.topology.columns as usize;
                    let mut removed = Vec::with_capacity(rows);
                    let mut ids = Vec::with_capacity(rows * (columns - 1));
                    for row in 0..rows {
                        for col in 0..columns {
                            let id = candidate.topology.control_point_ids[row * columns + col];
                            if col == column as usize { removed.push(id); } else { ids.push(id); }
                        }
                    }
                    candidate.topology.control_point_ids = ids;
                    candidate.control_state.points.retain(|point| !removed.contains(&point.id));
                    candidate.topology.columns -= 1;
                }
            }
            let count = (candidate.topology.rows as usize)
                .checked_mul(candidate.topology.columns as usize)
                .ok_or(RegularLatticeEditError::PointCountOverflow)?;
            if count > MAX_LATTICE_CONTROL_POINTS {
                return Err(RegularLatticeEditError::TooManyPoints(count));
            }
        }

        let next_revision = candidate.topology.revision.get().checked_add(1)
            .ok_or(RegularLatticeEditError::TopologyRevisionExhausted)?;
        candidate.topology.revision = DeformationTopologyRevision::new(next_revision)
            .map_err(RegularLatticeEditError::Topology)?;
        candidate.control_state.topology_revision = candidate.topology.revision;
        candidate.topology.validate_structure().map_err(RegularLatticeEditError::Topology)?;
        candidate.control_state.validate_against(&candidate.topology)
            .map_err(RegularLatticeEditError::Topology)?;
        candidate.validate_geometry().map_err(RegularLatticeEditError::InvalidLattice)?;

        *self = candidate;
        Ok(added_ids)
    }

    /// Return the current point positions keyed by stable ID.
    fn positions_by_id(&self) -> Result<std::collections::HashMap<ControlPointId, ControlPointPosition>, RegularLatticeEditError> {
        self.control_state.points.iter().map(|point| {
            point.position.validate().map_err(RegularLatticeEditError::Topology)?;
            Ok((point.id, point.position))
        }).collect()
    }

    pub fn set_lattice_control_point_position(
        &mut self,
        id: ControlPointId,
        position: ControlPointPosition,
    ) -> Result<bool, RegularLatticeEditError> {
        position.validate().map_err(RegularLatticeEditError::Topology)?;
        let index = self.control_state.points.iter().position(|point| point.id == id)
            .ok_or(RegularLatticeEditError::UnknownControlPoint(id.get()))?;
        if self.control_state.points[index].position == position { return Ok(false); }
        let mut candidate = self.clone();
        candidate.control_state.points[index].position = position;
        candidate.increment_control_revision()?;
        candidate.validate_geometry().map_err(RegularLatticeEditError::InvalidLattice)?;
        *self = candidate;
        Ok(true)
    }

    pub fn set_lattice_control_point_pinned(
        &mut self,
        id: ControlPointId,
        pinned: bool,
    ) -> Result<bool, RegularLatticeEditError> {
        let index = self.control_state.points.iter().position(|point| point.id == id)
            .ok_or(RegularLatticeEditError::UnknownControlPoint(id.get()))?;
        if self.control_state.points[index].pinned == pinned { return Ok(false); }
        let mut candidate = self.clone();
        candidate.control_state.points[index].pinned = pinned;
        candidate.increment_control_revision()?;
        candidate.control_state.validate_against(&candidate.topology)
            .map_err(RegularLatticeEditError::Topology)?;
        *self = candidate;
        Ok(true)
    }

    fn increment_control_revision(&mut self) -> Result<(), RegularLatticeEditError> {
        let next = self.control_state.revision.get().checked_add(1)
            .ok_or(RegularLatticeEditError::ControlStateRevisionExhausted)?;
        self.control_state.revision = ControlStateRevision::new(next)
            .map_err(RegularLatticeEditError::Topology)?;
        Ok(())
    }
}

fn midpoint(a: ControlPointPosition, b: ControlPointPosition) -> Result<ControlPointPosition, RegularLatticeEditError> {
    ControlPointPosition::new(a.x * 0.5 + b.x * 0.5, a.y * 0.5 + b.y * 0.5)
        .map_err(RegularLatticeEditError::Topology)
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegularLatticeEditError {
    Topology(crate::topology::TopologyContractError),
    InvalidLattice(RegularLatticeError),
    EmptyEditBatch,
    InvalidInsertionIndex,
    InvalidRemoval,
    DimensionOverflow,
    PointCountOverflow,
    TooManyPoints(usize),
    TopologyRevisionExhausted,
    ControlStateRevisionExhausted,
    UnknownControlPoint(u64),
    ControlPointIdExhausted,
}
impl From<TopologyEditError> for RegularLatticeEditError {
    fn from(value: TopologyEditError) -> Self {
        match value {
            TopologyEditError::Topology(error) => Self::Topology(error),
            TopologyEditError::ControlPointIdExhausted => Self::ControlPointIdExhausted,
            TopologyEditError::TopologyRevisionExhausted => Self::TopologyRevisionExhausted,
            TopologyEditError::ControlStateRevisionExhausted => Self::ControlStateRevisionExhausted,
            TopologyEditError::UnknownControlPoint(id) => Self::UnknownControlPoint(id),
            _ => Self::EmptyEditBatch,
        }
    }
}
impl std::fmt::Display for RegularLatticeEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Topology(error) => write!(f, "lattice edit topology error: {error}"),
            Self::InvalidLattice(error) => write!(f, "edit would produce an invalid lattice: {error}"),
            Self::EmptyEditBatch => write!(f, "lattice edit batch must not be empty"),
            Self::InvalidInsertionIndex => write!(f, "insertion index must identify an existing interval"),
            Self::InvalidRemoval => write!(f, "removal must identify an existing row or column and leave at least two"),
            Self::DimensionOverflow => write!(f, "lattice dimensions exceed u32"),
            Self::PointCountOverflow => write!(f, "lattice point count overflows addressable size"),
            Self::TooManyPoints(count) => write!(f, "lattice has {count} points; maximum is {MAX_LATTICE_CONTROL_POINTS}"),
            Self::TopologyRevisionExhausted => write!(f, "topology revision is exhausted"),
            Self::ControlStateRevisionExhausted => write!(f, "control-state revision is exhausted"),
            Self::UnknownControlPoint(id) => write!(f, "control point {id} does not exist"),
            Self::ControlPointIdExhausted => write!(f, "control-point ID allocator is exhausted"),
        }
    }
}
impl std::error::Error for RegularLatticeEditError {}

/// One editing facade for either cage representation. An edit batch must match
/// the cage variant; no implicit conversion or topology reinterpretation occurs.
#[derive(Clone, Debug, PartialEq)]
pub enum EditableDeformationCage {
    RegularLattice(RegularLattice),
    IrregularCage(IrregularCage),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeformationCageEdit {
    RegularLattice(RegularLatticeEdit),
    IrregularCage(IrregularCageTopologyEdit),
}

impl EditableDeformationCage {
    pub fn topology(&self) -> &dyn TopologyView {
        match self {
            Self::RegularLattice(cage) => &cage.topology,
            Self::IrregularCage(cage) => &cage.topology,
        }
    }

    pub fn control_state(&self) -> &crate::topology::DeformationControlState {
        match self {
            Self::RegularLattice(cage) => &cage.control_state,
            Self::IrregularCage(cage) => &cage.control_state,
        }
    }

    pub fn apply_edits(
        &mut self,
        allocator: &mut ControlPointIdAllocator,
        edits: &[DeformationCageEdit],
    ) -> Result<Vec<ControlPointId>, UnifiedTopologyEditError> {
        if edits.is_empty() { return Err(UnifiedTopologyEditError::EmptyEditBatch); }
        match self {
            Self::RegularLattice(cage) => {
                let mut regular = Vec::with_capacity(edits.len());
                for edit in edits {
                    match edit {
                        DeformationCageEdit::RegularLattice(value) => regular.push(*value),
                        DeformationCageEdit::IrregularCage(_) => return Err(UnifiedTopologyEditError::WrongEditKind),
                    }
                }
                cage.apply_lattice_edits(allocator, &regular)
                    .map_err(UnifiedTopologyEditError::RegularLattice)
            }
            Self::IrregularCage(cage) => {
                let mut irregular = Vec::with_capacity(edits.len());
                for edit in edits {
                    match edit {
                        DeformationCageEdit::IrregularCage(value) => irregular.push(*value),
                        DeformationCageEdit::RegularLattice(_) => return Err(UnifiedTopologyEditError::WrongEditKind),
                    }
                }
                cage.apply_topology_edits(allocator, &irregular)
                    .map_err(UnifiedTopologyEditError::IrregularCage)
            }
        }
    }

    pub fn set_control_point_position(
        &mut self,
        id: ControlPointId,
        position: ControlPointPosition,
    ) -> Result<bool, UnifiedTopologyEditError> {
        match self {
            Self::RegularLattice(cage) => cage.set_lattice_control_point_position(id, position)
                .map_err(UnifiedTopologyEditError::RegularLattice),
            Self::IrregularCage(cage) => cage.set_control_point_position(id, position)
                .map_err(UnifiedTopologyEditError::IrregularCage),
        }
    }

    pub fn set_control_point_pinned(
        &mut self,
        id: ControlPointId,
        pinned: bool,
    ) -> Result<bool, UnifiedTopologyEditError> {
        match self {
            Self::RegularLattice(cage) => cage.set_lattice_control_point_pinned(id, pinned)
                .map_err(UnifiedTopologyEditError::RegularLattice),
            Self::IrregularCage(cage) => cage.set_control_point_pinned(id, pinned)
                .map_err(UnifiedTopologyEditError::IrregularCage),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnifiedTopologyEditError {
    EmptyEditBatch,
    WrongEditKind,
    RegularLattice(RegularLatticeEditError),
    IrregularCage(TopologyEditError),
}
impl std::fmt::Display for UnifiedTopologyEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyEditBatch => write!(f, "deformation cage edit batch must not be empty"),
            Self::WrongEditKind => write!(f, "edit kind does not match the active cage topology"),
            Self::RegularLattice(error) => write!(f, "regular lattice edit failed: {error}"),
            Self::IrregularCage(error) => write!(f, "irregular cage edit failed: {error}"),
        }
    }
}
impl std::error::Error for UnifiedTopologyEditError {}
