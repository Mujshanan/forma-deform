//! Explicit-connectivity irregular cages and geometric validation.
//!
//! Faces retain authored vertex order. Validation never repairs winding,
//! connectivity, or point positions. Global overlap detection between non-adjacent
//! interior faces is intentionally deferred; this module validates topology,
//! local face geometry, and the single outer boundary.

use crate::topology::{
    ControlPointId, ControlPointPosition, DeformationControlState, IrregularCageTopology,
    TopologyContractError, TopologyFace, TopologyView,
};
use std::collections::{HashMap, HashSet, VecDeque};

pub const MAX_IRREGULAR_CAGE_POINTS: usize = 100_000;
pub const MAX_IRREGULAR_CAGE_FACES: usize = 100_000;
pub const MAX_IRREGULAR_CAGE_BOUNDARY_POINTS: usize = 4_096;
const MIN_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug, PartialEq)]
pub struct IrregularCage {
    pub topology: IrregularCageTopology,
    pub control_state: DeformationControlState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct EdgeKey(ControlPointId, ControlPointId);

impl EdgeKey {
    fn new(a: ControlPointId, b: ControlPointId) -> Self {
        if a < b { Self(a, b) } else { Self(b, a) }
    }
}

#[derive(Clone, Copy, Debug)]
struct DirectedEdge {
    from: ControlPointId,
    to: ControlPointId,
    face: usize,
}

impl IrregularCage {
    /// Construct from explicit topology and control state, then validate both.
    pub fn new(
        topology: IrregularCageTopology,
        control_state: DeformationControlState,
    ) -> Result<Self, IrregularCageError> {
        let cage = Self { topology, control_state };
        cage.validate_geometry()?;
        Ok(cage)
    }

    /// Validate identity, connectivity, manifoldness, face geometry, and boundary.
    pub fn validate_geometry(&self) -> Result<(), IrregularCageError> {
        self.topology.validate_structure().map_err(IrregularCageError::Topology)?;
        if self.topology.control_point_ids.len() > MAX_IRREGULAR_CAGE_POINTS {
            return Err(IrregularCageError::TooManyPoints(self.topology.control_point_ids.len()));
        }
        if self.topology.faces.len() > MAX_IRREGULAR_CAGE_FACES {
            return Err(IrregularCageError::TooManyFaces(self.topology.faces.len()));
        }
        self.control_state.validate_against(&self.topology)
            .map_err(IrregularCageError::Topology)?;

        let positions = self.position_lookup()?;
        let ordered: Vec<_> = self.topology.control_point_ids.iter()
            .map(|id| positions.get(id).copied()
                .ok_or(IrregularCageError::MissingPoint(id.get())))
            .collect::<Result<_, _>>()?;
        let (min_x, min_y, max_x, max_y) = bounds(&ordered)?;
        let diagonal = (max_x - min_x).hypot(max_y - min_y);
        if !diagonal.is_finite() || diagonal <= 0.0 {
            return Err(IrregularCageError::DegenerateBounds);
        }
        let tolerance = (diagonal * 1.0e-9).max(MIN_TOLERANCE);
        reject_coincident_points(&ordered, tolerance)?;

        let edge_map = self.build_edge_map()?;
        self.validate_face_connectivity(&edge_map)?;
        self.validate_face_geometry(&positions, tolerance)?;
        let boundary = self.extract_boundary(&edge_map)?;
        if boundary.len() > MAX_IRREGULAR_CAGE_BOUNDARY_POINTS {
            return Err(IrregularCageError::BoundaryTooLarge(boundary.len()));
        }
        let boundary_positions: Vec<_> = boundary.iter()
            .map(|id| positions.get(id).copied()
                .ok_or(IrregularCageError::MissingPoint(id.get())))
            .collect::<Result<_, _>>()?;
        validate_boundary(&boundary_positions, tolerance)?;
        Ok(())
    }

    /// Return the single outer boundary in deterministic directed-edge order.
    pub fn boundary_control_point_ids(&self) -> Result<Vec<ControlPointId>, IrregularCageError> {
        let edge_map = self.build_edge_map()?;
        self.extract_boundary(&edge_map)
    }

    fn position_lookup(&self) -> Result<HashMap<ControlPointId, ControlPointPosition>, IrregularCageError> {
        self.control_state.points.iter().map(|point| {
            point.position.validate().map_err(IrregularCageError::Topology)?;
            Ok((point.id, point.position))
        }).collect()
    }

