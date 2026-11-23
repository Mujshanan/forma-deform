//! Explicit resolution contracts for source points outside a certified cage.
//!
//! This module is intentionally separate from point location: inside-cage
//! ownership remains governed by FDM-03.2. Outside resolution never changes
//! the input point or silently converts an outside point into an inside hit.

use crate::binding::{FaceCoordinates, ValidatedCageGeometry};
pub use crate::binding::OutsideCagePolicy as OutsideCageResolutionPolicy;
use crate::path2d::Point2;
use crate::topology::{ControlPointId, TopologyFace, TopologyView};
use std::collections::BTreeMap;

pub const OUTSIDE_CAGE_POLICY_VERSION: u32 = 1;
const DISTANCE_TIE_RELATIVE_TOLERANCE: f64 = 1.0e-12;


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
        coordinates: FaceCoordinates,
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
            let coordinates = extrapolated_face_coordinates(point, geometry, boundary.face_index as usize)?;
            if offset.iter().any(|value| !value.is_finite()) {
                return Err(OutsideCageError::InvalidBoundaryGeometry);
            }
            Ok(OutsideCageResolution::Extrapolated { boundary, coordinates, offset })
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


/// Invert the owning face's mapping at an outside point. Triangle coordinates
/// are affine barycentric weights. Quad coordinates solve the full bilinear
/// inverse; no diagonal triangulation or nearest-vertex approximation is used.
fn extrapolated_face_coordinates(
    point: Point2,
    geometry: &ValidatedCageGeometry,
    face_index: usize,
) -> Result<FaceCoordinates, OutsideCageError> {
    let face = geometry.face(face_index).ok_or(OutsideCageError::InvalidBoundaryGeometry)?;
    let ids = face.control_point_ids();
    let points: Vec<[f64; 2]> = ids.iter().map(|id| {
        geometry.position(*id).map(|p| [p.x, p.y]).ok_or(OutsideCageError::MissingControlPoint(*id))
    }).collect::<Result<_, _>>()?;
    let q = [point.x(), point.y()];
    match face {
        TopologyFace::Triangle(_) => {
            let a=points[0]; let b=points[1]; let c=points[2];
            let ab=[b[0]-a[0],b[1]-a[1]];
            let ac=[c[0]-a[0],c[1]-a[1]];
            let aq=[q[0]-a[0],q[1]-a[1]];
            let det=ab[0]*ac[1]-ab[1]*ac[0];
            let scale=ab[0].hypot(ab[1]).max(ac[0].hypot(ac[1])).max((c[0]-b[0]).hypot(c[1]-b[1]));
            if !det.is_finite() || !scale.is_finite() || scale<=0.0
                || det.abs()<=f64::EPSILON*64.0*scale*scale {
                return Err(OutsideCageError::InvalidBoundaryGeometry);
            }
            let wb=(aq[0]*ac[1]-aq[1]*ac[0])/det;
            let wc=(ab[0]*aq[1]-ab[1]*aq[0])/det;
            let wa=1.0-wb-wc;
            if [wa,wb,wc].iter().any(|v| !v.is_finite()) {
                return Err(OutsideCageError::InvalidBoundaryGeometry);
            }
            Ok(FaceCoordinates::TriangleBarycentric { weights:[wa,wb,wc] })
        }
        TopologyFace::Quad(_) => inverse_bilinear_extrapolated(&points, q),
    }
}

