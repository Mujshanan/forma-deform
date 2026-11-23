//! Stateless evaluation of transient adaptive curve samples against a current cage pose.
//!
//! Authored Path2D remains authoritative. This module evaluates only derived
//! tessellation samples and never rewrites or flattens the source path.

use crate::binding::{BindingError, BindingStamp, FaceCoordinates, ValidatedCageGeometry};
use crate::binding_persistence::ValidatedBindingRecord;
use crate::curve_sampling::{
    map_curve_samples, sample_path_curves, CurveMappingError, CurveSampleMapping,
    CurveSamplingError, MappedCurveSample, MappedCurveSampleSet,
};
use crate::host_contract::{GeometrySnapshot, GeometrySnapshotStamp};
use crate::path2d::Point2;
use crate::topology::{DeformationTopology, TopologyFace, TopologyView};

const WEIGHT_SUM_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvaluatedCurveSample {
    pub command_index: u32,
    pub subpath_index: u32,
    pub sample_index: usize,
    pub t: f64,
    pub point: Point2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluatedCurveSampleSet {
    policy_version: u32,
    policy: crate::curve_sampling::CurveSamplingPolicy,
    source_stamp: GeometrySnapshotStamp,
    binding_stamp: BindingStamp,
    rest_cage_fingerprint: [u8; 32],
    current_cage_fingerprint: [u8; 32],
    samples: Vec<EvaluatedCurveSample>,
    mapped_samples: usize,
    clamped_samples: usize,
    extrapolated_samples: usize,
    unbound_samples: usize,
}

impl EvaluatedCurveSampleSet {
    pub fn policy_version(&self) -> u32 { self.policy_version }
    pub fn policy(&self) -> crate::curve_sampling::CurveSamplingPolicy { self.policy }
    pub fn source_stamp(&self) -> &GeometrySnapshotStamp { &self.source_stamp }
    pub fn binding_stamp(&self) -> &BindingStamp { &self.binding_stamp }
    pub fn rest_cage_fingerprint(&self) -> &[u8; 32] { &self.rest_cage_fingerprint }
    pub fn current_cage_fingerprint(&self) -> &[u8; 32] { &self.current_cage_fingerprint }
    pub fn samples(&self) -> &[EvaluatedCurveSample] { &self.samples }
    pub fn mapped_samples(&self) -> usize { self.mapped_samples }
    pub fn clamped_samples(&self) -> usize { self.clamped_samples }
    pub fn extrapolated_samples(&self) -> usize { self.extrapolated_samples }
    pub fn unbound_samples(&self) -> usize { self.unbound_samples }
}

/// Evaluate mapped adaptive samples from the original source path on a new
/// certified cage pose.
///
/// The supplied mapped set is treated as untrusted derived data: its sample
/// sequence is regenerated from the stamped source and its mappings are
/// recomputed against the supplied certified rest cage. This prevents stale,
/// reordered, forged, or mismatched sample mappings from being evaluated.
/// Both cages must retain the topology identity and revision captured by the
/// validated binding. The current cage may have a newer control-state pose.
pub fn evaluate_curve_samples(
    validated: &ValidatedBindingRecord,
    source: &GeometrySnapshot,
    topology: &DeformationTopology,
    rest_cage: &ValidatedCageGeometry,
    current_cage: &ValidatedCageGeometry,
    mapped_samples: &MappedCurveSampleSet,
) -> Result<EvaluatedCurveSampleSet, CurveSampleEvaluationError> {
    source.validate_envelope()
        .map_err(CurveSampleEvaluationError::InvalidSourceSnapshot)?;

    let binding = validated.binding();
    let stamp = binding.stamp();
    stamp.validate_freshness(source, topology)
        .map_err(CurveSampleEvaluationError::Binding)?;

    validate_cage_topology(rest_cage, topology)?;
    validate_cage_topology(current_cage, topology)?;
    if validated.rest_cage_fingerprint() != rest_cage.geometry_fingerprint() {
        return Err(CurveSampleEvaluationError::StaleRestCageGeometry);
    }

    let regenerated = sample_path_curves(source.payload(), mapped_samples.policy)
        .map_err(CurveSampleEvaluationError::Sampling)?;
    if regenerated.policy_version != mapped_samples.policy_version {
        return Err(CurveSampleEvaluationError::InvalidMappedSampleSet);
    }

    let expected_mapped = map_curve_samples(&regenerated, rest_cage, stamp.outside_policy)
        .map_err(CurveSampleEvaluationError::Mapping)?;
    if expected_mapped != *mapped_samples {
        return Err(CurveSampleEvaluationError::InvalidMappedSampleSet);
    }

    let mut samples = Vec::with_capacity(mapped_samples.samples.len());
    let mut mapped_count = 0usize;
    let mut clamped_count = 0usize;
    let mut extrapolated_count = 0usize;
    let mut unbound_count = 0usize;

    for sample in &mapped_samples.samples {
        let point = match &sample.mapping {
            CurveSampleMapping::Mapped { .. } => {
                mapped_count += 1;
                evaluate_sample_mapping(sample, current_cage)?
            }
            CurveSampleMapping::Clamped { .. } => {
                clamped_count += 1;
                evaluate_sample_mapping(sample, current_cage)?
            }
            CurveSampleMapping::Extrapolated { .. } => {
                extrapolated_count += 1;
                evaluate_sample_mapping(sample, current_cage)?
            }
            CurveSampleMapping::Unbound => {
                unbound_count += 1;
                sample.point
            }
        };
        samples.push(EvaluatedCurveSample {
            command_index: sample.command_index,
            subpath_index: sample.subpath_index,
            sample_index: sample.sample_index,
            t: sample.t,
            point,
        });
    }

    Ok(EvaluatedCurveSampleSet {
        policy_version: mapped_samples.policy_version,
        policy: mapped_samples.policy,
        source_stamp: source.stamp(),
        binding_stamp: stamp.clone(),
        rest_cage_fingerprint: *rest_cage.geometry_fingerprint(),
        current_cage_fingerprint: *current_cage.geometry_fingerprint(),
        samples,
        mapped_samples: mapped_count,
        clamped_samples: clamped_count,
        extrapolated_samples: extrapolated_count,
        unbound_samples: unbound_count,
    })
}


/// Evaluate one original source-space point through the exact same rest-cage
/// point-location and outside-cage semantics used by adaptive curve samples.
pub(crate) fn evaluate_source_point(
    validated: &ValidatedBindingRecord,
    _source: &GeometrySnapshot,
    _topology: &DeformationTopology,
    rest_cage: &ValidatedCageGeometry,
    current_cage: &ValidatedCageGeometry,
    outside_policy: crate::binding::OutsideCagePolicy,
    point: Point2,
) -> Result<Point2, CurveSampleEvaluationError> {
    use crate::outside_policy::{resolve_outside_point, OutsideCageResolution};
    use crate::point_location::{locate_point, PointLocationOutcome};
    use crate::curve_sampling::{CurveSampleMapping, MappedCurveSample};

    if outside_policy != validated.binding().stamp().outside_policy
        || validated.rest_cage_fingerprint() != rest_cage.geometry_fingerprint()
    { return Err(CurveSampleEvaluationError::StaleRestCageGeometry); }
    let mapping = match locate_point(point, rest_cage)
        .map_err(|error| CurveSampleEvaluationError::Mapping(CurveMappingError::PointLocation(error)))? {
        PointLocationOutcome::Located(face) => CurveSampleMapping::Mapped {
            face_index: face.face_index,
            subface_index: face.subface_index,
            control_point_ids: face.control_point_ids,
            coordinates: face.coordinates,
        },
        PointLocationOutcome::OutsideCage => match resolve_outside_point(point, rest_cage, outside_policy)
            .map_err(|error| CurveSampleEvaluationError::Mapping(CurveMappingError::OutsideCage(error)))? {
            OutsideCageResolution::Unbound => CurveSampleMapping::Unbound,
            OutsideCageResolution::Clamped(boundary) => {
                let ids = rest_cage.face(boundary.face_index as usize)
                    .ok_or(CurveSampleEvaluationError::FaceIndexOutOfRange(boundary.face_index as usize))?
                    .control_point_ids().to_vec();
                CurveSampleMapping::Clamped {
                    face_index: boundary.face_index, subface_index: 0,
                    control_point_ids: ids, coordinates: boundary.coordinates,
                }
            }
            OutsideCageResolution::Extrapolated { boundary, coordinates, offset } => {
                let ids = rest_cage.face(boundary.face_index as usize)
                    .ok_or(CurveSampleEvaluationError::FaceIndexOutOfRange(boundary.face_index as usize))?
                    .control_point_ids().to_vec();
                CurveSampleMapping::Extrapolated {
                    face_index: boundary.face_index, subface_index: 0,
                    control_point_ids: ids, coordinates, boundary_offset: offset,
                }
            }
        },
    };
    let sample = MappedCurveSample {
        command_index: 0, subpath_index: 0, sample_index: 0, t: 0.0, point, mapping,
    };
    evaluate_sample_mapping(&sample, current_cage)
}

fn validate_cage_topology(
    cage: &ValidatedCageGeometry,
    topology: &DeformationTopology,
) -> Result<(), CurveSampleEvaluationError> {
    if cage.topology_id() != topology.topology_id()
        || cage.topology_revision() != topology.topology_revision()
    {
        return Err(CurveSampleEvaluationError::StaleCageTopology);
    }
    Ok(())
}

fn evaluate_sample_mapping(
    sample: &crate::curve_sampling::MappedCurveSample,
    cage: &ValidatedCageGeometry,
) -> Result<Point2, CurveSampleEvaluationError> {
    let (face_index, subface_index, control_point_ids, coordinates) =
        match &sample.mapping {
            CurveSampleMapping::Mapped {
                face_index, subface_index, control_point_ids, coordinates,
            }
            | CurveSampleMapping::Clamped {
                face_index, subface_index, control_point_ids, coordinates,
            }
            | CurveSampleMapping::Extrapolated {
                face_index, subface_index, control_point_ids, coordinates, ..
            } => (*face_index, *subface_index, control_point_ids, *coordinates),
            CurveSampleMapping::Unbound => return Ok(sample.point),
        };

    if subface_index != 0 {
        return Err(CurveSampleEvaluationError::UnsupportedSubface(subface_index));
    }
    let index = face_index as usize;
    let face = cage.face(index)
        .ok_or(CurveSampleEvaluationError::FaceIndexOutOfRange(index))?;
    if face.control_point_ids() != control_point_ids.as_slice() {
        return Err(CurveSampleEvaluationError::ControlPointIdOrderMismatch {
            face_index: index,
        });
    }

    let positions = control_point_ids.iter().map(|id| {
        cage.position(*id).ok_or(CurveSampleEvaluationError::MissingControlPoint(*id))
    }).collect::<Result<Vec<_>, _>>()?;

    let (x, y) = match (face, coordinates) {
        (TopologyFace::Triangle(_), FaceCoordinates::TriangleBarycentric { weights }) => {
            if weights.iter().any(|weight| !weight.is_finite()) {
                return Err(CurveSampleEvaluationError::NonFiniteMapping);
            }
            let sum = weights.iter().sum::<f64>();
            if !sum.is_finite() || (sum - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
                return Err(CurveSampleEvaluationError::InvalidBarycentricWeights);
            }
            (
                positions[0].x * weights[0]
                    + positions[1].x * weights[1]
                    + positions[2].x * weights[2],
                positions[0].y * weights[0]
                    + positions[1].y * weights[1]
                    + positions[2].y * weights[2],
            )
        }
        (TopologyFace::Quad(_), FaceCoordinates::QuadBilinear { u, v }) => {
            if !u.is_finite() || !v.is_finite() {
                return Err(CurveSampleEvaluationError::NonFiniteMapping);
            }
            let weights = [
                (1.0 - u) * (1.0 - v),
                u * (1.0 - v),
                u * v,
                (1.0 - u) * v,
            ];
            (
                positions.iter().zip(weights).map(|(point, weight)| point.x * weight).sum(),
                positions.iter().zip(weights).map(|(point, weight)| point.y * weight).sum(),
            )
        }
        _ => return Err(CurveSampleEvaluationError::FaceMappingKindMismatch {
            face_index: index,
        }),
    };

    Point2::new(x, y).map_err(|_| CurveSampleEvaluationError::NonFiniteResult)
}

#[derive(Clone, Debug, PartialEq)]
pub enum CurveSampleEvaluationError {
    InvalidSourceSnapshot(crate::host_contract::ContractError),
    Binding(BindingError),
    Sampling(CurveSamplingError),
    Mapping(CurveMappingError),
    InvalidMappedSampleSet,
    StaleCageTopology,
    StaleRestCageGeometry,
    UnsupportedSubface(u8),
    FaceIndexOutOfRange(usize),
    ControlPointIdOrderMismatch { face_index: usize },
    MissingControlPoint(crate::topology::ControlPointId),
    FaceMappingKindMismatch { face_index: usize },
    NonFiniteMapping,
    InvalidBarycentricWeights,
    NonFiniteResult,
}

impl std::fmt::Display for CurveSampleEvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSourceSnapshot(error) => write!(f, "source snapshot is invalid: {error}"),
            Self::Binding(error) => write!(f, "binding is stale or invalid: {error}"),
            Self::Sampling(error) => write!(f, "source curve sampling failed: {error}"),
            Self::Mapping(error) => write!(f, "rest-pose curve mapping failed: {error:?}"),
            Self::InvalidMappedSampleSet => write!(f, "mapped curve samples do not match the stamped source and rest cage"),
            Self::StaleCageTopology => write!(f, "rest or current cage topology identity/revision is stale"),
            Self::StaleRestCageGeometry => write!(f, "rest cage geometry differs from the geometry used to validate the binding"),
            Self::UnsupportedSubface(index) => write!(f, "unsupported curve-sample subface index {index}"),
            Self::FaceIndexOutOfRange(index) => write!(f, "curve sample face index {index} is out of range"),
            Self::ControlPointIdOrderMismatch { face_index } => write!(f, "curve sample control-point order does not match face {face_index}"),
            Self::MissingControlPoint(id) => write!(f, "curve sample references missing control point {}", id.get()),
            Self::FaceMappingKindMismatch { face_index } => write!(f, "curve sample coordinate kind does not match face {face_index}"),
            Self::NonFiniteMapping => write!(f, "curve sample mapping contains non-finite coordinates"),
            Self::InvalidBarycentricWeights => write!(f, "curve sample barycentric weights are invalid"),
            Self::NonFiniteResult => write!(f, "evaluated curve sample is non-finite"),
        }
    }
}
impl std::error::Error for CurveSampleEvaluationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::{
        enumerate_source_features, BindingRecord, BindingStamp, FeatureBindingEntry,
        MappingPolicy, OutsideCagePolicy, SourceFeatureBinding, ValidatedCageGeometry,
    };
    use crate::binding_persistence::PersistedBinding;
    use crate::embedding::validate_cage_embedding;
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceDescriptor, CoordinateSpaceId,
        CoordinateSpaceKind, GeometryRevision, HostDocumentId, HostNamespace, HostTargetId,
        HostTargetKind, HostTargetRef, TopologyRevision,
    };
    use crate::path2d::{Path2D, PathCommand, PATH2D_PAYLOAD_KIND};
    use crate::point_location::{locate_point, PointLocationOutcome};
    use crate::topology::{
        ControlPointId, ControlPointPosition, ControlPointState, ControlStateRevision,
        DeformationControlState, DeformationTopologyId, DeformationTopologyRevision,
        IrregularCageTopology, TopologyFace, TopologyView,
    };

    fn point(x: f64, y: f64) -> Point2 { Point2::new(x, y).unwrap() }

    fn source_path() -> Path2D {
        Path2D::new(vec![
            PathCommand::MoveTo(point(0.2, 0.2)),
            PathCommand::CubicTo {
                control1: point(0.4, 0.3),
                control2: point(0.8, 0.9),
                to: point(1.2, 1.0),
            },
        ]).unwrap()
    }

    fn space() -> CoordinateSpaceDescriptor {
        CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("target-local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        }
    }

    fn target() -> HostTargetRef {
        HostTargetRef {
            namespace: HostNamespace::new("anicraft").unwrap(),
            document_id: HostDocumentId::new("curve-evaluation-doc").unwrap(),
            target_kind: HostTargetKind::new("shape").unwrap(),
            target_id: HostTargetId::new("curve-shape").unwrap(),
            generation: 1,
        }
    }

    fn snapshot(path: Path2D, revision: u64) -> GeometrySnapshot {
        let coordinate_space = space();
        let fingerprint = path.fingerprint(&coordinate_space).unwrap();
        GeometrySnapshot::new(
            target(), GeometryRevision(revision), TopologyRevision(1),
            fingerprint, coordinate_space, PATH2D_PAYLOAD_KIND, 1, path,
        ).unwrap()
    }

    fn cage() -> (DeformationTopology, DeformationControlState) {
        let ids = (1..=4).map(|value| ControlPointId::new(value).unwrap()).collect::<Vec<_>>();
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: DeformationTopologyId::new(51).unwrap(),
            revision: DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids: ids.clone(),
            faces: vec![
                TopologyFace::Triangle([ids[0], ids[1], ids[2]]),
                TopologyFace::Triangle([ids[2], ids[1], ids[3]]),
            ],
        });
        let positions = [(0.0, 0.0), (2.0, 0.0), (0.0, 2.0), (2.0, 2.0)];
        let state = DeformationControlState {
            topology_id: topology.topology_id(),
            topology_revision: topology.topology_revision(),
            revision: ControlStateRevision::new(1).unwrap(),
            points: ids.into_iter().zip(positions).map(|(id, (x, y))| ControlPointState {
                id, position: ControlPointPosition::new(x, y).unwrap(), pinned: false,
            }).collect(),
        };
        (topology, state)
    }

    fn validated_fixture() -> (
        GeometrySnapshot, DeformationTopology, DeformationControlState,
        ValidatedCageGeometry, ValidatedBindingRecord,
    ) {
        validated_fixture_with_policy(OutsideCagePolicy::UnboundZeroInfluence)
    }

    fn validated_fixture_with_policy(
        outside_policy: OutsideCagePolicy,
    ) -> (
        GeometrySnapshot, DeformationTopology, DeformationControlState,
        ValidatedCageGeometry, ValidatedBindingRecord,
    ) {
        let source = snapshot(source_path(), 1);
        let (topology, rest_state) = cage();
        let certificate = validate_cage_embedding(&topology, &rest_state).unwrap();
        let rest_geometry = ValidatedCageGeometry::new(&topology, &rest_state, &certificate).unwrap();
        let stamp = BindingStamp::from_snapshot(
            &source, &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            outside_policy,
        ).unwrap();
        let entries = enumerate_source_features(source.payload()).unwrap().into_iter().map(|feature| {
            let status = match locate_point(feature.point, &rest_geometry).unwrap() {
                PointLocationOutcome::Located(location) => {
                    let face = topology.face_at(location.face_index as usize).unwrap();
                    SourceFeatureBinding::Mapped(crate::binding::MappingRecord {
                        locator: feature.locator,
                        face_index: location.face_index,
                        subface_index: 0,
                        control_point_ids: face.control_point_ids().to_vec(),
                        coordinates: location.coordinates,
                    })
                }
                PointLocationOutcome::OutsideCage => SourceFeatureBinding::Unbound,
            };
            FeatureBindingEntry { locator: feature.locator, status }
        }).collect();
        let persisted = PersistedBinding::new(BindingRecord::new(stamp, entries)).unwrap();
        let validated = persisted.validate_against(&source, &topology, &rest_state).unwrap();
        (source, topology, rest_state, rest_geometry, validated)
    }

    fn translated_state(
        topology: &DeformationTopology,
        state: &DeformationControlState,
        dx: f64,
        dy: f64,
    ) -> DeformationControlState {
        let mut moved = state.clone();
        moved.revision = ControlStateRevision::new(state.revision.get() + 1).unwrap();
        for point in &mut moved.points {
            point.position = ControlPointPosition::new(point.position.x + dx, point.position.y + dy).unwrap();
        }
        moved.validate_against(topology).unwrap();
        moved
    }

    #[test]
    fn cubic_samples_follow_current_pose_without_mutating_or_flattening_source() {
        let (source, topology, rest_state, rest_geometry, validated) = validated_fixture();
        let sample_set = sample_path_curves(source.payload(), crate::curve_sampling::CurveSamplingPolicy {
            max_deviation: 0.02,
            max_subdivision_depth: 16,
            max_samples_per_curve: 1024,
            max_total_samples: 2048,
        }).unwrap();
        let mapped = map_curve_samples(
            &sample_set, &rest_geometry, validated.binding().stamp().outside_policy,
        ).unwrap();

        let moved = translated_state(&topology, &rest_state, 5.0, -2.0);
        let certificate = validate_cage_embedding(&topology, &moved).unwrap();
        let current_geometry = ValidatedCageGeometry::new(&topology, &moved, &certificate).unwrap();

        let first = evaluate_curve_samples(
            &validated, &source, &topology, &rest_geometry, &current_geometry, &mapped,
        ).unwrap();
        let second = evaluate_curve_samples(
            &validated, &source, &topology, &rest_geometry, &current_geometry, &mapped,
        ).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.samples().len(), mapped.samples.len());
        assert!(first.mapped_samples() > 0);
        assert_eq!(first.clamped_samples(), 0);
        assert_eq!(first.extrapolated_samples(), 0);
        assert_eq!(first.unbound_samples(), 0);
        for (original, evaluated) in mapped.samples.iter().zip(first.samples()) {
            assert_eq!(original.command_index, evaluated.command_index);
            assert_eq!(original.subpath_index, evaluated.subpath_index);
            assert_eq!(original.sample_index, evaluated.sample_index);
            assert_eq!(original.t, evaluated.t);
            assert!((evaluated.point.x() - (original.point.x() + 5.0)).abs() < 1.0e-8);
            assert!((evaluated.point.y() - (original.point.y() - 2.0)).abs() < 1.0e-8);
        }
        assert_eq!(source.payload(), &source_path());
        assert_eq!(source.payload().commands().len(), 2);
    }

    #[test]
    fn stale_source_and_tampered_sample_mappings_are_rejected() {
        let (source, topology, _rest_state, rest_geometry, validated) = validated_fixture();
        let sample_set = sample_path_curves(source.payload(), crate::curve_sampling::CurveSamplingPolicy::default()).unwrap();
        let mapped = map_curve_samples(
            &sample_set, &rest_geometry, validated.binding().stamp().outside_policy,
        ).unwrap();

        let stale_source = snapshot(source_path(), 2);
        assert!(matches!(
            evaluate_curve_samples(&validated, &stale_source, &topology, &rest_geometry, &rest_geometry, &mapped),
            Err(CurveSampleEvaluationError::Binding(_))
        ));

        let mut tampered = mapped.clone();
        tampered.samples[0].point = point(0.7, 0.7);
        assert_eq!(
            evaluate_curve_samples(&validated, &source, &topology, &rest_geometry, &rest_geometry, &tampered),
            Err(CurveSampleEvaluationError::InvalidMappedSampleSet)
        );
    }


    #[test]
    fn same_topology_with_different_rest_geometry_is_rejected() {
        let (source, topology, rest_state, rest_geometry, validated) = validated_fixture();
        let sample_set = sample_path_curves(
            source.payload(),
            crate::curve_sampling::CurveSamplingPolicy::default(),
        ).unwrap();
        let mapped = map_curve_samples(
            &sample_set, &rest_geometry, validated.binding().stamp().outside_policy,
        ).unwrap();

        let different_rest_state = translated_state(&topology, &rest_state, 0.5, 0.0);
        let certificate = validate_cage_embedding(&topology, &different_rest_state).unwrap();
        let different_rest_geometry =
            ValidatedCageGeometry::new(&topology, &different_rest_state, &certificate).unwrap();

        assert_eq!(different_rest_geometry.topology_id(), rest_geometry.topology_id());
        assert_eq!(different_rest_geometry.topology_revision(), rest_geometry.topology_revision());
        assert_ne!(different_rest_geometry.geometry_fingerprint(), rest_geometry.geometry_fingerprint());
        assert_eq!(
            evaluate_curve_samples(
                &validated, &source, &topology, &different_rest_geometry, &rest_geometry, &mapped,
            ),
            Err(CurveSampleEvaluationError::StaleRestCageGeometry)
        );
    }

    #[test]
    fn mismatched_cage_topology_is_rejected() {
        let (source, topology, _rest_state, rest_geometry, validated) = validated_fixture();
        let sample_set = sample_path_curves(source.payload(), crate::curve_sampling::CurveSamplingPolicy::default()).unwrap();
        let mapped = map_curve_samples(
            &sample_set, &rest_geometry, validated.binding().stamp().outside_policy,
        ).unwrap();
        let other_topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: DeformationTopologyId::new(52).unwrap(),
            revision: DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids: topology.control_point_ids().to_vec(),
            faces: (0..topology.face_count()).map(|index| topology.face_at(index).unwrap()).collect(),
        });
        assert!(matches!(
            evaluate_curve_samples(&validated, &source, &other_topology, &rest_geometry, &rest_geometry, &mapped),
            Err(CurveSampleEvaluationError::Binding(_))
        ));
    }

    #[test]

    #[test]
    fn output_refinement_detects_a_curve_crossing_the_shared_triangle_edge() {
        use crate::output_space_refinement::{refine_path_curves, OutputRefinementPolicy};
        let (source, topology, rest_state, rest_geometry, validated) = validated_fixture();
        let mut warped = rest_state.clone();
        warped.revision = ControlStateRevision::new(rest_state.revision.get() + 1).unwrap();
        let corner = ControlPointId::new(4).unwrap();
        let point_state = warped.points.iter_mut().find(|point_state| point_state.id == corner).unwrap();
        point_state.position = ControlPointPosition::new(4.0, 3.0).unwrap();
        warped.validate_against(&topology).unwrap();
        let certificate = validate_cage_embedding(&topology, &warped).unwrap();
        let current = ValidatedCageGeometry::new(&topology, &warped, &certificate).unwrap();
        let policy = OutputRefinementPolicy {
            max_deviation: 0.001, max_subdivision_depth: 20,
            max_samples_per_curve: 8192, max_total_samples: 16_384,
            max_total_evaluations: 100_000,
        };
        let refined = refine_path_curves(&validated, &source, &topology, &rest_geometry, &current,
            OutsideCagePolicy::UnboundZeroInfluence, policy).unwrap();
        let samples = &refined.segments[0].samples;
        let side = |sample: &crate::output_space_refinement::RefinedCurveSample| {
            sample.source_point.x() + sample.source_point.y() - 2.0
        };
        assert!(samples.iter().any(|sample| side(sample) < -1.0e-6));
        assert!(samples.iter().any(|sample| side(sample) > 1.0e-6));
        assert!(samples.windows(2).any(|pair| side(&pair[0]) * side(&pair[1]) <= 0.0));
        assert!(samples.iter().any(|sample| side(sample) > 1.0e-6
            && (sample.output_point.x() - sample.source_point.x()).abs() > 0.1));
        assert!(refined.total_evaluations > samples.len());
    }

    #[test]
    fn arbitrary_source_point_obeys_each_outside_cage_policy() {
        let query = point(2.5, 1.0);
        let (source, topology, _rest, rest_geometry, unbound) =
            validated_fixture_with_policy(OutsideCagePolicy::UnboundZeroInfluence);
        assert_eq!(evaluate_source_point(&unbound, &source, &topology, &rest_geometry, &rest_geometry,
            OutsideCagePolicy::UnboundZeroInfluence, query).unwrap(), query);

        let (source, topology, rest, rest_geometry, clamped) =
            validated_fixture_with_policy(OutsideCagePolicy::ClampToBoundary);
        let moved = translated_state(&topology, &rest, 5.0, -2.0);
        let certificate = validate_cage_embedding(&topology, &moved).unwrap();
        let current = ValidatedCageGeometry::new(&topology, &moved, &certificate).unwrap();
        let clamped_point = evaluate_source_point(&clamped, &source, &topology, &rest_geometry, &current,
            OutsideCagePolicy::ClampToBoundary, query).unwrap();
        assert!((clamped_point.x() - 7.0).abs() < 1.0e-8);
        assert!((clamped_point.y() + 1.0).abs() < 1.0e-8);

        let extrapolation = OutsideCagePolicy::BoundedExtrapolation { max_distance: 1.0 };
        let (source, topology, rest, rest_geometry, extrapolated) =
            validated_fixture_with_policy(extrapolation);
        let moved = translated_state(&topology, &rest, 5.0, -2.0);
        let certificate = validate_cage_embedding(&topology, &moved).unwrap();
        let current = ValidatedCageGeometry::new(&topology, &moved, &certificate).unwrap();
        let extrapolated_point = evaluate_source_point(&extrapolated, &source, &topology, &rest_geometry,
            &current, extrapolation, query).unwrap();
        assert!((extrapolated_point.x() - 7.5).abs() < 1.0e-8);
        assert!((extrapolated_point.y() + 1.0).abs() < 1.0e-8);

        let limited = OutsideCagePolicy::BoundedExtrapolation { max_distance: 0.1 };
        let (source, topology, _rest, rest_geometry, too_limited) =
            validated_fixture_with_policy(limited);
        assert!(matches!(evaluate_source_point(&too_limited, &source, &topology, &rest_geometry,
            &rest_geometry, limited, query),
            Err(CurveSampleEvaluationError::Mapping(CurveMappingError::OutsideCage(
                crate::outside_policy::OutsideCageError::DistanceLimitExceeded { .. }
            )))
        ));
    }

    fn output_space_refinement_is_deterministic_and_uses_current_pose() {
        use crate::output_space_refinement::{refine_path_curves, OutputRefinementPolicy};
        let (source, topology, rest_state, rest_geometry, validated) = validated_fixture();
        let moved = translated_state(&topology, &rest_state, 5.0, -2.0);
        let certificate = validate_cage_embedding(&topology, &moved).unwrap();
        let current = ValidatedCageGeometry::new(&topology, &moved, &certificate).unwrap();
        let policy = OutputRefinementPolicy {
            max_deviation: 0.01,
            max_subdivision_depth: 16,
            max_samples_per_curve: 4096,
            max_total_samples: 8192,
            max_total_evaluations: 100_000,
        };
        let first = refine_path_curves(
            &validated, &source, &topology, &rest_geometry, &current,
            validated.binding().stamp().outside_policy, policy,
        ).unwrap();
        let second = refine_path_curves(
            &validated, &source, &topology, &rest_geometry, &current,
            validated.binding().stamp().outside_policy, policy,
        ).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.segments.len(), 1);
        let segment = &first.segments[0];
        assert_eq!(segment.samples.first().unwrap().t, 0.0);
        assert_eq!(segment.samples.last().unwrap().t, 1.0);
        assert!(segment.samples.len() > 2);
        assert!(segment.samples.windows(2).all(|pair| pair[0].t < pair[1].t));
        for sample in &segment.samples {
            assert!((sample.output_point.x() - (sample.source_point.x() + 5.0)).abs() < 1.0e-8);
            assert!((sample.output_point.y() - (sample.source_point.y() - 2.0)).abs() < 1.0e-8);
        }
        assert!(first.max_observed_deviation <= policy.max_deviation);
        assert!(first.total_evaluations <= policy.max_total_evaluations);
    }

    #[test]
    fn output_space_refinement_fails_closed_when_depth_budget_is_exhausted() {
        use crate::output_space_refinement::{refine_path_curves, OutputRefinementError, OutputRefinementPolicy};
        let (source, topology, rest_state, rest_geometry, validated) = validated_fixture();
        let policy = OutputRefinementPolicy {
            max_deviation: 1.0e-12,
            max_subdivision_depth: 0,
            max_samples_per_curve: 64,
            max_total_samples: 64,
            max_total_evaluations: 64,
        };
        let result = refine_path_curves(
            &validated, &source, &topology, &rest_geometry, &rest_geometry,
            validated.binding().stamp().outside_policy, policy,
        );
        assert!(matches!(result, Err(OutputRefinementError::SubdivisionDepthExceeded { .. })));
        let _ = rest_state;
    }
}
