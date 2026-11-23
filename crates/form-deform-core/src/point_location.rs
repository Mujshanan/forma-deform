//! Deterministic point location against a globally certified cage embedding.
//!
//! Every face is considered. First-hit selection is deliberately avoided:
//! multiple candidates are accepted only when their faces share the exact
//! topological boundary containing the query point.

use crate::binding::{FaceCoordinates, ValidatedCageGeometry};
use crate::path2d::Point2;
use crate::topology::{ControlPointId, ControlPointPosition, TopologyFace};

pub const POINT_LOCATION_POLICY_VERSION: u32 = 1;
const PARAMETER_TOLERANCE: f64 = 1.0e-9;
const RELATIVE_SPATIAL_TOLERANCE: f64 = 1.0e-9;
const MIN_SPATIAL_TOLERANCE: f64 = 1.0e-12;
const FLOAT_EPSILON_FACTOR: f64 = 64.0;

#[derive(Clone, Debug, PartialEq)]
pub struct LocatedFace {
    pub face_index: u32,
    pub subface_index: u8,
    pub control_point_ids: Vec<ControlPointId>,
    pub coordinates: FaceCoordinates,
}

/// Stable policy for choosing among valid faces when a point lies on shared topology.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointLocationTieBreak {
    /// Preserve the Path2D binding policy: lowest deterministic topology face index.
    LowestFaceIndex,
    /// Generic point attachments choose the lowest sorted stable vertex-ID support.
    LowestStableVertexIds,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PointLocationOutcome {
    Located(LocatedFace),
    OutsideCage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PointLocationError {
    FaceIndexExceedsLocatorCapacity(usize),
    MissingControlPoint(ControlPointId),
    InvalidFaceGeometry(usize),
    DegenerateTriangle(usize),
    SingularBilinearInverse(usize),
    AmbiguousBilinearInverse(usize),
    NonFiniteIntermediate(usize),
    AmbiguousFaceOwnership { first_face: usize, second_face: usize },
    InvalidCandidateBoundary { first_face: usize, second_face: usize },
}

impl std::fmt::Display for PointLocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FaceIndexExceedsLocatorCapacity(index) => write!(f, "face index {index} exceeds the binding locator capacity"),
            Self::MissingControlPoint(id) => write!(f, "certified geometry is missing control point {}", id.get()),
            Self::InvalidFaceGeometry(index) => write!(f, "face {index} has invalid or non-finite geometry"),
            Self::DegenerateTriangle(index) => write!(f, "triangle face {index} is numerically degenerate"),
            Self::SingularBilinearInverse(index) => write!(f, "bilinear inverse for quad face {index} is singular or ill-conditioned"),
            Self::AmbiguousBilinearInverse(index) => write!(f, "bilinear inverse for quad face {index} has multiple accepted solutions"),
            Self::NonFiniteIntermediate(index) => write!(f, "point-location arithmetic became non-finite on face {index}"),
            Self::AmbiguousFaceOwnership { first_face, second_face } => write!(f, "point is covered by non-adjacent faces {first_face} and {second_face}"),
            Self::InvalidCandidateBoundary { first_face, second_face } => write!(f, "candidate faces {first_face} and {second_face} do not contain the point on their declared shared boundary"),
        }
    }
}
impl std::error::Error for PointLocationError {}

#[derive(Clone, Copy, Debug)]
struct Vec2 { x: f64, y: f64 }
impl Vec2 {
    fn from_position(point: ControlPointPosition) -> Self { Self { x: point.x, y: point.y } }
    fn from_query(point: Point2) -> Self { Self { x: point.x(), y: point.y() } }
    fn add(self, rhs: Self) -> Self { Self { x: self.x + rhs.x, y: self.y + rhs.y } }
    fn sub(self, rhs: Self) -> Self { Self { x: self.x - rhs.x, y: self.y - rhs.y } }
    fn scale(self, factor: f64) -> Self { Self { x: self.x * factor, y: self.y * factor } }
    fn dot(self, rhs: Self) -> f64 { self.x * rhs.x + self.y * rhs.y }
    fn cross(self, rhs: Self) -> f64 { self.x * rhs.y - self.y * rhs.x }
    fn length(self) -> f64 { self.x.hypot(self.y) }
    fn finite(self) -> bool { self.x.is_finite() && self.y.is_finite() }
}

