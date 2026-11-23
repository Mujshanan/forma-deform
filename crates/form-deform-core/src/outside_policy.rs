//! Explicit resolution contracts for source points outside a certified cage.
//!
//! This module is intentionally separate from point location: inside-cage
//! ownership remains governed by FDM-03.2. Outside resolution never changes
//! the input point or silently converts an outside point into an inside hit.

use crate::binding::{FaceCoordinates, ValidatedCageGeometry};
use crate::path2d::Point2;
use crate::topology::{ControlPointId, TopologyFace};
use std::collections::BTreeMap;

pub const OUTSIDE_CAGE_POLICY_VERSION: u32 = 1;
const DISTANCE_TIE_RELATIVE_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutsideCageResolutionPolicy {
    /// Preserve the historical behavior: an outside source feature is unbound.
    UnboundZeroInfluence,
    /// Project to the nearest topological boundary segment.
    ClampToBoundary,
    /// Preserve the offset from the nearest boundary, but only within this
    /// explicit source-space distance budget.
    BoundedExtrapolation { max_distance: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryProjection {
    pub face_index: u32,
    pub edge_index: u8,
    pub control_point_ids: [ControlPointId; 2],
    pub coordinates: FaceCoordinates,
    pub point: Point2,
    pub distance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutsideCageResolution {
    Unbound,
    Clamped(BoundaryProjection),
    Extrapolated {
        boundary: BoundaryProjection,
        /// Source-space displacement from the projected boundary point.
        /// This is retained explicitly; it is not folded into boundary
        /// coordinates or mistaken for an inside-face mapping.
        offset: [f64; 2],
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutsideCageError {
    InvalidDistanceLimit,
    FaceIndexExceedsLocatorCapacity(usize),
    MissingControlPoint(ControlPointId),
    InvalidBoundaryGeometry,
    DistanceLimitExceeded { distance: f64, maximum: f64 },
    NoBoundary,
}

impl std::fmt::Display for OutsideCageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDistanceLimit => write!(f, "bounded extrapolation requires a finite positive distance limit"),
            Self::FaceIndexExceedsLocatorCapacity(index) => write!(f, "boundary face index {index} exceeds locator capacity"),
            Self::MissingControlPoint(id) => write!(f, "certified cage geometry is missing control point {}", id.get()),
            Self::InvalidBoundaryGeometry => write!(f, "cage boundary contains invalid or degenerate geometry"),
            Self::DistanceLimitExceeded { distance, maximum } => write!(f, "outside point distance {distance} exceeds extrapolation limit {maximum}"),
            Self::NoBoundary => write!(f, "certified cage has no projectable boundary"),
        }
    }
}
impl std::error::Error for OutsideCageError {}

#[derive(Clone, Copy)]
struct EdgeCandidate {
    face_index: usize,
    edge_index: u8,
    ids: [ControlPointId; 2],
    start: [f64; 2],
    end: [f64; 2],
    start_coordinates: FaceCoordinates,
    end_coordinates: FaceCoordinates,
}

#[derive(Clone, Copy)]
struct ProjectionCandidate {
    edge: EdgeCandidate,
    t: f64,
    point: [f64; 2],
    distance: f64,
}

/// Resolve an already-classified outside point. Callers should first invoke
/// FDM-03.2; this function is not an alternate point locator.
///
/// Boundary edges are derived from face incidence (exactly one incident face).
/// Equal-distance ties are deterministic by face index, then authored edge
/// index. No nearest-interior-face fallback is performed.
pub fn resolve_outside_point(
    point: Point2,
    geometry: &ValidatedCageGeometry,
    policy: OutsideCageResolutionPolicy,
) -> Result<OutsideCageResolution, OutsideCageError> {
    match policy {
        OutsideCageResolutionPolicy::UnboundZeroInfluence => Ok(OutsideCageResolution::Unbound),
        OutsideCageResolutionPolicy::ClampToBoundary => {
            let projection = nearest_boundary_projection(point, geometry)?;
            Ok(OutsideCageResolution::Clamped(projection))
        }
        OutsideCageResolutionPolicy::BoundedExtrapolation { max_distance } => {
            if !max_distance.is_finite() || max_distance <= 0.0 {
                return Err(OutsideCageError::InvalidDistanceLimit);
            }
            let boundary = nearest_boundary_projection(point, geometry)?;
            if boundary.distance > max_distance {
                return Err(OutsideCageError::DistanceLimitExceeded {
                    distance: boundary.distance,
                    maximum: max_distance,
                });
            }
            let offset = [
                point.x() - boundary.point.x(),
                point.y() - boundary.point.y(),
            ];
            if offset.iter().any(|value| !value.is_finite()) {
                return Err(OutsideCageError::InvalidBoundaryGeometry);
            }
            Ok(OutsideCageResolution::Extrapolated { boundary, offset })
        }
    }
}

fn nearest_boundary_projection(
    point: Point2,
    geometry: &ValidatedCageGeometry,
) -> Result<BoundaryProjection, OutsideCageError> {
    let mut incidence: BTreeMap<(ControlPointId, ControlPointId), Vec<EdgeCandidate>> = BTreeMap::new();
    for face_index in 0..geometry.face_count() {
        let face = geometry.face(face_index).ok_or(OutsideCageError::InvalidBoundaryGeometry)?;
        let ids = face.control_point_ids();
        let local_edges: &[(usize, usize)] = match face {
            TopologyFace::Triangle(_) => &[(0, 1), (1, 2), (2, 0)],
            TopologyFace::Quad(_) => &[(0, 1), (1, 2), (2, 3), (3, 0)],
        };
        if face_index > u32::MAX as usize {
            return Err(OutsideCageError::FaceIndexExceedsLocatorCapacity(face_index));
        }
        for (edge_index, &(start_index, end_index)) in local_edges.iter().enumerate() {
            let start_id = ids[start_index];
            let end_id = ids[end_index];
            let start_position = geometry.position(start_id).ok_or(OutsideCageError::MissingControlPoint(start_id))?;
            let end_position = geometry.position(end_id).ok_or(OutsideCageError::MissingControlPoint(end_id))?;
            let start = [start_position.x, start_position.y];
            let end = [end_position.x, end_position.y];
            let start_coordinates = vertex_coordinates(face, start_index);
            let end_coordinates = vertex_coordinates(face, end_index);
            let key = if start_id < end_id { (start_id, end_id) } else { (end_id, start_id) };
            incidence.entry(key).or_default().push(EdgeCandidate {
                face_index, edge_index: edge_index as u8,
                ids: [start_id, end_id], start, end,
                start_coordinates, end_coordinates,
            });
        }
    }

    let query = [point.x(), point.y()];
    let mut best: Option<ProjectionCandidate> = None;
    for incident in incidence.values().filter(|incident| incident.len() == 1) {
        let edge = incident[0];
        let dx = edge.end[0] - edge.start[0];
        let dy = edge.end[1] - edge.start[1];
        let length_squared = dx * dx + dy * dy;
        if !length_squared.is_finite() || length_squared <= 0.0 {
            return Err(OutsideCageError::InvalidBoundaryGeometry);
        }
        let raw_t = ((query[0] - edge.start[0]) * dx + (query[1] - edge.start[1]) * dy) / length_squared;
        if !raw_t.is_finite() { return Err(OutsideCageError::InvalidBoundaryGeometry); }
        let t = raw_t.clamp(0.0, 1.0);
        let projected = [edge.start[0] + t * dx, edge.start[1] + t * dy];
        let distance = (query[0] - projected[0]).hypot(query[1] - projected[1]);
        if !distance.is_finite() { return Err(OutsideCageError::InvalidBoundaryGeometry); }
        let candidate = ProjectionCandidate { edge, t, point: projected, distance };
        if best.map_or(true, |current| projection_precedes(candidate, current)) {
            best = Some(candidate);
        }
    }
    let best = best.ok_or(OutsideCageError::NoBoundary)?;
    let coordinates = interpolate_coordinates(
        best.edge.start_coordinates,
        best.edge.end_coordinates,
        best.t,
    )?;
    let projected_point = Point2::new(best.point[0], best.point[1])
        .map_err(|_| OutsideCageError::InvalidBoundaryGeometry)?;
    Ok(BoundaryProjection {
        face_index: best.edge.face_index as u32,
        edge_index: best.edge.edge_index,
        control_point_ids: best.edge.ids,
        coordinates,
        point: projected_point,
        distance: best.distance,
    })
}

fn projection_precedes(candidate: ProjectionCandidate, current: ProjectionCandidate) -> bool {
    let scale = candidate.distance.abs().max(current.distance.abs()).max(f64::MIN_POSITIVE);
    let tie_tolerance = scale * DISTANCE_TIE_RELATIVE_TOLERANCE;
    if candidate.distance < current.distance - tie_tolerance { return true; }
    if (candidate.distance - current.distance).abs() > tie_tolerance { return false; }
    (candidate.edge.face_index, candidate.edge.edge_index)
        < (current.edge.face_index, current.edge.edge_index)
}

fn vertex_coordinates(face: TopologyFace, index: usize) -> FaceCoordinates {
    match face {
        TopologyFace::Triangle(_) => {
            let mut weights = [0.0; 3];
            weights[index] = 1.0;
            FaceCoordinates::TriangleBarycentric { weights }
        }
        TopologyFace::Quad(_) => {
            let (u, v) = match index {
                0 => (0.0, 0.0), // P00
                1 => (1.0, 0.0), // P10
                2 => (1.0, 1.0), // P11
                _ => (0.0, 1.0), // P01
            };
            FaceCoordinates::QuadBilinear { u, v }
        }
    }
}

fn interpolate_coordinates(
    start: FaceCoordinates,
    end: FaceCoordinates,
    t: f64,
) -> Result<FaceCoordinates, OutsideCageError> {
    match (start, end) {
        (FaceCoordinates::TriangleBarycentric { weights: a }, FaceCoordinates::TriangleBarycentric { weights: b }) => {
            let weights = [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t);
            if weights.iter().any(|weight| !weight.is_finite()) {
                return Err(OutsideCageError::InvalidBoundaryGeometry);
            }
            Ok(FaceCoordinates::TriangleBarycentric { weights })
        }
        (FaceCoordinates::QuadBilinear { u: au, v: av }, FaceCoordinates::QuadBilinear { u: bu, v: bv }) => {
            let u = au + (bu - au) * t;
            let v = av + (bv - av) * t;
            if !u.is_finite() || !v.is_finite() {
                return Err(OutsideCageError::InvalidBoundaryGeometry);
            }
            Ok(FaceCoordinates::QuadBilinear { u, v })
        }
        _ => Err(OutsideCageError::InvalidBoundaryGeometry),
    }
}
