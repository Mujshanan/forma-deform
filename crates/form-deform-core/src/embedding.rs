//! Global planar embedding validation for deformation cages.
//!
//! Local face validity does not prove that the complete mesh is embedded
//! without overlap. This module validates pairwise face relationships and
//! issues a certificate tied to the exact topology and control-state revision.

use crate::topology::{
    ControlPointId, ControlPointPosition, ControlStateRevision, DeformationControlState,
    DeformationTopology, DeformationTopologyId, DeformationTopologyRevision, TopologyFace,
    TopologyView,
};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

pub const MAX_EMBEDDING_POINTS: usize = 100_000;
pub const MAX_EMBEDDING_FACES: usize = 100_000;
/// A hard work bound prevents adversarial all-overlapping AABBs from causing
/// unbounded quadratic work. Exceeding it fails closed.
pub const MAX_FACE_PAIR_CANDIDATES: usize = 8_000_000;

const RELATIVE_TOLERANCE: f64 = 1.0e-9;
const MIN_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmbeddingTolerance {
    pub spatial: f64,
    pub area: f64,
}

impl EmbeddingTolerance {
    fn from_bounds(points: &[ControlPointPosition]) -> Result<Self, EmbeddingError> {
        let first = points.first().ok_or(EmbeddingError::EmptyGeometry)?;
        let (mut min_x, mut min_y, mut max_x, mut max_y) =
            (first.x, first.y, first.x, first.y);
        for point in points.iter().skip(1) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
        let diagonal = (max_x - min_x).hypot(max_y - min_y);
        if !diagonal.is_finite() || diagonal <= 0.0 {
            return Err(EmbeddingError::DegenerateBounds);
        }
        let spatial = (diagonal * RELATIVE_TOLERANCE).max(MIN_TOLERANCE);
        let area = spatial * diagonal;
        if !spatial.is_finite() || !area.is_finite() {
            return Err(EmbeddingError::CoordinateRangeExceeded);
        }
        Ok(Self { spatial, area })
    }
}

/// Evidence that a particular cage geometry passed global embedding checks.
///
/// Fields are private: callers cannot manufacture a certificate. The
/// certificate is intentionally invalidated by any control-state revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CageEmbeddingCertificate {
    topology_id: DeformationTopologyId,
    topology_revision: DeformationTopologyRevision,
    control_state_revision: ControlStateRevision,
    geometry_fingerprint: [u8; 32],
    face_count: usize,
}

impl CageEmbeddingCertificate {
    pub fn topology_id(&self) -> DeformationTopologyId { self.topology_id }
    pub fn topology_revision(&self) -> DeformationTopologyRevision { self.topology_revision }
    pub fn control_state_revision(&self) -> ControlStateRevision { self.control_state_revision }
    pub fn geometry_fingerprint(&self) -> &[u8; 32] { &self.geometry_fingerprint }
    pub fn face_count(&self) -> usize { self.face_count }