#[derive(Clone, Debug)]
struct Candidate {
    located: LocatedFace,
    parameter_tolerance: f64,
}
impl Candidate {
    fn basis_weights(&self) -> Vec<f64> {
        match self.located.coordinates {
            FaceCoordinates::TriangleBarycentric { weights } => weights.to_vec(),
            FaceCoordinates::QuadBilinear { u, v } => vec![
                (1.0-u)*(1.0-v), u*(1.0-v), u*v, (1.0-u)*v,
            ],
        }
    }
}

/// Locate one source-space point in a certified cage. All candidate faces are
/// examined; stable iteration order never substitutes for ambiguity checks.
pub fn locate_point(
    point: Point2,
    geometry: &ValidatedCageGeometry,
) -> Result<PointLocationOutcome, PointLocationError> {
    locate_point_with_tie_break(point, geometry, PointLocationTieBreak::LowestFaceIndex)
}

/// Locate a point using an explicit shared-boundary ownership policy.
pub fn locate_point_with_tie_break(
    point: Point2,
    geometry: &ValidatedCageGeometry,
    tie_break: PointLocationTieBreak,
) -> Result<PointLocationOutcome, PointLocationError> {
    let mut candidates = Vec::new();
    for face_index in 0..geometry.face_count() {
        let face = geometry.face(face_index).ok_or(PointLocationError::InvalidFaceGeometry(face_index))?;
        let ids = face.control_point_ids();
        let mut points = Vec::with_capacity(ids.len());
        for id in ids {
            let position = geometry.position(*id).ok_or(PointLocationError::MissingControlPoint(*id))?;
            points.push(Vec2::from_position(position));
        }
        let query = Vec2::from_query(point);
        let located = match face {
            TopologyFace::Triangle(_) => locate_in_triangle(face_index, ids, &points, query)?,
            TopologyFace::Quad(_) => locate_in_quad(face_index, ids, &points, query)?,
        };
        if let Some((coordinates, parameter_tolerance)) = located {
            if face_index > u32::MAX as usize {
                return Err(PointLocationError::FaceIndexExceedsLocatorCapacity(face_index));
            }
            candidates.push(Candidate {
                located: LocatedFace {
                    face_index: face_index as u32,
                    subface_index: 0,
                    control_point_ids: ids.to_vec(),
                    coordinates,
                },
                parameter_tolerance,
            });
        }
    }

    if candidates.is_empty() { return Ok(PointLocationOutcome::OutsideCage); }
    match tie_break {
        PointLocationTieBreak::LowestFaceIndex => {
            candidates.sort_by_key(|candidate| candidate.located.face_index);
        }
        PointLocationTieBreak::LowestStableVertexIds => {
            candidates.sort_by(|left, right| {
                let mut left_ids = left.located.control_point_ids.clone();
                let mut right_ids = right.located.control_point_ids.clone();
                left_ids.sort_unstable();
                right_ids.sort_unstable();
                left_ids.cmp(&right_ids)
                    .then_with(|| left.located.face_index.cmp(&right.located.face_index))
            });
        }
    }

    for first in 0..candidates.len() {
        for second in (first + 1)..candidates.len() {
            let a = &candidates[first];
            let b = &candidates[second];
            let common: Vec<_> = a.located.control_point_ids.iter().copied()
                .filter(|id| b.located.control_point_ids.contains(id)).collect();
            if common.is_empty() {
                return Err(PointLocationError::AmbiguousFaceOwnership {
                    first_face: a.located.face_index as usize,
                    second_face: b.located.face_index as usize,
                });
            }
            if !candidate_is_on_shared_boundary(a, &common) || !candidate_is_on_shared_boundary(b, &common) {
                return Err(PointLocationError::InvalidCandidateBoundary {
                    first_face: a.located.face_index as usize,
                    second_face: b.located.face_index as usize,
                });
            }
        }
    }

    // Lowest topology face index wins, but only after all candidates are
    // proven to meet at their declared shared topological boundary.
    Ok(PointLocationOutcome::Located(candidates.remove(0).located))
}

fn candidate_is_on_shared_boundary(candidate: &Candidate, shared: &[ControlPointId]) -> bool {
    let weights = candidate.basis_weights();
    weights.len() == candidate.located.control_point_ids.len()
        && weights.iter().zip(&candidate.located.control_point_ids).all(|(weight, id)| {
            shared.contains(id) || weight.abs() <= candidate.parameter_tolerance
        })
}