fn inverse_bilinear_extrapolated(
    points: &[[f64;2]],
    query: [f64;2],
) -> Result<FaceCoordinates, OutsideCageError> {
    // P(u,v) = a + b*u + c*v + d*u*v, authored order P00,P10,P11,P01.
    let a=points[0];
    let b=[points[1][0]-a[0],points[1][1]-a[1]];
    let c=[points[3][0]-a[0],points[3][1]-a[1]];
    let d=[a[0]-points[1][0]+points[2][0]-points[3][0],
           a[1]-points[1][1]+points[2][1]-points[3][1]];
    let q=[query[0]-a[0],query[1]-a[1]];
    let cross=|x:[f64;2],y:[f64;2]| x[0]*y[1]-x[1]*y[0];
    let dot=|x:[f64;2],y:[f64;2]| x[0]*y[0]+x[1]*y[1];
    let aa=-cross(b,d);
    let bb=cross(q,d)-cross(b,c);
    let cc=cross(q,c);
    let coeff=aa.abs().max(bb.abs()).max(cc.abs());
    if !coeff.is_finite() || coeff<=f64::MIN_POSITIVE {
        return Err(OutsideCageError::InvalidBoundaryGeometry);
    }
    let tol=f64::EPSILON*64.0*coeff;
    let roots: Vec<f64>;
    if aa.abs()<=tol {
        if bb.abs()<=tol { return Err(OutsideCageError::InvalidBoundaryGeometry); }
        roots=vec![-cc/bb];
    } else {
        let disc=bb*bb-4.0*aa*cc;
        let disc_scale=(bb*bb).abs().max((4.0*aa*cc).abs()).max(f64::MIN_POSITIVE);
        let disc_tol=f64::EPSILON*64.0*disc_scale;
        if !disc.is_finite() || disc < -disc_tol { return Err(OutsideCageError::InvalidBoundaryGeometry); }
        let root=disc.max(0.0).sqrt();
        if root==0.0 { roots=vec![-bb/(2.0*aa)]; }
        else {
            let stable=-0.5*(bb+bb.signum()*root);
            if stable==0.0 || !stable.is_finite() { return Err(OutsideCageError::InvalidBoundaryGeometry); }
            roots=vec![stable/aa,cc/stable];
        }
    }
    let mut solutions=Vec::new();
    let scale=points.iter().enumerate().flat_map(|(i,p)| points.iter().skip(i+1).map(move |r| {
        (p[0]-r[0]).hypot(p[1]-r[1])
    })).fold(0.0_f64,f64::max);
    if !scale.is_finite() || scale<=0.0 { return Err(OutsideCageError::InvalidBoundaryGeometry); }
    for u in roots {
        if !u.is_finite() { return Err(OutsideCageError::InvalidBoundaryGeometry); }
        let axis=[c[0]+d[0]*u,c[1]+d[1]*u];
        let denominator=dot(axis,axis);
        if !denominator.is_finite() || denominator<=f64::EPSILON*64.0*scale*scale {
            return Err(OutsideCageError::InvalidBoundaryGeometry);
        }
        let residual=[q[0]-b[0]*u,q[1]-b[1]*u];
        let v=dot(residual,axis)/denominator;
        if !v.is_finite() { return Err(OutsideCageError::InvalidBoundaryGeometry); }
        // Reject a locally singular inverse even if reconstruction happens to
        // produce a small residual at floating-point precision.
        let du=[b[0]+d[0]*v,b[1]+d[1]*v];
        let jacobian_determinant=cross(du,axis);
        let jacobian_scale=(dot(du,du)*denominator).sqrt();
        if !jacobian_determinant.is_finite() || !jacobian_scale.is_finite()
            || jacobian_scale<=0.0
            || jacobian_determinant.abs()<=f64::EPSILON*64.0*jacobian_scale {
            return Err(OutsideCageError::InvalidBoundaryGeometry);
        }
        let reconstructed=[
            a[0]+b[0]*u+c[0]*v+d[0]*u*v,
            a[1]+b[1]*u+c[1]*v+d[1]*u*v,
        ];
        if !reconstructed.iter().all(|x| x.is_finite()) { return Err(OutsideCageError::InvalidBoundaryGeometry); }
        let error=(reconstructed[0]-query[0]).hypot(reconstructed[1]-query[1]);
        if error<=scale*1.0e-9 && solutions.iter().all(|(old_u,old_v):&(f64,f64)| {
            (old_u-u).abs()>1.0e-9 || (old_v-v).abs()>1.0e-9
        }) { solutions.push((u,v)); }
    }
    if solutions.len()!=1 { return Err(OutsideCageError::InvalidBoundaryGeometry); }
    let (u,v)=solutions[0];
    Ok(FaceCoordinates::QuadBilinear { u,v })
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::validate_cage_embedding;
    use crate::topology::{
        ControlPointState, ControlStateRevision, DeformationControlState,
        DeformationTopology, DeformationTopologyId, DeformationTopologyRevision,
        IrregularCageTopology, TopologyFace,
    };

    fn id(value:u64)->ControlPointId { ControlPointId::new(value).unwrap() }
    fn position(x:f64,y:f64)->crate::topology::ControlPointPosition {
        crate::topology::ControlPointPosition::new(x,y).unwrap()
    }
    fn point(x:f64,y:f64)->Point2 { Point2::new(x,y).unwrap() }

    fn certified_face(face:TopologyFace, coordinates:&[(u64,f64,f64)]) -> ValidatedCageGeometry {
        let topology=DeformationTopology::IrregularCage(IrregularCageTopology {
            id:DeformationTopologyId::new(91).unwrap(),
            revision:DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids:coordinates.iter().map(|(key,_,_)|id(*key)).collect(),
            faces:vec![face],
        });
        let state=DeformationControlState {
            topology_id:topology.topology_id(),
            topology_revision:topology.topology_revision(),
            revision:ControlStateRevision::new(1).unwrap(),
            points:coordinates.iter().map(|(key,x,y)|ControlPointState {
                id:id(*key),position:position(*x,*y),pinned:false,
            }).collect(),
        };
        let certificate=validate_cage_embedding(&topology,&state).unwrap();
        ValidatedCageGeometry::new(&topology,&state,&certificate).unwrap()
    }

    #[test]
    fn triangle_extrapolation_returns_unbounded_barycentric_coordinates_within_limit() {
        let geometry=certified_face(
            TopologyFace::Triangle([id(1),id(2),id(3)]),
            &[(1,0.0,0.0),(2,2.0,0.0),(3,0.0,2.0)],
        );
        let result=resolve_outside_point(point(0.5,-0.5),&geometry,
            OutsideCageResolutionPolicy::BoundedExtrapolation{max_distance:0.6}).unwrap();
        let OutsideCageResolution::Extrapolated{boundary,coordinates,offset}=result else {
            panic!("expected bounded extrapolation");
        };
        assert_eq!(boundary.point,point(0.5,0.0));
        assert!((boundary.distance-0.5).abs()<1.0e-12);
        assert_eq!(offset,[0.0,-0.5]);
        assert_eq!(coordinates,FaceCoordinates::TriangleBarycentric{weights:[1.0,0.25,-0.25]});
    }

    #[test]
    fn quad_extrapolation_inverts_the_bilinear_patch_outside_parameter_domain() {
        let geometry=certified_face(
            TopologyFace::Quad([id(1),id(2),id(3),id(4)]),
            &[(1,0.0,0.0),(2,2.0,0.0),(3,2.0,2.0),(4,0.0,2.0)],
        );
        let result=resolve_outside_point(point(1.0,-0.5),&geometry,
            OutsideCageResolutionPolicy::BoundedExtrapolation{max_distance:0.6}).unwrap();
        let OutsideCageResolution::Extrapolated{coordinates,..}=result else {
            panic!("expected bounded extrapolation");
        };
        assert_eq!(coordinates,FaceCoordinates::QuadBilinear{u:0.5,v:-0.25});
    }

    #[test]
    fn bounded_extrapolation_rejects_points_beyond_the_distance_budget() {
        let geometry=certified_face(
            TopologyFace::Triangle([id(1),id(2),id(3)]),
            &[(1,0.0,0.0),(2,2.0,0.0),(3,0.0,2.0)],
        );
        assert!(matches!(resolve_outside_point(point(0.5,-0.5),&geometry,
            OutsideCageResolutionPolicy::BoundedExtrapolation{max_distance:0.4}),
            Err(OutsideCageError::DistanceLimitExceeded{..})));
    }
}
