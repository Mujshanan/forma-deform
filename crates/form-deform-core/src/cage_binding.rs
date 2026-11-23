//! Generic point-array bindings over the canonical certified cage geometry.
//!
//! This preserves the FDM-02.7 triangle-barycentric point attachment use case
//! without retaining a second indexed cage or global embedding validator.

use std::collections::{BTreeMap, BTreeSet};

use crate::binding::{FaceCoordinates, ValidatedCageGeometry};
use crate::path2d::Point2;
use crate::point_location::{
    locate_point_with_tie_break, PointLocationError, PointLocationOutcome,
    PointLocationTieBreak,
};
use crate::topology::{
    ControlPointId, DeformationTopology, DeformationTopologyId,
    DeformationTopologyRevision, TopologyFace, TopologyView,
};

const BARYCENTRIC_SUM_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BindingPoint {
    pub id: u64,
    pub position: Point2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundPoint {
    pub id: u64,
    /// Stable cage vertex IDs in ascending order.
    pub cage_vertex_ids: [ControlPointId; 3],
    /// Weights corresponding to cage_vertex_ids; sum to one.
    pub barycentric: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct CageBinding {
    topology_id: DeformationTopologyId,
    topology_revision: DeformationTopologyRevision,
    cage_vertex_ids: Vec<ControlPointId>,
    /// Canonical sorted stable vertex IDs for every triangle.
    cage_triangles: Vec<[ControlPointId; 3]>,
    points: Vec<BoundPoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CageBindingError {
    StaleCageTopology,
    UnsupportedFaceKind { face_index: usize },
    DuplicatePointId { point: u64 },
    PointOutsideCage { point: u64 },
    InvalidPointLocation(PointLocationError),
    MissingControlPoint(ControlPointId),
    NonFiniteEvaluation { point: u64 },
}

impl std::fmt::Display for CageBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cage binding failed: {self:?}")
    }
}
impl std::error::Error for CageBindingError {}

impl CageBinding {
    /// Bind arbitrary authored points to a certified triangle cage.
    ///
    /// Points on shared edges/vertices use the lexicographically smallest
    /// sorted stable vertex-ID triple, independent of face-array ordering.
    /// Outside points are rejected; no nearest-edge or extrapolation fallback
    /// is implicit.
    pub fn bind(
        topology: &DeformationTopology,
        rest_cage: &ValidatedCageGeometry,
        points: &[BindingPoint],
    ) -> Result<Self, CageBindingError> {
        validate_topology_view(topology, rest_cage)?;
        let cage_vertex_ids = canonical_vertex_ids(topology);
        let cage_triangles = canonical_triangle_identities(topology)?;

        let mut point_ids = BTreeSet::new();
        for point in points {
            if !point_ids.insert(point.id) {
                return Err(CageBindingError::DuplicatePointId { point: point.id });
            }
        }

        let mut bound = Vec::with_capacity(points.len());
        for point in points {
            let located = match locate_point_with_tie_break(
                point.position,
                rest_cage,
                PointLocationTieBreak::LowestStableVertexIds,
            ).map_err(CageBindingError::InvalidPointLocation)? {
                PointLocationOutcome::Located(located) => located,
                PointLocationOutcome::OutsideCage => {
                    return Err(CageBindingError::PointOutsideCage { point: point.id });
                }
            };
            let FaceCoordinates::TriangleBarycentric { weights } = located.coordinates else {
                return Err(CageBindingError::UnsupportedFaceKind {
                    face_index: located.face_index as usize,
                });
            };
            let ids: [ControlPointId; 3] = located.control_point_ids.try_into()
                .map_err(|_| CageBindingError::UnsupportedFaceKind {
                    face_index: located.face_index as usize,
                })?;
            let mut pairs = [(ids[0], weights[0]), (ids[1], weights[1]), (ids[2], weights[2])];
            pairs.sort_by_key(|pair| pair.0);
            let cage_vertex_ids = [pairs[0].0, pairs[1].0, pairs[2].0];
            let mut barycentric = [pairs[0].1, pairs[1].1, pairs[2].1];
            let sum = barycentric.iter().sum::<f64>();
            if !sum.is_finite() || (sum - 1.0).abs() > BARYCENTRIC_SUM_TOLERANCE {
                return Err(CageBindingError::PointOutsideCage { point: point.id });
            }
            for weight in &mut barycentric { *weight /= sum; }
            bound.push(BoundPoint { id: point.id, cage_vertex_ids, barycentric });
        }

        Ok(Self {
            topology_id: topology.topology_id(),
            topology_revision: topology.topology_revision(),
            cage_vertex_ids,
            cage_triangles,
            points: bound,
        })
    }

    pub fn points(&self) -> &[BoundPoint] { &self.points }

    pub(crate) fn from_persisted_parts(
        topology_id: DeformationTopologyId,
        topology_revision: DeformationTopologyRevision,
        cage_vertex_ids: Vec<ControlPointId>,
        cage_triangles: Vec<[ControlPointId; 3]>,
        points: Vec<BoundPoint>,
    ) -> Self {
        Self { topology_id, topology_revision, cage_vertex_ids, cage_triangles, points }
    }
    pub fn cage_vertex_ids(&self) -> &[ControlPointId] { &self.cage_vertex_ids }
    pub fn cage_triangles(&self) -> &[[ControlPointId; 3]] { &self.cage_triangles }
    pub fn topology_id(&self) -> DeformationTopologyId { self.topology_id }
    pub fn topology_revision(&self) -> DeformationTopologyRevision { self.topology_revision }

    /// Evaluate attachments absolutely against another certified cage pose.
    /// No previous evaluated point positions are read or mutated.
    pub fn evaluate(
        &self,
        topology: &DeformationTopology,
        current_cage: &ValidatedCageGeometry,
    ) -> Result<Vec<BindingPoint>, CageBindingError> {
        validate_topology_view(topology, current_cage)?;
        if topology.topology_id() != self.topology_id
            || topology.topology_revision() != self.topology_revision
            || canonical_vertex_ids(topology) != self.cage_vertex_ids
            || canonical_triangle_identities(topology)? != self.cage_triangles
        {
            return Err(CageBindingError::StaleCageTopology);
        }

        let positions = self.cage_vertex_ids.iter().map(|id| {
            current_cage.position(*id)
                .map(|position| (*id, position))
                .ok_or(CageBindingError::MissingControlPoint(*id))
        }).collect::<Result<BTreeMap<_, _>, _>>()?;

        let mut evaluated = Vec::with_capacity(self.points.len());
        for binding in &self.points {
            let mut x = 0.0;
            let mut y = 0.0;
            for (id, weight) in binding.cage_vertex_ids.iter().zip(binding.barycentric.iter()) {
                let position = positions.get(id).ok_or(CageBindingError::MissingControlPoint(*id))?;
                x += position.x * weight;
                y += position.y * weight;
            }
            let position = Point2::new(x, y)
                .map_err(|_| CageBindingError::NonFiniteEvaluation { point: binding.id })?;
            evaluated.push(BindingPoint { id: binding.id, position });
        }
        Ok(evaluated)
    }
}

pub(crate) fn validate_topology_view(
    topology: &DeformationTopology,
    geometry: &ValidatedCageGeometry,
) -> Result<(), CageBindingError> {
    if geometry.topology_id() != topology.topology_id()
        || geometry.topology_revision() != topology.topology_revision()
        || geometry.face_count() != topology.face_count()
    {
        return Err(CageBindingError::StaleCageTopology);
    }
    for face_index in 0..topology.face_count() {
        if topology.face_at(face_index) != geometry.face(face_index) {
            return Err(CageBindingError::StaleCageTopology);
        }
    }
    if topology.control_point_ids().iter().any(|id| geometry.position(*id).is_none()) {
        return Err(CageBindingError::StaleCageTopology);
    }
    Ok(())
}

pub(crate) fn canonical_vertex_ids(topology: &DeformationTopology) -> Vec<ControlPointId> {
    let mut ids = topology.control_point_ids().to_vec();
    ids.sort_unstable();
    ids
}

pub(crate) fn canonical_triangle_identities(
    topology: &DeformationTopology,
) -> Result<Vec<[ControlPointId; 3]>, CageBindingError> {
    let mut triangles = Vec::with_capacity(topology.face_count());
    for face_index in 0..topology.face_count() {
        let face = topology.face_at(face_index)
            .ok_or(CageBindingError::StaleCageTopology)?;
        let TopologyFace::Triangle(ids) = face else {
            return Err(CageBindingError::UnsupportedFaceKind { face_index });
        };
        let mut ids = ids;
        ids.sort_unstable();
        triangles.push(ids);
    }
    triangles.sort_unstable();
    Ok(triangles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::validate_cage_embedding;
    use crate::topology::{
        ControlPointPosition, ControlPointState, ControlStateRevision,
        DeformationControlState, IrregularCageTopology,
    };

    fn id(value: u64) -> ControlPointId { ControlPointId::new(value).unwrap() }
    fn point(x: f64, y: f64) -> Point2 { Point2::new(x, y).unwrap() }

    fn square(offset_x: f64, offset_y: f64, control_revision: u64)
        -> (DeformationTopology, ValidatedCageGeometry)
    {
        let topology_id = DeformationTopologyId::new(50).unwrap();
        let topology_revision = DeformationTopologyRevision::new(1).unwrap();
        // Face order is intentionally opposite the stable-ID tie-break order.
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: topology_id,
            revision: topology_revision,
            control_point_ids: vec![id(10), id(20), id(30), id(40)],
            faces: vec![
                TopologyFace::Triangle([id(10), id(30), id(40)]),
                TopologyFace::Triangle([id(10), id(20), id(30)]),
            ],
        });
        let coordinates = [
            (offset_x, offset_y),
            (offset_x + 10.0, offset_y),
            (offset_x + 10.0, offset_y + 10.0),
            (offset_x, offset_y + 10.0),
        ];
        let state = DeformationControlState {
            topology_id,
            topology_revision,
            revision: ControlStateRevision::new(control_revision).unwrap(),
            points: coordinates.iter().enumerate().map(|(index, (x, y))| {
                ControlPointState {
                    id: id((index as u64 + 1) * 10),
                    position: ControlPointPosition::new(*x, *y).unwrap(),
                    pinned: false,
                }
            }).collect(),
        };
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology, &state, &certificate).unwrap();
        (topology, geometry)
    }

    #[test]
    fn bind_uses_stable_id_tie_break_on_shared_diagonal() {
        let (topology, geometry) = square(0.0, 0.0, 1);
        let binding = CageBinding::bind(
            &topology, &geometry,
            &[BindingPoint { id: 7, position: point(5.0, 5.0) }],
        ).unwrap();
        assert_eq!(binding.points()[0].cage_vertex_ids, [id(10), id(20), id(30)]);
    }

    #[test]
    fn rest_evaluation_is_identity_and_current_pose_is_absolute() {
        let (topology, rest) = square(0.0, 0.0, 1);
        let binding = CageBinding::bind(
            &topology, &rest,
            &[BindingPoint { id: 7, position: point(2.0, 3.0) }],
        ).unwrap();
        let at_rest = binding.evaluate(&topology, &rest).unwrap();
        assert!((at_rest[0].position.x() - 2.0).abs() <= 1.0e-12);
        assert!((at_rest[0].position.y() - 3.0).abs() <= 1.0e-12);

        let (_, moved) = square(5.0, -2.0, 2);
        let evaluated = binding.evaluate(&topology, &moved).unwrap();
        assert!((evaluated[0].position.x() - 7.0).abs() <= 1.0e-12);
        assert!((evaluated[0].position.y() - 1.0).abs() <= 1.0e-12);
    }

    #[test]
    fn outside_points_and_duplicate_point_ids_are_rejected() {
        let (topology, geometry) = square(0.0, 0.0, 1);
        assert!(matches!(
            CageBinding::bind(&topology, &geometry, &[BindingPoint { id: 1, position: point(20.0, 20.0) }]),
            Err(CageBindingError::PointOutsideCage { point: 1 })
        ));
        assert!(matches!(
            CageBinding::bind(&topology, &geometry, &[
                BindingPoint { id: 1, position: point(2.0, 2.0) },
                BindingPoint { id: 1, position: point(3.0, 3.0) },
            ]),
            Err(CageBindingError::DuplicatePointId { point: 1 })
        ));
    }

    #[test]
    fn quad_cages_are_not_silently_triangulated_for_point_bindings() {
        let topology_id = DeformationTopologyId::new(51).unwrap();
        let topology_revision = DeformationTopologyRevision::new(1).unwrap();
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: topology_id,
            revision: topology_revision,
            control_point_ids: vec![id(1), id(2), id(3), id(4)],
            faces: vec![TopologyFace::Quad([id(1), id(2), id(3), id(4)])],
        });
        let state = DeformationControlState {
            topology_id,
            topology_revision,
            revision: ControlStateRevision::new(1).unwrap(),
            points: vec![
                (1, 0.0, 0.0), (2, 4.0, 0.0), (3, 4.0, 4.0), (4, 0.0, 4.0),
            ].into_iter().map(|(id_value, x, y)| ControlPointState {
                id: id(id_value),
                position: ControlPointPosition::new(x, y).unwrap(),
                pinned: false,
            }).collect(),
        };
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology, &state, &certificate).unwrap();
        assert!(matches!(
            CageBinding::bind(&topology, &geometry, &[BindingPoint { id: 1, position: point(2.0, 2.0) }]),
            Err(CageBindingError::UnsupportedFaceKind { face_index: 0 })
        ));
    }
}