    /// Check that the certificate still describes this exact cage state.
    pub fn matches(
        &self,
        topology: &DeformationTopology,
        control_state: &DeformationControlState,
    ) -> bool {
        self.topology_id == topology.topology_id()
            && self.topology_revision == topology.topology_revision()
            && self.control_state_revision == control_state.revision
            && geometry_fingerprint(topology, control_state)
                .is_ok_and(|fingerprint| fingerprint == self.geometry_fingerprint)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmbeddingError {
    TopologyMismatch,
    InvalidTopology(String),
    TooManyPoints(usize),
    TooManyFaces(usize),
    EmptyGeometry,
    DegenerateBounds,
    CoordinateRangeExceeded,
    MissingControlPoint(u64),
    DuplicateControlPoint(u64),
    CoincidentControlPoints(usize, usize),
    NonFiniteCoordinate(u64),
    InvalidFace(usize),
    DegenerateFace(usize),
    InconsistentFaceWinding(usize),
    NonConvexQuad(usize),
    NonManifoldEdge(u64, u64),
    InvalidSharedEdgeWinding(usize, usize),
    DisconnectedFaces(usize),
    InvalidBoundary,
    CandidateLimitExceeded,
    NonAdjacentFaceIntersection { first_face: usize, second_face: usize },
    FaceInteriorOverlap { first_face: usize, second_face: usize },
}

#[derive(Clone, Copy, Debug)]
struct FaceGeometry {
    index: usize,
    ids: FaceIds,
    points: FacePoints,
    bounds: Aabb,
}

#[derive(Clone, Copy, Debug)]
enum FaceIds {
    Triangle([ControlPointId; 3]),
    Quad([ControlPointId; 4]),
}

impl FaceIds {
    fn as_slice(&self) -> &[ControlPointId] {
        match self {
            Self::Triangle(ids) => ids,
            Self::Quad(ids) => ids,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum FacePoints {
    Triangle([ControlPointPosition; 3]),
    Quad([ControlPointPosition; 4]),
}

impl FacePoints {
    fn as_slice(&self) -> &[ControlPointPosition] {
        match self {
            Self::Triangle(points) => points,
            Self::Quad(points) => points,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Aabb {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Aabb {
    fn from_points(points: &[ControlPointPosition]) -> Self {
        let first = points[0];
        let (mut min_x, mut min_y, mut max_x, mut max_y) =
            (first.x, first.y, first.x, first.y);
        for point in points.iter().skip(1) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
        Self { min_x, min_y, max_x, max_y }
    }

    fn overlaps(self, other: Self, tolerance: f64) -> bool {
        self.min_x <= other.max_x + tolerance
            && self.max_x + tolerance >= other.min_x
            && self.min_y <= other.max_y + tolerance
            && self.max_y + tolerance >= other.min_y
    }
}

/// Validate local face geometry and global pairwise embedding.
///
/// This function is the only constructor path for a certificate. It accepts
/// the strongly typed topology enum and its matching control state.
pub fn validate_cage_embedding(
    topology: &DeformationTopology,
    control_state: &DeformationControlState,
) -> Result<CageEmbeddingCertificate, EmbeddingError> {
    if topology.topology_id() != control_state.topology_id
        || topology.topology_revision() != control_state.topology_revision
    {
        return Err(EmbeddingError::TopologyMismatch);
    }
    match topology {
        DeformationTopology::RegularLattice(value) => value.validate_structure(),
        DeformationTopology::IrregularCage(value) => value.validate_structure(),
    }.map_err(|error| EmbeddingError::InvalidTopology(error.to_string()))?;
    control_state.validate_against(topology)
        .map_err(|error| EmbeddingError::InvalidTopology(error.to_string()))?;

    let point_ids = topology.control_point_ids();
    let face_count = topology.face_count();
    if point_ids.is_empty() || face_count == 0 {
        return Err(EmbeddingError::EmptyGeometry);
    }
    if point_ids.len() > MAX_EMBEDDING_POINTS {
        return Err(EmbeddingError::TooManyPoints(point_ids.len()));
    }
    if face_count > MAX_EMBEDDING_FACES {
        return Err(EmbeddingError::TooManyFaces(face_count));
    }

    let positions = build_position_lookup(point_ids, control_state)?;
    let ordered_points: Vec<_> = point_ids.iter()
        .map(|id| positions.get(id).copied().ok_or(EmbeddingError::MissingControlPoint(id.get())))
        .collect::<Result<_, _>>()?;
    let tolerance = EmbeddingTolerance::from_bounds(&ordered_points)?;
    reject_near_coincident_points(&ordered_points, tolerance)?;
    let faces = build_faces(topology, &positions, tolerance)?;
    validate_shared_edge_incidence(&faces)?;
    validate_face_connectivity(&faces)?;
    validate_boundary_and_point_usage(&faces, point_ids)?;
    validate_pairwise_embedding(&faces, tolerance)?;

    Ok(CageEmbeddingCertificate {
        topology_id: topology.topology_id(),
        topology_revision: topology.topology_revision(),
        control_state_revision: control_state.revision,
        geometry_fingerprint: geometry_fingerprint(topology, control_state)?,
        face_count,
    })
}

fn build_position_lookup(
    topology_ids: &[ControlPointId],
    state: &DeformationControlState,
) -> Result<HashMap<ControlPointId, ControlPointPosition>, EmbeddingError> {
    if state.points.len() != topology_ids.len() {
        return Err(EmbeddingError::InvalidTopology(
            "control state must contain exactly one point for every topology ID".into(),
        ));
    }
    let expected: HashSet<_> = topology_ids.iter().copied().collect();
    let mut lookup = HashMap::with_capacity(state.points.len());
    for point in &state.points {
        if !expected.contains(&point.id) {
            return Err(EmbeddingError::InvalidTopology(
                "control state contains an ID absent from topology".into(),
            ));
        }
        if lookup.insert(point.id, point.position).is_some() {
            return Err(EmbeddingError::DuplicateControlPoint(point.id.get()));
        }
        if !point.position.x.is_finite() || !point.position.y.is_finite() {
            return Err(EmbeddingError::NonFiniteCoordinate(point.id.get()));
        }
    }
    if lookup.len() != expected.len() {
        return Err(EmbeddingError::InvalidTopology(
            "control state is missing one or more topology IDs".into(),
        ));
    }
    Ok(lookup)
}

fn reject_near_coincident_points(
    points: &[ControlPointPosition],
    tolerance: EmbeddingTolerance,
) -> Result<(), EmbeddingError> {
    let mut buckets: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (index, point) in points.iter().enumerate() {
        let cell_x = (point.x / tolerance.spatial).floor();
        let cell_y = (point.y / tolerance.spatial).floor();
        if !cell_x.is_finite() || !cell_y.is_finite()
            || cell_x.abs() > 9.0e18 || cell_y.abs() > 9.0e18
        {
            return Err(EmbeddingError::CoordinateRangeExceeded);
        }
        let cell = (cell_x as i64, cell_y as i64);
        for offset_x in -1..=1 {
            for offset_y in -1..=1 {
                if let Some(previous) = buckets.get(&(cell.0 + offset_x, cell.1 + offset_y)) {
                    for &other_index in previous {
                        if distance(*point, points[other_index]) <= tolerance.spatial {
                            return Err(EmbeddingError::CoincidentControlPoints(
                                other_index, index,
                            ));
                        }
                    }
                }
            }
        }
        buckets.entry(cell).or_default().push(index);
    }
    Ok(())
}

fn build_faces(
    topology: &DeformationTopology,
    positions: &HashMap<ControlPointId, ControlPointPosition>,
    tolerance: EmbeddingTolerance,
) -> Result<Vec<FaceGeometry>, EmbeddingError> {
    let mut result = Vec::with_capacity(topology.face_count());
    let mut winding = None;
    for index in 0..topology.face_count() {
        let face = topology.face_at(index).ok_or(EmbeddingError::InvalidFace(index))?;
        let (ids, points) = match face {
            TopologyFace::Triangle(ids) => {
                let points = [
                    position(positions, ids[0])?,
                    position(positions, ids[1])?,
                    position(positions, ids[2])?,
                ];
                (FaceIds::Triangle(ids), FacePoints::Triangle(points))
            }
            TopologyFace::Quad(ids) => {
                let points = [
                    position(positions, ids[0])?,
                    position(positions, ids[1])?,
                    position(positions, ids[2])?,
                    position(positions, ids[3])?,
                ];
                (FaceIds::Quad(ids), FacePoints::Quad(points))
            }
        };
        let polygon = points.as_slice();
        let area = signed_area_twice(polygon);
        if !area.is_finite() || area.abs() <= tolerance.area {
            return Err(EmbeddingError::DegenerateFace(index));
        }
        let sign = area.signum();
        if winding.is_some_and(|expected| expected != sign) {
            return Err(EmbeddingError::InconsistentFaceWinding(index));
        }
        winding = Some(sign);

        if polygon.len() == 4 {
            for corner in 0..4 {
                let turn = cross(
                    polygon[corner],
                    polygon[(corner + 1) % 4],
                    polygon[(corner + 2) % 4],
                );
                if !turn.is_finite() || turn.abs() <= tolerance.area
                    || turn.signum() != sign
                {
                    return Err(EmbeddingError::NonConvexQuad(index));
                }
            }
        }
        for edge in 0..polygon.len() {
            if distance(polygon[edge], polygon[(edge + 1) % polygon.len()])
                <= tolerance.spatial
            {
                return Err(EmbeddingError::DegenerateFace(index));
            }
        }

        result.push(FaceGeometry {
            index,
            ids,
            points,
            bounds: Aabb::from_points(polygon),
        });
    }
    Ok(result)
}

fn position(
    positions: &HashMap<ControlPointId, ControlPointPosition>,
    id: ControlPointId,
) -> Result<ControlPointPosition, EmbeddingError> {
    positions.get(&id).copied().ok_or(EmbeddingError::MissingControlPoint(id.get()))
}

type SharedEdgeKey = (ControlPointId, ControlPointId);
type EdgeIncidence = (usize, ControlPointId, ControlPointId);
type EdgeIncidenceMap = HashMap<SharedEdgeKey, Vec<EdgeIncidence>>;

fn validate_shared_edge_incidence(faces: &[FaceGeometry]) -> Result<(), EmbeddingError> {
    let mut edges: EdgeIncidenceMap = HashMap::new();
    for face in faces {
        let ids = face.ids.as_slice();
        for edge in 0..ids.len() {
            let from = ids[edge];
            let to = ids[(edge + 1) % ids.len()];
            let key = if from < to { (from, to) } else { (to, from) };
            let incidences = edges.entry(key).or_default();
            incidences.push((face.index, from, to));
            if incidences.len() > 2 {
                return Err(EmbeddingError::NonManifoldEdge(key.0.get(), key.1.get()));
            }
        }
    }
    for incidences in edges.values() {
        if incidences.len() == 2
            && !(incidences[0].1 == incidences[1].2 && incidences[0].2 == incidences[1].1)
        {
            return Err(EmbeddingError::InvalidSharedEdgeWinding(
                incidences[0].0, incidences[1].0,
            ));
        }
    }
    Ok(())
}

fn validate_boundary_and_point_usage(
    faces: &[FaceGeometry],
    topology_point_ids: &[ControlPointId],
) -> Result<(), EmbeddingError> {
    let mut edge_faces: EdgeIncidenceMap = HashMap::new();
    let mut referenced = HashSet::new();

    for face in faces {
        let ids = face.ids.as_slice();
        for index in 0..ids.len() {
            let from = ids[index];
            let to = ids[(index + 1) % ids.len()];
            referenced.insert(from);
            let key = if from < to { (from, to) } else { (to, from) };
            edge_faces.entry(key).or_default().push((face.index, from, to));
        }
    }
    if let Some(unused) = topology_point_ids.iter().find(|id| !referenced.contains(id)) {
        return Err(EmbeddingError::InvalidTopology(format!(
            "control point {} is not referenced by any face", unused.get()
        )));
    }

    let boundary: Vec<_> = edge_faces.values()
        .filter(|incidences| incidences.len() == 1)
        .map(|incidences| (incidences[0].1, incidences[0].2))
        .collect();
    if boundary.len() < 3 {
        return Err(EmbeddingError::InvalidBoundary);
    }

    let mut outgoing = HashMap::with_capacity(boundary.len());
    let mut incoming = HashSet::with_capacity(boundary.len());
    for (from, to) in &boundary {
        if outgoing.insert(*from, *to).is_some() || !incoming.insert(*to) {
            return Err(EmbeddingError::InvalidBoundary);
        }
    }
    if outgoing.len() != boundary.len()
        || incoming.len() != boundary.len()
        || outgoing.keys().any(|id| !incoming.contains(id))
    {
        return Err(EmbeddingError::InvalidBoundary);
    }

    let start = outgoing.keys().copied().min().ok_or(EmbeddingError::InvalidBoundary)?;
    let mut current = start;
    let mut visited = HashSet::with_capacity(boundary.len());
    loop {
        if !visited.insert(current) {
            if current == start { break; }
            return Err(EmbeddingError::InvalidBoundary);
        }
        current = *outgoing.get(&current).ok_or(EmbeddingError::InvalidBoundary)?;
        if current == start { break; }
        if visited.len() >= boundary.len() {
            return Err(EmbeddingError::InvalidBoundary);
        }
    }
    if visited.len() != boundary.len() {
        return Err(EmbeddingError::InvalidBoundary);
    }
    Ok(())
}

fn validate_face_connectivity(faces: &[FaceGeometry]) -> Result<(), EmbeddingError> {
    let mut edge_faces: HashMap<(ControlPointId, ControlPointId), Vec<usize>> = HashMap::new();
    for face in faces {
        let ids = face.ids.as_slice();
        for edge in 0..ids.len() {
            let a = ids[edge];
            let b = ids[(edge + 1) % ids.len()];
            let key = if a < b { (a, b) } else { (b, a) };
            edge_faces.entry(key).or_default().push(face.index);
        }
    }
    let mut adjacency = vec![Vec::new(); faces.len()];
    for incident in edge_faces.values().filter(|incident| incident.len() == 2) {
        adjacency[incident[0]].push(incident[1]);
        adjacency[incident[1]].push(incident[0]);
    }
    let mut visited = vec![false; faces.len()];
    let mut stack = vec![0usize];
    visited[0] = true;
    while let Some(face) = stack.pop() {
        for &neighbor in &adjacency[face] {
            if !visited[neighbor] {
                visited[neighbor] = true;
                stack.push(neighbor);
            }
        }
    }
    if let Some(index) = visited.iter().position(|visited| !visited) {
        return Err(EmbeddingError::DisconnectedFaces(index));
    }
    Ok(())
}

fn validate_pairwise_embedding(
    faces: &[FaceGeometry],
    tolerance: EmbeddingTolerance,
) -> Result<(), EmbeddingError> {
    // Sweep by minimum X. The AABB filter keeps ordinary meshes near-linear;
    // the explicit candidate budget bounds worst-case adversarial input.
    let mut order: Vec<_> = (0..faces.len()).collect();
    order.sort_by(|&a, &b| {
        faces[a].bounds.min_x.total_cmp(&faces[b].bounds.min_x)
            .then_with(|| faces[a].index.cmp(&faces[b].index))
    });

    let mut candidates = 0usize;
    for position_in_order in 0..order.len() {
        let first = faces[order[position_in_order]];
        for &second_index in order.iter().skip(position_in_order + 1) {
            let second = faces[second_index];
            if second.bounds.min_x > first.bounds.max_x + tolerance.spatial {
                break;
            }
            if !first.bounds.overlaps(second.bounds, tolerance.spatial) {
                continue;
            }
            candidates = candidates.checked_add(1)
                .ok_or(EmbeddingError::CandidateLimitExceeded)?;
            if candidates > MAX_FACE_PAIR_CANDIDATES {
                return Err(EmbeddingError::CandidateLimitExceeded);
            }
            validate_face_pair(first, second, tolerance)?;
        }
    }
    Ok(())
}

fn validate_face_pair(
    first: FaceGeometry,
    second: FaceGeometry,
    tolerance: EmbeddingTolerance,
) -> Result<(), EmbeddingError> {
    let shared: Vec<_> = first.ids.as_slice().iter().copied()
        .filter(|id| second.ids.as_slice().contains(id))
        .collect();

    let first_points = first.points.as_slice();
    let second_points = second.points.as_slice();

    for first_edge in 0..first_points.len() {
        let a = first_points[first_edge];
        let b = first_points[(first_edge + 1) % first_points.len()];
        let first_ids = first.ids.as_slice();
        let edge_a = first_ids[first_edge];
        let edge_b = first_ids[(first_edge + 1) % first_ids.len()];

        for second_edge in 0..second_points.len() {
            let c = second_points[second_edge];
            let d = second_points[(second_edge + 1) % second_points.len()];
            let second_ids = second.ids.as_slice();
            let edge_c = second_ids[second_edge];
            let edge_d = second_ids[(second_edge + 1) % second_ids.len()];

            if !segments_intersect(a, b, c, d, tolerance) {
                continue;
            }
            if same_undirected_edge(edge_a, edge_b, edge_c, edge_d) {
                continue;
            }
            if shared.iter().copied().any(|shared_id| {
                edge_only_touches_at_shared_vertex(
                    edge_a, edge_b, a, b,
                    edge_c, edge_d, c, d,
                    shared_id, tolerance,
                )
            }) {
                continue;
            }
            return Err(EmbeddingError::NonAdjacentFaceIntersection {
                first_face: first.index,
                second_face: second.index,
            });
        }
    }

    // With no illegal edge crossings, strict containment is the remaining
    // way two convex faces can overlap in their interiors.
    for (index, point) in first_points.iter().enumerate() {
        if !shared.contains(&first.ids.as_slice()[index])
            && point_in_convex_polygon(*point, second_points, tolerance)
        {
            return Err(EmbeddingError::FaceInteriorOverlap {
                first_face: first.index,
                second_face: second.index,
            });
        }
    }
    for (index, point) in second_points.iter().enumerate() {
        if !shared.contains(&second.ids.as_slice()[index])
            && point_in_convex_polygon(*point, first_points, tolerance)
        {
            return Err(EmbeddingError::FaceInteriorOverlap {
                first_face: first.index,
                second_face: second.index,
            });
        }
    }

    // A shared-vertex-only pair can overlap as two wedges without a
    // non-shared vertex strictly inside the other polygon. Test interior
    // representatives as a final containment guard.
    let first_center = polygon_centroid(first_points);
    let second_center = polygon_centroid(second_points);
    if point_in_convex_polygon(first_center, second_points, tolerance)
        || point_in_convex_polygon(second_center, first_points, tolerance)
    {
        return Err(EmbeddingError::FaceInteriorOverlap {
            first_face: first.index,
            second_face: second.index,
        });
    }
    Ok(())
}

fn same_undirected_edge(
    a: ControlPointId, b: ControlPointId,
    c: ControlPointId, d: ControlPointId,
) -> bool {
    (a == c && b == d) || (a == d && b == c)
}

fn edge_only_touches_at_shared_vertex(
    a_id: ControlPointId, b_id: ControlPointId,
    a: ControlPointPosition, b: ControlPointPosition,
    c_id: ControlPointId, d_id: ControlPointId,
    c: ControlPointPosition, d: ControlPointPosition,
    shared_id: ControlPointId,
    tolerance: EmbeddingTolerance,
) -> bool {
    let first_other = if a_id == shared_id { b } else if b_id == shared_id { a } else { return false };
    let second_other = if c_id == shared_id { d } else if d_id == shared_id { c } else { return false };
    let shared = if a_id == shared_id { a } else { b };
    let first_cross = cross(shared, first_other, second_other).abs();
    if first_cross > tolerance.area {
        return true;
    }
    // Collinear rays are permitted only when they point in opposite
    // directions; same-ray segments overlap beyond the shared vertex.
    let first_vector = (first_other.x - shared.x, first_other.y - shared.y);
    let second_vector = (second_other.x - shared.x, second_other.y - shared.y);
    first_vector.0 * second_vector.0 + first_vector.1 * second_vector.1
        < -tolerance.spatial * tolerance.spatial
}

fn segments_intersect(
    a: ControlPointPosition, b: ControlPointPosition,
    c: ControlPointPosition, d: ControlPointPosition,
    tolerance: EmbeddingTolerance,
) -> bool {
    if a.x.max(b.x) + tolerance.spatial < c.x.min(d.x)
        || c.x.max(d.x) + tolerance.spatial < a.x.min(b.x)
        || a.y.max(b.y) + tolerance.spatial < c.y.min(d.y)
        || c.y.max(d.y) + tolerance.spatial < a.y.min(b.y)
    {
        return false;
    }
    let (o1, o2, o3, o4) = (
        cross(a, b, c), cross(a, b, d), cross(c, d, a), cross(c, d, b),
    );
    if ((o1 > tolerance.area && o2 < -tolerance.area)
        || (o1 < -tolerance.area && o2 > tolerance.area))
        && ((o3 > tolerance.area && o4 < -tolerance.area)
            || (o3 < -tolerance.area && o4 > tolerance.area))
    {
        return true;
    }
    point_on_segment(c, a, b, tolerance)
        || point_on_segment(d, a, b, tolerance)
        || point_on_segment(a, c, d, tolerance)
        || point_on_segment(b, c, d, tolerance)
}

fn point_on_segment(
    point: ControlPointPosition,
    a: ControlPointPosition,
    b: ControlPointPosition,
    tolerance: EmbeddingTolerance,
) -> bool {
    cross(a, b, point).abs() <= tolerance.area
        && point.x >= a.x.min(b.x) - tolerance.spatial
        && point.x <= a.x.max(b.x) + tolerance.spatial
        && point.y >= a.y.min(b.y) - tolerance.spatial
        && point.y <= a.y.max(b.y) + tolerance.spatial
}

fn point_in_convex_polygon(
    point: ControlPointPosition,
    polygon: &[ControlPointPosition],
    tolerance: EmbeddingTolerance,
) -> bool {
    let mut positive = false;
    let mut negative = false;
    for index in 0..polygon.len() {
        let value = cross(polygon[index], polygon[(index + 1) % polygon.len()], point);
        if value > tolerance.area { positive = true; }
        if value < -tolerance.area { negative = true; }
    }
    !(positive && negative)
}

fn polygon_centroid(points: &[ControlPointPosition]) -> ControlPointPosition {
    let count = points.len() as f64;
    ControlPointPosition { 
        x: points.iter().map(|point| point.x).sum::<f64>() / count,
        y: points.iter().map(|point| point.y).sum::<f64>() / count,
    }
}

fn signed_area_twice(points: &[ControlPointPosition]) -> f64 {
    (0..points.len()).map(|index| {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        a.x * b.y - b.x * a.y
    }).sum()
}

fn cross(
    a: ControlPointPosition,
    b: ControlPointPosition,
    c: ControlPointPosition,
) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn distance(a: ControlPointPosition, b: ControlPointPosition) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

fn geometry_fingerprint(
    topology: &DeformationTopology,
    control_state: &DeformationControlState,
) -> Result<[u8; 32], EmbeddingError> {
    let mut points: Vec<_> = control_state.points.iter().collect();
    points.sort_by_key(|point| point.id);
    let mut hasher = Sha256::new();
    hasher.update(b"FORMA-DEFORM\0CAGE-EMBEDDING\0V1");
    hasher.update(topology.topology_id().get().to_le_bytes());
    hasher.update(topology.topology_revision().get().to_le_bytes());
    hasher.update(control_state.revision.get().to_le_bytes());
    hasher.update((points.len() as u64).to_le_bytes());
    for point in points {
        if !point.position.x.is_finite() || !point.position.y.is_finite() {
            return Err(EmbeddingError::NonFiniteCoordinate(point.id.get()));
        }
        hasher.update(point.id.get().to_le_bytes());
        hasher.update(canonical_f64(point.position.x).to_le_bytes());
        hasher.update(canonical_f64(point.position.y).to_le_bytes());
    }
    hasher.update((topology.face_count() as u64).to_le_bytes());
    for index in 0..topology.face_count() {
        let face = topology.face_at(index).ok_or(EmbeddingError::InvalidFace(index))?;
        match face {
            TopologyFace::Triangle(ids) => {
                hasher.update([0x03]);
                for id in ids { hasher.update(id.get().to_le_bytes()); }
            }
            TopologyFace::Quad(ids) => {
                hasher.update([0x04]);
                for id in ids { hasher.update(id.get().to_le_bytes()); }
            }
        }
    }
    Ok(hasher.finalize().into())
}

fn canonical_f64(value: f64) -> u64 {
    if value == 0.0 { 0.0_f64.to_bits() } else { value.to_bits() }
}

impl std::fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TopologyMismatch => write!(f, "control state references a different topology identity or revision"),
            Self::InvalidTopology(message) => write!(f, "invalid cage topology or control state: {message}"),
            Self::TooManyPoints(count) => write!(f, "cage has {count} points; maximum is {MAX_EMBEDDING_POINTS}"),
            Self::TooManyFaces(count) => write!(f, "cage has {count} faces; maximum is {MAX_EMBEDDING_FACES}"),
            Self::EmptyGeometry => write!(f, "cage must contain control points and faces"),
            Self::DegenerateBounds => write!(f, "cage bounds must have finite positive diagonal"),
            Self::CoordinateRangeExceeded => write!(f, "cage coordinates exceed the supported numeric range"),
            Self::MissingControlPoint(id) => write!(f, "control point {id} is missing"),
            Self::DuplicateControlPoint(id) => write!(f, "control point {id} occurs more than once"),
            Self::CoincidentControlPoints(a, b) => write!(f, "control points at storage positions {a} and {b} are coincident or too close"),
            Self::NonFiniteCoordinate(id) => write!(f, "control point {id} has a non-finite coordinate"),
            Self::InvalidFace(index) => write!(f, "face {index} is invalid"),
            Self::DegenerateFace(index) => write!(f, "face {index} is degenerate"),
            Self::InconsistentFaceWinding(index) => write!(f, "face {index} has inconsistent winding"),
            Self::NonConvexQuad(index) => write!(f, "quad face {index} is not strictly convex"),
            Self::NonManifoldEdge(a, b) => write!(f, "edge ({a}, {b}) is incident to more than two faces"),
            Self::InvalidSharedEdgeWinding(a, b) => write!(f, "faces {a} and {b} traverse their shared edge in the same direction"),
            Self::DisconnectedFaces(index) => write!(f, "face {index} is disconnected from the cage"),
            Self::InvalidBoundary => write!(f, "cage has an invalid boundary"),
            Self::CandidateLimitExceeded => write!(f, "global overlap validation exceeded its bounded candidate budget"),
            Self::NonAdjacentFaceIntersection { first_face, second_face } => write!(f, "faces {first_face} and {second_face} intersect beyond their declared shared topology"),
            Self::FaceInteriorOverlap { first_face, second_face } => write!(f, "faces {first_face} and {second_face} overlap in their interiors"),
        }
    }
}

impl std::error::Error for EmbeddingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{
        ControlPointState, ControlStateRevision, DeformationTopologyId,
        DeformationTopologyRevision, IrregularCageTopology,
    };

    fn id(value: u64) -> ControlPointId { ControlPointId::new(value).unwrap() }
    fn point(x: f64, y: f64) -> ControlPointPosition {
        ControlPointPosition::new(x, y).unwrap()
    }

    fn two_triangle_cage(
        positions: [(f64, f64); 4],
    ) -> (DeformationTopology, DeformationControlState) {
        let topology_id = DeformationTopologyId::new(1).unwrap();
        let topology_revision = DeformationTopologyRevision::new(1).unwrap();
        let topology = IrregularCageTopology {
            id: topology_id,
            revision: topology_revision,
            control_point_ids: vec![id(1), id(2), id(3), id(4)],
            faces: vec![
                TopologyFace::Triangle([id(1), id(2), id(3)]),
                TopologyFace::Triangle([id(3), id(2), id(4)]),
            ],
        };
        let state = DeformationControlState {
            topology_id,
            topology_revision,
            revision: ControlStateRevision::new(1).unwrap(),
            points: positions.iter().enumerate().map(|(index, (x, y))| {
                ControlPointState {
                    id: id(index as u64 + 1),
                    position: point(*x, *y),
                    pinned: false,
                }
            }).collect(),
        };
        (DeformationTopology::IrregularCage(topology), state)
    }

    #[test]
    fn adjacent_triangles_receive_a_revision_bound_certificate() {
        let (topology, state) = two_triangle_cage([
            (0.0, 0.0), (2.0, 0.0), (0.0, 2.0), (2.0, 2.0),
        ]);
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        assert!(certificate.matches(&topology, &state));
    }

    #[test]
    fn overlapping_non_adjacent_faces_are_rejected() {
        let topology_id = DeformationTopologyId::new(2).unwrap();
        let topology_revision = DeformationTopologyRevision::new(1).unwrap();
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: topology_id,
            revision: topology_revision,
            control_point_ids: (1..=5).map(id).collect(),
            faces: vec![
                TopologyFace::Triangle([id(1), id(2), id(3)]),
                TopologyFace::Triangle([id(1), id(3), id(4)]),
                TopologyFace::Triangle([id(1), id(4), id(5)]),
            ],
        });
        let state = DeformationControlState {
            topology_id,
            topology_revision,
            revision: ControlStateRevision::new(1).unwrap(),
            points: [
                (0.0, 0.0),
                (2.0, 0.0),
                (-1.0, 1.732),
                (-1.0, -1.732),
                (1.732, 1.0),
            ].iter().enumerate().map(|(index, (x, y))| ControlPointState {
                id: id(index as u64 + 1),
                position: point(*x, *y),
                pinned: false,
            }).collect(),
        };

        assert!(matches!(
            validate_cage_embedding(&topology, &state),
            Err(EmbeddingError::NonAdjacentFaceIntersection { .. })
                | Err(EmbeddingError::FaceInteriorOverlap { .. })
        ));
    }

    #[test]
    fn certificate_is_invalidated_by_control_state_revision_or_geometry() {
        let (topology, state) = two_triangle_cage([
            (0.0, 0.0), (2.0, 0.0), (0.0, 2.0), (2.0, 2.0),
        ]);
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        let mut changed_revision = state.clone();
        changed_revision.revision = ControlStateRevision::new(2).unwrap();
        assert!(!certificate.matches(&topology, &changed_revision));

        let mut changed_geometry = state.clone();
        changed_geometry.points[0].position = point(-0.25, 0.0);
        assert!(!certificate.matches(&topology, &changed_geometry));
    }
}