fn locate_in_triangle(
    face_index: usize,
    ids: &[ControlPointId],
    points: &[Vec2],
    query: Vec2,
) -> Result<Option<(FaceCoordinates, f64)>, PointLocationError> {
    if points.len() != 3 || ids.len() != 3 || points.iter().any(|point| !point.finite()) || !query.finite() {
        return Err(PointLocationError::InvalidFaceGeometry(face_index));
    }
    let a = points[0];
    let b = points[1];
    let c = points[2];
    let ab = b.sub(a);
    let ac = c.sub(a);
    let scale = ab.length().max(ac.length()).max(c.sub(b).length());
    let determinant = ab.cross(ac);
    let determinant_tolerance = f64::EPSILON * FLOAT_EPSILON_FACTOR * scale * scale;
    if !scale.is_finite() || scale <= 0.0 || !determinant.is_finite() || determinant.abs() <= determinant_tolerance {
        return Err(PointLocationError::DegenerateTriangle(face_index));
    }

    let relative = query.sub(a);
    let weight_b = relative.cross(ac) / determinant;
    let weight_c = ab.cross(relative) / determinant;
    let weight_a = 1.0 - weight_b - weight_c;
    if !weight_a.is_finite() || !weight_b.is_finite() || !weight_c.is_finite() {
        return Err(PointLocationError::NonFiniteIntermediate(face_index));
    }
    let spatial = spatial_tolerance(points, face_index)?;
    let parameter_tolerance = (spatial / scale).max(PARAMETER_TOLERANCE);
    let mut weights = [weight_a, weight_b, weight_c];
    if weights.iter().any(|weight| *weight < -parameter_tolerance || *weight > 1.0 + parameter_tolerance) {
        return Ok(None);
    }
    for weight in &mut weights { *weight = weight.clamp(0.0, 1.0); }
    let sum = weights.iter().sum::<f64>();
    if !sum.is_finite() || sum <= 0.0 { return Err(PointLocationError::NonFiniteIntermediate(face_index)); }
    for weight in &mut weights { *weight /= sum; }
    let reconstructed = a.scale(weights[0]).add(b.scale(weights[1])).add(c.scale(weights[2]));
    if reconstructed.sub(query).length() > spatial { return Ok(None); }
    Ok(Some((FaceCoordinates::TriangleBarycentric { weights }, parameter_tolerance)))
}

fn locate_in_quad(
    face_index: usize,
    ids: &[ControlPointId],
    points: &[Vec2],
    query: Vec2,
) -> Result<Option<(FaceCoordinates, f64)>, PointLocationError> {
    if points.len() != 4 || ids.len() != 4 || points.iter().any(|point| !point.finite()) || !query.finite() {
        return Err(PointLocationError::InvalidFaceGeometry(face_index));
    }
    // P(u,v) = a + b*u + c*v + d*u*v, order P00,P10,P11,P01.
    let a = points[0];
    let b = points[1].sub(points[0]);
    let c = points[3].sub(points[0]);
    let d = points[0].sub(points[1]).add(points[2]).sub(points[3]);
    let q = query.sub(a);
    let scale = points.iter().enumerate().flat_map(|(i, p)| {
        points.iter().skip(i + 1).map(move |other| p.sub(*other).length())
    }).fold(0.0_f64, f64::max);
    if !scale.is_finite() || scale <= 0.0 || !b.finite() || !c.finite() || !d.finite() {
        return Err(PointLocationError::InvalidFaceGeometry(face_index));
    }
    let spatial = spatial_tolerance(points, face_index)?;
    let parameter_tolerance = (spatial / scale).max(PARAMETER_TOLERANCE);

    // Eliminate v using cross(q - b*u, c + d*u) = 0.
    let coefficient_a = -b.cross(d);
    let coefficient_b = q.cross(d) - b.cross(c);
    let coefficient_c = q.cross(c);
    let coefficient_scale = coefficient_a.abs().max(coefficient_b.abs()).max(coefficient_c.abs());
    if !coefficient_scale.is_finite() { return Err(PointLocationError::NonFiniteIntermediate(face_index)); }
    if coefficient_scale <= f64::MIN_POSITIVE { return Err(PointLocationError::SingularBilinearInverse(face_index)); }
    let coefficient_tolerance = f64::EPSILON * FLOAT_EPSILON_FACTOR * coefficient_scale;
    let roots = solve_quadratic(coefficient_a, coefficient_b, coefficient_c, coefficient_tolerance, face_index)?;

    let mut accepted: Vec<(f64, f64)> = Vec::new();
    for mut u in roots {
        if !u.is_finite() { return Err(PointLocationError::NonFiniteIntermediate(face_index)); }
        if u < -parameter_tolerance || u > 1.0 + parameter_tolerance {
            continue;
        }
        let residual_vector = q.sub(b.scale(u));
        let v_axis = c.add(d.scale(u));
        let denominator = v_axis.dot(v_axis);
        let denominator_tolerance = f64::EPSILON * FLOAT_EPSILON_FACTOR * scale * scale;
        if !denominator.is_finite() || denominator <= denominator_tolerance {
            return Err(PointLocationError::SingularBilinearInverse(face_index));
        }
        let mut v = residual_vector.dot(v_axis) / denominator;
        if !v.is_finite() { return Err(PointLocationError::NonFiniteIntermediate(face_index)); }
        if v < -parameter_tolerance || v > 1.0 + parameter_tolerance { continue; }
        u = u.clamp(0.0, 1.0);
        v = v.clamp(0.0, 1.0);
        let reconstructed = a.add(b.scale(u)).add(c.scale(v)).add(d.scale(u*v));
        if !reconstructed.finite() { return Err(PointLocationError::NonFiniteIntermediate(face_index)); }
        if reconstructed.sub(query).length() > spatial { continue; }
        if accepted.iter().all(|(old_u, old_v)| {
            (old_u-u).abs() > parameter_tolerance || (old_v-v).abs() > parameter_tolerance
        }) { accepted.push((u,v)); }
    }

    match accepted.len() {
        0 => Ok(None),
        1 => {
            let (u,v) = accepted[0];
            Ok(Some((FaceCoordinates::QuadBilinear { u, v }, parameter_tolerance)))
        }
        _ => Err(PointLocationError::AmbiguousBilinearInverse(face_index)),
    }
}

