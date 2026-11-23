//! Deterministic regular-lattice construction and geometric validation.

use crate::topology::{
    ControlPointId, ControlPointPosition, ControlPointState, ControlStateRevision,
    DeformationControlState, DeformationTopologyId, DeformationTopologyRevision,
    RegularLatticeTopology, TopologyContractError, TopologyFace, TopologyView,
};
use std::collections::HashMap;

pub const MAX_LATTICE_CONTROL_POINTS: usize = 1_000_000;
pub const MAX_LATTICE_BOUNDARY_POINTS: usize = 4_096;
const MIN_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug, PartialEq)]
pub struct RegularLattice {
    pub topology: RegularLatticeTopology,
    pub control_state: DeformationControlState,
}

impl RegularLattice {
    /// Create an axis-aligned lattice. IDs are allocated in row-major order;
    /// the caller must reserve this ID range and never reuse deleted IDs.
    pub fn rectangular(
        topology_id: DeformationTopologyId,
        topology_revision: DeformationTopologyRevision,
        control_state_revision: ControlStateRevision,
        rows: u32,
        columns: u32,
        min: ControlPointPosition,
        max: ControlPointPosition,
        first_control_point_id: u64,
    ) -> Result<Self, RegularLatticeError> {
        min.validate().map_err(RegularLatticeError::Topology)?;
        max.validate().map_err(RegularLatticeError::Topology)?;
        let count = checked_count(rows, columns)?;
        let width = max.x - min.x;
        let height = max.y - min.y;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(RegularLatticeError::DegenerateBounds);
        }
        if first_control_point_id == 0 {
            return Err(RegularLatticeError::InvalidFirstId);
        }
        first_control_point_id.checked_add((count - 1) as u64)
            .ok_or(RegularLatticeError::IdOverflow)?;

        let mut ids = Vec::with_capacity(count);
        let mut points = Vec::with_capacity(count);
        for row in 0..rows {
            let ty = row as f64 / (rows - 1) as f64;
            for column in 0..columns {
                let tx = column as f64 / (columns - 1) as f64;
                let raw_id = first_control_point_id.checked_add(ids.len() as u64)
                    .ok_or(RegularLatticeError::IdOverflow)?;
                let id = ControlPointId::new(raw_id).map_err(RegularLatticeError::Topology)?;
                let position = ControlPointPosition::new(
                    min.x + width * tx,
                    min.y + height * ty,
                ).map_err(RegularLatticeError::Topology)?;
                ids.push(id);
                points.push(ControlPointState { id, position, pinned: false });
            }
        }