    fn build_edge_map(&self) -> Result<HashMap<EdgeKey, Vec<DirectedEdge>>, IrregularCageError> {
        let mut edges: HashMap<EdgeKey, Vec<DirectedEdge>> = HashMap::new();
        for (face_index, face) in self.topology.faces.iter().enumerate() {
            let ids = face.control_point_ids();
            for index in 0..ids.len() {
                let from = ids[index];
                let to = ids[(index + 1) % ids.len()];
                let incidences = edges.entry(EdgeKey::new(from, to)).or_default();
                incidences.push(DirectedEdge { from, to, face: face_index });
                if incidences.len() > 2 {
                    return Err(IrregularCageError::NonManifoldEdge(from.get(), to.get()));
                }
            }
        }
        for incidences in edges.values() {
            if incidences.len() == 2
                && !(incidences[0].from == incidences[1].to
                    && incidences[0].to == incidences[1].from)
            {
                return Err(IrregularCageError::InconsistentSharedEdgeWinding {
                    first_face: incidences[0].face,
                    second_face: incidences[1].face,
                });
            }
        }
        Ok(edges)
    }

    fn validate_face_connectivity(
        &self,
        edges: &HashMap<EdgeKey, Vec<DirectedEdge>>,
    ) -> Result<(), IrregularCageError> {
        let mut adjacency = vec![Vec::new(); self.topology.faces.len()];
        for incidences in edges.values() {
            if incidences.len() == 2 {
                let a = incidences[0].face;
                let b = incidences[1].face;
                adjacency[a].push(b);
                adjacency[b].push(a);
            }
        }
        let mut visited = vec![false; adjacency.len()];
        let mut queue = VecDeque::from([0usize]);
        visited[0] = true;
        while let Some(face) = queue.pop_front() {
            for &neighbor in &adjacency[face] {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }
        if let Some(index) = visited.iter().position(|visited| !visited) {
            return Err(IrregularCageError::DisconnectedFace(index));
        }
        Ok(())
    }

    fn validate_face_geometry(
        &self,
        positions: &HashMap<ControlPointId, ControlPointPosition>,
        tolerance: f64,
    ) -> Result<(), IrregularCageError> {
        let mut expected_sign = 0.0_f64;
        for (face_index, face) in self.topology.faces.iter().enumerate() {
            let ids = face.control_point_ids();
            let points: Vec<_> = ids.iter().map(|id| positions.get(id).copied()
                .ok_or(IrregularCageError::MissingPoint(id.get())))
                .collect::<Result<_, _>>()?;
            for edge in 0..points.len() {
                if distance(points[edge], points[(edge + 1) % points.len()]) <= tolerance {
                    return Err(IrregularCageError::ZeroLengthFaceEdge(face_index, edge));
                }
            }
            let signed_area = polygon_area2(&points);
            if !signed_area.is_finite() || signed_area.abs() <= tolerance * tolerance {
                return Err(IrregularCageError::DegenerateFace(face_index));
            }
            let sign = signed_area.signum();
            if expected_sign == 0.0 {
                expected_sign = sign;
            } else if sign != expected_sign {
                return Err(IrregularCageError::InconsistentFaceWinding(face_index));
            }

            // Triangles are convex unless degenerate; quads must be strictly convex.
            if matches!(face, TopologyFace::Quad(_)) {
                for corner in 0..4 {
                    let turn = cross(
                        points[corner],
                        points[(corner + 1) % 4],
                        points[(corner + 2) % 4],
                    );
                    if !turn.is_finite() || turn.abs() <= tolerance * tolerance {
                        return Err(IrregularCageError::DegenerateFace(face_index));
                    }
                    if turn.signum() != sign {
                        return Err(IrregularCageError::NonConvexFace(face_index));
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_boundary(
        &self,
        edges: &HashMap<EdgeKey, Vec<DirectedEdge>>,
    ) -> Result<Vec<ControlPointId>, IrregularCageError> {
        let boundary_edges: Vec<_> = edges.values()
            .filter(|incidences| incidences.len() == 1)
            .map(|incidences| incidences[0])
            .collect();
        if boundary_edges.len() < 3 {
            return Err(IrregularCageError::InvalidBoundary);
        }
        if boundary_edges.len() > MAX_IRREGULAR_CAGE_BOUNDARY_POINTS {
            return Err(IrregularCageError::BoundaryTooLarge(boundary_edges.len()));
        }

        let mut outgoing: HashMap<ControlPointId, ControlPointId> = HashMap::new();
        let mut incoming: HashSet<ControlPointId> = HashSet::new();
        for edge in &boundary_edges {
            if outgoing.insert(edge.from, edge.to).is_some() || !incoming.insert(edge.to) {
                return Err(IrregularCageError::BoundaryBranch);
            }
        }
        if outgoing.len() != boundary_edges.len() || incoming.len() != boundary_edges.len()
            || outgoing.keys().any(|id| !incoming.contains(id))
        {
            return Err(IrregularCageError::BoundaryBranch);
        }

        let start = boundary_edges.iter().map(|edge| edge.from).min()
            .ok_or(IrregularCageError::InvalidBoundary)?;
        let mut loop_ids = Vec::with_capacity(boundary_edges.len());
        let mut seen = HashSet::with_capacity(boundary_edges.len());
        let mut current = start;
        loop {
            if !seen.insert(current) {
                if current == start { break; }
                return Err(IrregularCageError::MultipleBoundaryLoops);
            }
            loop_ids.push(current);
            current = *outgoing.get(&current).ok_or(IrregularCageError::BoundaryBranch)?;
            if current == start { break; }
            if loop_ids.len() >= boundary_edges.len() {
                return Err(IrregularCageError::MultipleBoundaryLoops);
            }
        }
        if loop_ids.len() != boundary_edges.len() {
            return Err(IrregularCageError::MultipleBoundaryLoops);
        }
        Ok(loop_ids)
    }
}

fn bounds(points: &[ControlPointPosition]) -> Result<(f64, f64, f64, f64), IrregularCageError> {
    let first = points.first().ok_or(IrregularCageError::Empty)?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Ok((min_x, min_y, max_x, max_y))
}

fn reject_coincident_points(points: &[ControlPointPosition], tolerance: f64) -> Result<(), IrregularCageError> {
    let mut buckets: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (index, point) in points.iter().enumerate() {
        let qx = (point.x / tolerance).floor();
        let qy = (point.y / tolerance).floor();
        if !qx.is_finite() || !qy.is_finite() || qx.abs() > 9.0e18 || qy.abs() > 9.0e18 {
            return Err(IrregularCageError::CoordinateRangeExceeded);
        }
        let cell = (qx as i64, qy as i64);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(candidates) = buckets.get(&(cell.0 + dx, cell.1 + dy)) {
                    for &previous in candidates {
                        if distance(*point, points[previous]) <= tolerance {
                            return Err(IrregularCageError::CoincidentPoints(previous, index));
                        }
                    }
                }
            }
        }
        buckets.entry(cell).or_default().push(index);
    }
    Ok(())
}

fn distance(a: ControlPointPosition, b: ControlPointPosition) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}
fn cross(a: ControlPointPosition, b: ControlPointPosition, c: ControlPointPosition) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}
fn polygon_area2(points: &[ControlPointPosition]) -> f64 {
    (0..points.len()).map(|index| {
        points[index].x * points[(index + 1) % points.len()].y
            - points[(index + 1) % points.len()].x * points[index].y
    }).sum()
}
fn validate_boundary(points: &[ControlPointPosition], tolerance: f64) -> Result<(), IrregularCageError> {
    if points.len() < 3 { return Err(IrregularCageError::InvalidBoundary); }
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        if distance(a, b) <= tolerance {
            return Err(IrregularCageError::ZeroLengthBoundaryEdge(i));
        }
        for j in (i + 1)..points.len() {
            if j == i + 1 || (i == 0 && j == points.len() - 1) { continue; }
            let c = points[j];
            let d = points[(j + 1) % points.len()];
            if segments_intersect(a, b, c, d, tolerance) {
                return Err(IrregularCageError::SelfIntersectingBoundary(i, j));
            }
        }
    }
    let area = polygon_area2(points);
    if !area.is_finite() || area.abs() <= tolerance * tolerance {
        return Err(IrregularCageError::DegenerateBoundary);
    }
    Ok(())
}
fn segments_intersect(
    a: ControlPointPosition, b: ControlPointPosition,
    c: ControlPointPosition, d: ControlPointPosition, tolerance: f64,
) -> bool {
    let (o1, o2, o3, o4) = (cross(a,b,c), cross(a,b,d), cross(c,d,a), cross(c,d,b));
    let epsilon = tolerance * tolerance;
    if ((o1 > epsilon && o2 < -epsilon) || (o1 < -epsilon && o2 > epsilon))
        && ((o3 > epsilon && o4 < -epsilon) || (o3 < -epsilon && o4 > epsilon)) {
        return true;
    }
    on_segment(c,a,b,tolerance) || on_segment(d,a,b,tolerance)
        || on_segment(a,c,d,tolerance) || on_segment(b,c,d,tolerance)
}
fn on_segment(p: ControlPointPosition, a: ControlPointPosition, b: ControlPointPosition, tolerance: f64) -> bool {
    cross(a,b,p).abs() <= tolerance * tolerance
        && p.x >= a.x.min(b.x) - tolerance && p.x <= a.x.max(b.x) + tolerance
        && p.y >= a.y.min(b.y) - tolerance && p.y <= a.y.max(b.y) + tolerance
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrregularCageError {
    Topology(TopologyContractError),
    TooManyPoints(usize),
    TooManyFaces(usize),
    DegenerateBounds,
    CoordinateRangeExceeded,
    Empty,
    MissingPoint(u64),
    CoincidentPoints(usize, usize),
    NonManifoldEdge(u64, u64),
    InconsistentSharedEdgeWinding { first_face: usize, second_face: usize },
    DisconnectedFace(usize),
    ZeroLengthFaceEdge(usize, usize),
    DegenerateFace(usize),
    InconsistentFaceWinding(usize),
    NonConvexFace(usize),
    InvalidBoundary,
    BoundaryTooLarge(usize),
    BoundaryBranch,
    MultipleBoundaryLoops,
    ZeroLengthBoundaryEdge(usize),
    SelfIntersectingBoundary(usize, usize),
    DegenerateBoundary,
}

impl std::fmt::Display for IrregularCageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Topology(error) => write!(f, "invalid cage topology: {error}"),
            Self::TooManyPoints(count) => write!(f, "cage has {count} points; maximum is {MAX_IRREGULAR_CAGE_POINTS}"),
            Self::TooManyFaces(count) => write!(f, "cage has {count} faces; maximum is {MAX_IRREGULAR_CAGE_FACES}"),
            Self::DegenerateBounds => write!(f, "cage bounds must have finite positive diagonal"),
            Self::CoordinateRangeExceeded => write!(f, "coordinates exceed the supported range for scale-aware validation"),
            Self::Empty => write!(f, "cage has no control-point geometry"),
            Self::MissingPoint(id) => write!(f, "control point {id} is missing"),
            Self::CoincidentPoints(a, b) => write!(f, "control points at storage positions {a} and {b} are coincident or too close"),
            Self::NonManifoldEdge(a, b) => write!(f, "edge ({a}, {b}) is shared by more than two faces"),
            Self::InconsistentSharedEdgeWinding { first_face, second_face } => write!(f, "faces {first_face} and {second_face} traverse their shared edge in the same direction"),
            Self::DisconnectedFace(index) => write!(f, "face {index} is disconnected from the cage face component"),
            Self::ZeroLengthFaceEdge(face, edge) => write!(f, "face {face} has a zero-length edge at local edge {edge}"),
            Self::DegenerateFace(index) => write!(f, "face {index} has near-zero area or a degenerate corner"),
            Self::InconsistentFaceWinding(index) => write!(f, "face {index} has winding inconsistent with the other faces"),
            Self::NonConvexFace(index) => write!(f, "quad face {index} is concave or folded"),
            Self::InvalidBoundary => write!(f, "cage does not have a valid closed outer boundary"),
            Self::BoundaryTooLarge(count) => write!(f, "boundary has {count} vertices; maximum for bounded intersection validation is {MAX_IRREGULAR_CAGE_BOUNDARY_POINTS}"),
            Self::BoundaryBranch => write!(f, "boundary vertices must each have exactly one incoming and one outgoing boundary edge"),
            Self::MultipleBoundaryLoops => write!(f, "cage must have exactly one boundary loop and no holes"),
            Self::ZeroLengthBoundaryEdge(index) => write!(f, "boundary edge {index} is zero-length"),
            Self::SelfIntersectingBoundary(a, b) => write!(f, "boundary edges {a} and {b} intersect"),
            Self::DegenerateBoundary => write!(f, "outer boundary has near-zero area"),
        }
    }
}
impl std::error::Error for IrregularCageError {}