fn solve_quadratic(
    a: f64,
    b: f64,
    c: f64,
    coefficient_tolerance: f64,
    face_index: usize,
) -> Result<Vec<f64>, PointLocationError> {
    if a.abs() <= coefficient_tolerance {
        if b.abs() <= coefficient_tolerance {
            return Err(PointLocationError::SingularBilinearInverse(face_index));
        }
        let root = -c / b;
        if !root.is_finite() { return Err(PointLocationError::NonFiniteIntermediate(face_index)); }
        return Ok(vec![root]);
    }
    let mut discriminant = b*b - 4.0*a*c;
    let discriminant_scale = (b*b).abs().max((4.0*a*c).abs()).max(f64::MIN_POSITIVE);
    let discriminant_tolerance = f64::EPSILON * FLOAT_EPSILON_FACTOR * discriminant_scale;
    if !discriminant.is_finite() { return Err(PointLocationError::NonFiniteIntermediate(face_index)); }
    if discriminant < -discriminant_tolerance { return Ok(Vec::new()); }
    if discriminant < 0.0 { discriminant = 0.0; }
    let square_root = discriminant.sqrt();
    if square_root == 0.0 {
        let root = -b / (2.0*a);
        if !root.is_finite() { return Err(PointLocationError::NonFiniteIntermediate(face_index)); }
        return Ok(vec![root]);
    }
    let q = -0.5 * (b + b.signum() * square_root);
    if q == 0.0 || !q.is_finite() { return Err(PointLocationError::SingularBilinearInverse(face_index)); }
    let first = q / a;
    let second = c / q;
    if !first.is_finite() || !second.is_finite() { return Err(PointLocationError::NonFiniteIntermediate(face_index)); }
    if (first-second).abs() <= f64::EPSILON * FLOAT_EPSILON_FACTOR * first.abs().max(second.abs()).max(1.0) {
        Ok(vec![first])
    } else { Ok(vec![first,second]) }
}