        let topology = RegularLatticeTopology {
            id: topology_id, revision: topology_revision, rows, columns,
            control_point_ids: ids,
        };
        topology.validate_structure().map_err(RegularLatticeError::Topology)?;
        let control_state = DeformationControlState {
            topology_id, topology_revision, revision: control_state_revision, points,
        };
        control_state.validate_against(&topology).map_err(RegularLatticeError::Topology)?;
        let lattice = Self { topology, control_state };
        lattice.validate_geometry()?;
        Ok(lattice)
    }

    /// Validates finite positions, unique/non-coincident points, non-degenerate
    /// edges/cells, consistent cell orientation, convex cells, and a simple boundary.
    pub fn validate_geometry(&self) -> Result<(), RegularLatticeError> {
        self.topology.validate_structure().map_err(RegularLatticeError::Topology)?;
        self.control_state.validate_against(&self.topology).map_err(RegularLatticeError::Topology)?;
        let lookup: HashMap<_, _> = self.control_state.points.iter()
            .map(|p| (p.id, p.position)).collect();
        let ordered: Vec<_> = self.topology.control_point_ids.iter()
            .map(|id| lookup.get(id).copied().ok_or(RegularLatticeError::MissingPoint(id.get())))
            .collect::<Result<_, _>>()?;
        let (min_x, min_y, max_x, max_y) = bounds(&ordered)?;
        let diagonal = (max_x - min_x).hypot(max_y - min_y);
        if !diagonal.is_finite() || diagonal <= 0.0 {
            return Err(RegularLatticeError::DegenerateBounds);
        }
        let tol = (diagonal * 1.0e-9).max(MIN_TOLERANCE);
        reject_coincident_points(&ordered, tol)?;

        let mut expected_sign = 0.0_f64;
        for index in 0..self.topology.face_count() {
            let ids = match self.topology.face_at(index) {
                Some(TopologyFace::Quad(ids)) => ids,
                Some(TopologyFace::Triangle(_)) => return Err(RegularLatticeError::UnexpectedTriangle),
                None => return Err(RegularLatticeError::InvalidFace),
            };
            let q = [
                lookup.get(&ids[0]).copied().ok_or(RegularLatticeError::MissingPoint(ids[0].get()))?,
                lookup.get(&ids[1]).copied().ok_or(RegularLatticeError::MissingPoint(ids[1].get()))?,
                lookup.get(&ids[2]).copied().ok_or(RegularLatticeError::MissingPoint(ids[2].get()))?,
                lookup.get(&ids[3]).copied().ok_or(RegularLatticeError::MissingPoint(ids[3].get()))?,
            ];
            for edge in 0..4 {
                if distance(q[edge], q[(edge + 1) % 4]) <= tol {
                    return Err(RegularLatticeError::ZeroLengthCellEdge(index));
                }
            }
            let area2 = area2(&q);
            if !area2.is_finite() || area2.abs() <= tol * tol {
                return Err(RegularLatticeError::DegenerateCell(index));
            }
            let sign = area2.signum();
            if expected_sign == 0.0 { expected_sign = sign; }
            else if sign != expected_sign { return Err(RegularLatticeError::InvertedCell(index)); }
            let mut corner_sign = 0.0;
            for k in 0..4 {
                let cross = cross(q[k], q[(k + 1) % 4], q[(k + 2) % 4]);
                if !cross.is_finite() || cross.abs() <= tol * tol {
                    return Err(RegularLatticeError::DegenerateCell(index));
                }
                if corner_sign == 0.0 { corner_sign = cross.signum(); }
                else if cross.signum() != corner_sign {
                    return Err(RegularLatticeError::NonConvexCell(index));
                }
            }
        }
        let boundary = self.boundary_positions_from_lookup(&lookup)?;
        validate_boundary(&boundary, tol)?;
        Ok(())
    }

    pub fn boundary_positions(&self) -> Result<Vec<ControlPointPosition>, RegularLatticeError> {
        let lookup: HashMap<_, _> = self.control_state.points.iter()
            .map(|p| (p.id, p.position)).collect();
        self.boundary_positions_from_lookup(&lookup)
    }

    fn boundary_positions_from_lookup(
        &self, lookup: &HashMap<ControlPointId, ControlPointPosition>,
    ) -> Result<Vec<ControlPointPosition>, RegularLatticeError> {
        let rows = self.topology.rows as usize;
        let cols = self.topology.columns as usize;
        let at = |r: usize, c: usize| -> Result<ControlPointPosition, RegularLatticeError> {
            let id = self.topology.control_point_ids.get(r * cols + c)
                .ok_or(RegularLatticeError::InvalidBoundary)?;
            lookup.get(id).copied().ok_or(RegularLatticeError::MissingPoint(id.get()))
        };
        let boundary_count = 2 * rows + 2 * cols - 4;
        if boundary_count > MAX_LATTICE_BOUNDARY_POINTS {
            return Err(RegularLatticeError::BoundaryTooLarge(boundary_count));
        }
        let mut out = Vec::with_capacity(boundary_count);
        for c in 0..cols { out.push(at(0, c)?); }
        for r in 1..rows { out.push(at(r, cols - 1)?); }
        for c in (0..cols - 1).rev() { out.push(at(rows - 1, c)?); }
        for r in (1..rows - 1).rev() { out.push(at(r, 0)?); }
        Ok(out)
    }
}

fn checked_count(rows: u32, cols: u32) -> Result<usize, RegularLatticeError> {
    if rows < 2 || cols < 2 { return Err(RegularLatticeError::Topology(TopologyContractError::InvalidLatticeDimensions)); }
    let count = (rows as usize).checked_mul(cols as usize)
        .ok_or(RegularLatticeError::Topology(TopologyContractError::PointCountOverflow))?;
    if count > MAX_LATTICE_CONTROL_POINTS {
        return Err(RegularLatticeError::TooManyPoints(count));
    }
    Ok(count)
}

fn reject_coincident_points(points: &[ControlPointPosition], tol: f64) -> Result<(), RegularLatticeError> {
    // Spatial hashing keeps duplicate detection near-linear for dense lattices.
    let mut buckets: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (index, point) in points.iter().enumerate() {
        let qx = (point.x / tol).floor();
        let qy = (point.y / tol).floor();
        if !qx.is_finite() || !qy.is_finite()
            || qx.abs() > 9.0e18 || qy.abs() > 9.0e18 {
            return Err(RegularLatticeError::CoordinateRangeExceeded);
        }
        let cell = (qx as i64, qy as i64);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(candidates) = buckets.get(&(cell.0 + dx, cell.1 + dy)) {
                    for previous in candidates {
                        if distance(*point, points[*previous]) <= tol {
                            return Err(RegularLatticeError::CoincidentPoints(*previous, index));
                        }
                    }
                }
            }
        }
        buckets.entry(cell).or_default().push(index);
    }
    Ok(())
}

fn bounds(points: &[ControlPointPosition]) -> Result<(f64, f64, f64, f64), RegularLatticeError> {
    let p = points.first().ok_or(RegularLatticeError::Empty)?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (p.x, p.y, p.x, p.y);
    for p in points.iter().skip(1) {
        min_x = min_x.min(p.x); min_y = min_y.min(p.y);
        max_x = max_x.max(p.x); max_y = max_y.max(p.y);
    }
    Ok((min_x, min_y, max_x, max_y))
}
fn distance(a: ControlPointPosition, b: ControlPointPosition) -> f64 { (b.x-a.x).hypot(b.y-a.y) }
fn cross(a: ControlPointPosition, b: ControlPointPosition, c: ControlPointPosition) -> f64 {
    (b.x-a.x)*(c.y-a.y) - (b.y-a.y)*(c.x-a.x)
}
fn area2(p: &[ControlPointPosition; 4]) -> f64 { polygon_area2(p) }
fn polygon_area2(p: &[ControlPointPosition]) -> f64 {
    (0..p.len()).map(|i| p[i].x*p[(i+1)%p.len()].y - p[(i+1)%p.len()].x*p[i].y).sum()
}
fn validate_boundary(p: &[ControlPointPosition], tol: f64) -> Result<(), RegularLatticeError> {
    if p.len() < 4 { return Err(RegularLatticeError::InvalidBoundary); }
    for i in 0..p.len() {
        let a = p[i]; let b = p[(i+1)%p.len()];
        if distance(a,b) <= tol { return Err(RegularLatticeError::ZeroLengthBoundaryEdge(i)); }
        for j in (i+1)..p.len() {
            if j == (i+1)%p.len() || i == (j+1)%p.len() { continue; }
            let c = p[j]; let d = p[(j+1)%p.len()];
            if segments_intersect(a,b,c,d,tol) {
                return Err(RegularLatticeError::SelfIntersectingBoundary(i,j));
            }
        }
    }
    let area = polygon_area2(p);
    if !area.is_finite() || area.abs() <= tol*tol { return Err(RegularLatticeError::DegenerateBoundary); }
    Ok(())
}
fn segments_intersect(a: ControlPointPosition, b: ControlPointPosition,
    c: ControlPointPosition, d: ControlPointPosition, tol: f64) -> bool {
    let (o1,o2,o3,o4) = (cross(a,b,c),cross(a,b,d),cross(c,d,a),cross(c,d,b));
    let eps = tol*tol;
    if ((o1 > eps && o2 < -eps)||(o1 < -eps && o2 > eps))
        && ((o3 > eps && o4 < -eps)||(o3 < -eps && o4 > eps)) { return true; }
    on_segment(c,a,b,tol)||on_segment(d,a,b,tol)||on_segment(a,c,d,tol)||on_segment(b,c,d,tol)
}
fn on_segment(p: ControlPointPosition, a: ControlPointPosition, b: ControlPointPosition, tol: f64) -> bool {
    cross(a,b,p).abs() <= tol*tol
        && p.x >= a.x.min(b.x)-tol && p.x <= a.x.max(b.x)+tol
        && p.y >= a.y.min(b.y)-tol && p.y <= a.y.max(b.y)+tol
}