fn spatial_tolerance(points: &[Vec2], face_index: usize) -> Result<f64, PointLocationError> {
    let first = *points.first().ok_or(PointLocationError::InvalidFaceGeometry(face_index))?;
    let (mut min_x,mut min_y,mut max_x,mut max_y) = (first.x,first.y,first.x,first.y);
    for point in points.iter().skip(1) {
        min_x=min_x.min(point.x); min_y=min_y.min(point.y);
        max_x=max_x.max(point.x); max_y=max_y.max(point.y);
    }
    let diagonal=(max_x-min_x).hypot(max_y-min_y);
    let tolerance=(diagonal*RELATIVE_SPATIAL_TOLERANCE).max(MIN_SPATIAL_TOLERANCE);
    if !diagonal.is_finite() || diagonal<=0.0 || !tolerance.is_finite() {
        return Err(PointLocationError::InvalidFaceGeometry(face_index));
    }
    Ok(tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::validate_cage_embedding;
    use crate::topology::{
        ControlPointState, ControlStateRevision, DeformationControlState, DeformationTopology,
        DeformationTopologyId, DeformationTopologyRevision, IrregularCageTopology,
        RegularLatticeTopology, TopologyFace, TopologyView,
    };

    fn p(x:f64,y:f64)->ControlPointPosition { ControlPointPosition::new(x,y).unwrap() }
    fn id(value:u64)->ControlPointId { ControlPointId::new(value).unwrap() }
    fn query(x:f64,y:f64)->Point2 { Point2::new(x,y).unwrap() }

    fn certified(topology:DeformationTopology, positions:&[(u64,f64,f64)]) -> ValidatedCageGeometry {
        let state=DeformationControlState {
            topology_id:topology.topology_id(),
            topology_revision:topology.topology_revision(),
            revision:ControlStateRevision::new(1).unwrap(),
            points:positions.iter().map(|(key,x,y)| ControlPointState {
                id:id(*key), position:p(*x,*y), pinned:false,
            }).collect(),
        };
        let certificate=validate_cage_embedding(&topology,&state).unwrap();
        ValidatedCageGeometry::new(&topology,&state,&certificate).unwrap()
    }

    fn two_triangles()->(DeformationTopology,Vec<(u64,f64,f64)>) {
        let topology=DeformationTopology::IrregularCage(IrregularCageTopology {
            id:DeformationTopologyId::new(1).unwrap(),
            revision:DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids:vec![id(1),id(2),id(3),id(4)],
            faces:vec![
                TopologyFace::Triangle([id(1),id(2),id(3)]),
                TopologyFace::Triangle([id(3),id(2),id(4)]),
            ],
        });
        (topology,vec![(1,0.0,0.0),(2,2.0,0.0),(3,0.0,2.0),(4,2.0,2.0)])
    }

    #[test]
    fn locates_triangle_interior_with_barycentric_coordinates() {
        let (topology,positions)=two_triangles();
        let geometry=certified(topology,&positions);
        let result=locate_point(query(0.5,0.5),&geometry).unwrap();
        let PointLocationOutcome::Located(found)=result else { panic!("expected face match"); };
        assert_eq!(found.face_index,0);
        let FaceCoordinates::TriangleBarycentric { weights }=found.coordinates else { panic!("expected triangle coordinates"); };
        assert!((weights[0]-0.5).abs()<1.0e-9);
        assert!((weights[1]-0.25).abs()<1.0e-9);
        assert!((weights[2]-0.25).abs()<1.0e-9);
    }

    #[test]
    fn shared_edge_ownership_uses_lowest_adjacent_face_index() {
        let (topology,positions)=two_triangles();
        let geometry=certified(topology,&positions);
        let result=locate_point(query(1.0,1.0),&geometry).unwrap();
        let PointLocationOutcome::Located(found)=result else { panic!("expected shared-edge match"); };
        assert_eq!(found.face_index,0);
    }

    #[test]
    fn points_outside_all_faces_return_outside_cage() {
        let (topology,positions)=two_triangles();
        let geometry=certified(topology,&positions);
        assert_eq!(locate_point(query(4.0,4.0),&geometry).unwrap(),PointLocationOutcome::OutsideCage);
    }

    #[test]
    fn quad_inverse_bilinear_solves_skewed_convex_quad() {
        let topology=DeformationTopology::IrregularCage(IrregularCageTopology {
            id:DeformationTopologyId::new(2).unwrap(),
            revision:DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids:vec![id(1),id(2),id(3),id(4)],
            faces:vec![TopologyFace::Quad([id(1),id(2),id(3),id(4)])],
        });
        let positions=vec![(1,0.0,0.0),(2,3.0,0.0),(3,2.5,2.0),(4,0.0,2.0)];
        let geometry=certified(topology,&positions);
        let (u,v)=(0.3,0.7);
        let expected=Vec2{x:0.0,y:0.0}.scale((1.0-u)*(1.0-v))
            .add(Vec2{x:3.0,y:0.0}.scale(u*(1.0-v)))
            .add(Vec2{x:2.5,y:2.0}.scale(u*v))
            .add(Vec2{x:0.0,y:2.0}.scale((1.0-u)*v));
        let result=locate_point(query(expected.x,expected.y),&geometry).unwrap();
        let PointLocationOutcome::Located(found)=result else { panic!("expected quad match"); };
        assert_eq!(found.face_index,0);
        let FaceCoordinates::QuadBilinear{u:actual_u,v:actual_v}=found.coordinates else { panic!("expected bilinear coordinates"); };
        assert!((actual_u-u).abs()<1.0e-8);
        assert!((actual_v-v).abs()<1.0e-8);
    }

    #[test]
    fn quad_corner_and_edges_are_inclusive() {
        let topology=DeformationTopology::RegularLattice(RegularLatticeTopology {
            id:DeformationTopologyId::new(3).unwrap(),
            revision:DeformationTopologyRevision::new(1).unwrap(),
            rows:2,columns:2,control_point_ids:vec![id(1),id(2),id(3),id(4)],
        });
        let geometry=certified(topology,&[(1,0.0,0.0),(2,2.0,0.0),(3,0.0,2.0),(4,2.0,2.0)]);
        let result=locate_point(query(0.0,0.0),&geometry).unwrap();
        let PointLocationOutcome::Located(found)=result else { panic!("expected quad corner match"); };
        assert_eq!(found.coordinates,FaceCoordinates::QuadBilinear{u:0.0,v:0.0});
    }

    #[test]
    fn shared_vertex_ownership_uses_lowest_adjacent_face_index() {
        let topology=DeformationTopology::IrregularCage(IrregularCageTopology {
            id:DeformationTopologyId::new(4).unwrap(),
            revision:DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids:vec![id(1),id(2),id(3),id(4),id(5)],
            faces:vec![
                TopologyFace::Triangle([id(1),id(2),id(3)]),
                TopologyFace::Triangle([id(1),id(3),id(4)]),
                TopologyFace::Triangle([id(1),id(4),id(5)]),
            ],
        });
        let geometry=certified(topology,&[
            (1,0.0,0.0),(2,2.0,0.0),(3,1.0,1.0),(4,0.0,2.0),(5,-2.0,0.0),
        ]);
        let result=locate_point(query(0.0,0.0),&geometry).unwrap();
        let PointLocationOutcome::Located(found)=result else { panic!("expected shared vertex match"); };
        assert_eq!(found.face_index,0);
    }

    #[test]
    fn adjacent_quad_shared_edge_uses_stable_face_order() {
        let topology=DeformationTopology::IrregularCage(IrregularCageTopology {
            id:DeformationTopologyId::new(5).unwrap(),
            revision:DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids:vec![id(1),id(2),id(3),id(4),id(5),id(6)],
            faces:vec![
                TopologyFace::Quad([id(1),id(2),id(5),id(4)]),
                TopologyFace::Quad([id(2),id(3),id(6),id(5)]),
            ],
        });
        let geometry=certified(topology,&[
            (1,0.0,0.0),(2,1.0,0.0),(3,2.0,0.0),
            (4,0.0,1.0),(5,1.0,1.0),(6,2.0,1.0),
        ]);
        let result=locate_point(query(1.0,0.5),&geometry).unwrap();
        let PointLocationOutcome::Located(found)=result else { panic!("expected adjacent quad edge match"); };
        assert_eq!(found.face_index,0);
    }

    #[test]
    fn unrelated_candidates_do_not_pass_shared_boundary_check() {
        let a=Candidate {
            located:LocatedFace{face_index:0,subface_index:0,control_point_ids:vec![id(1),id(2),id(3)],
                coordinates:FaceCoordinates::TriangleBarycentric{weights:[0.2,0.3,0.5]}},
            parameter_tolerance:PARAMETER_TOLERANCE,
        };
        let b=Candidate {
            located:LocatedFace{face_index:1,subface_index:0,control_point_ids:vec![id(4),id(5),id(6)],
                coordinates:FaceCoordinates::TriangleBarycentric{weights:[0.2,0.3,0.5]}},
            parameter_tolerance:PARAMETER_TOLERANCE,
        };
        let shared:Vec<_>=a.located.control_point_ids.iter().copied()
            .filter(|id|b.located.control_point_ids.contains(id)).collect();
        assert!(shared.is_empty());
        assert!(!candidate_is_on_shared_boundary(&a,&shared));
    }
}