#[derive(Clone, Debug, PartialEq)]
pub enum RegularLatticeError {
    Topology(TopologyContractError), DegenerateBounds, InvalidFirstId, IdOverflow,
    TooManyPoints(usize), BoundaryTooLarge(usize), CoordinateRangeExceeded, Empty, MissingPoint(u64), InvalidFace, UnexpectedTriangle,
    InvalidBoundary, CoincidentPoints(usize,usize), ZeroLengthCellEdge(usize),
    DegenerateCell(usize), InvertedCell(usize), NonConvexCell(usize),
    ZeroLengthBoundaryEdge(usize), SelfIntersectingBoundary(usize,usize), DegenerateBoundary,
}
impl std::fmt::Display for RegularLatticeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Topology(e) => write!(f,"invalid lattice topology: {e}"),
            Self::DegenerateBounds => write!(f,"lattice bounds must have finite positive width and height"),
            Self::InvalidFirstId => write!(f,"first control-point ID must be non-zero"),
            Self::IdOverflow => write!(f,"control-point ID range overflows u64"),
            Self::TooManyPoints(n) => write!(f,"lattice requests {n} points; maximum is {MAX_LATTICE_CONTROL_POINTS}"),
            Self::BoundaryTooLarge(n) => write!(f,"lattice boundary has {n} points; maximum for bounded intersection validation is {MAX_LATTICE_BOUNDARY_POINTS}"),
            Self::CoordinateRangeExceeded => write!(f,"coordinates exceed the supported range for scale-aware spatial validation"),
            Self::Empty => write!(f,"lattice has no geometry"),
            Self::MissingPoint(id) => write!(f,"control point {id} is missing"),
            Self::InvalidFace => write!(f,"lattice face index is invalid"),
            Self::UnexpectedTriangle => write!(f,"regular lattice cells must be quads"),
            Self::InvalidBoundary => write!(f,"lattice boundary is invalid"),
            Self::CoincidentPoints(a,b) => write!(f,"control points {a} and {b} are coincident or too close"),
            Self::ZeroLengthCellEdge(i) => write!(f,"cell {i} has a zero-length edge"),
            Self::DegenerateCell(i) => write!(f,"cell {i} is degenerate"),
            Self::InvertedCell(i) => write!(f,"cell {i} has inconsistent orientation"),
            Self::NonConvexCell(i) => write!(f,"cell {i} is folded or non-convex"),
            Self::ZeroLengthBoundaryEdge(i) => write!(f,"boundary edge {i} is zero-length"),
            Self::SelfIntersectingBoundary(a,b) => write!(f,"boundary edges {a} and {b} intersect"),
            Self::DegenerateBoundary => write!(f,"lattice boundary has near-zero area"),
        }
    }
}
impl std::error::Error for RegularLatticeError {}
