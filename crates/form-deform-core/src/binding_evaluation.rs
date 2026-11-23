//! Stateless evaluation of validated source-to-cage bindings.
//!
//! A binding is authored in rest space and remains tied to that immutable
//! source/topology pair. Evaluation applies its stored face coordinates to a
//! separately certified current cage pose. No host state or mutable geometry
//! is accessed, and every call starts from the original Path2D snapshot.

use std::collections::BTreeMap;

use crate::binding::{
    BindingError, FaceCoordinates, SourceFeatureBinding, SourceFeatureError,
    SourceFeatureLocator, ValidatedBindingRecord, ValidatedCageGeometry,
    enumerate_source_features,
};
use crate::host_contract::{
    ContractError, GeometryFingerprint, GeometrySnapshot, GeometrySnapshotStamp,
};
use crate::path2d::{Path2D, Path2DError, PathCommand, Point2};
use crate::topology::{DeformationTopology, TopologyFace, TopologyView};

const WEIGHT_SUM_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluatedBindingGeometry {
    geometry: Path2D,
    source_stamp: GeometrySnapshotStamp,
    cage_geometry_fingerprint: [u8; 32],
    fingerprint: GeometryFingerprint,
    mapped_features: usize,
    clamped_features: usize,
    extrapolated_features: usize,
    unbound_features: usize,
}

impl EvaluatedBindingGeometry {
    pub fn geometry(&self) -> &Path2D { &self.geometry }
    pub fn into_geometry(self) -> Path2D { self.geometry }
    pub fn source_stamp(&self) -> &GeometrySnapshotStamp { &self.source_stamp }
    pub fn cage_geometry_fingerprint(&self) -> &[u8; 32] { &self.cage_geometry_fingerprint }
    pub fn fingerprint(&self) -> &GeometryFingerprint { &self.fingerprint }
    pub fn mapped_features(&self) -> usize { self.mapped_features }
    pub fn clamped_features(&self) -> usize { self.clamped_features }
    pub fn extrapolated_features(&self) -> usize { self.extrapolated_features }
    pub fn unbound_features(&self) -> usize { self.unbound_features }
}

/// Evaluate one validated binding against one certified current cage pose.
///
/// The binding must have been validated against the rest-space source and
/// cage. The current cage may have a newer control-state revision, but must
/// retain the exact topology identity and revision captured by the binding.
/// The caller supplies a cage created from a current embedding certificate.
pub fn evaluate_binding(
    validated: &ValidatedBindingRecord,
    source: &GeometrySnapshot,
    topology: &DeformationTopology,
    current_cage: &ValidatedCageGeometry,
) -> Result<EvaluatedBindingGeometry, BindingEvaluationError> {
    source.validate_envelope()
        .map_err(BindingEvaluationError::InvalidCurrentSnapshot)?;

    let binding = validated.binding();
    binding.stamp().validate_freshness(source, topology)
        .map_err(BindingEvaluationError::Binding)?;

    if current_cage.topology_id() != topology.topology_id()
        || current_cage.topology_revision() != topology.topology_revision()
    {
        return Err(BindingEvaluationError::StaleCageTopology);
    }

    let features = enumerate_source_features(source.payload())
        .map_err(BindingEvaluationError::SourceFeatures)?;
    if features.len() != binding.entries().len() {
        return Err(BindingEvaluationError::IncompleteFeatureCoverage);
    }

    let mut evaluated_points = BTreeMap::new();
    let mut mapped_features = 0usize;
    let mut clamped_features = 0usize;
    let mut extrapolated_features = 0usize;
    let mut unbound_features = 0usize;

    for (feature, entry) in features.iter().zip(binding.entries()) {
        if feature.locator != entry.locator || evaluated_points.contains_key(&entry.locator) {
            return Err(BindingEvaluationError::InvalidFeatureLocator(entry.locator));
        }

        let point = match &entry.status {
            SourceFeatureBinding::Mapped(mapping) => {
                mapped_features += 1;
                evaluate_mapping(mapping.locator, mapping.face_index, mapping.subface_index,
                    &mapping.control_point_ids, mapping.coordinates, current_cage)?
            }
            SourceFeatureBinding::Clamped(mapping) => {
                clamped_features += 1;
                evaluate_mapping(mapping.locator, mapping.face_index, mapping.subface_index,
                    &mapping.control_point_ids, mapping.coordinates, current_cage)?
            }
            SourceFeatureBinding::Extrapolated(mapping) => {
                extrapolated_features += 1;
                evaluate_mapping(mapping.locator, mapping.face_index, mapping.subface_index,
                    &mapping.control_point_ids, mapping.coordinates, current_cage)?
            }
            SourceFeatureBinding::Unbound => {
                unbound_features += 1;
                feature.point
            }
        };

        evaluated_points.insert(entry.locator, point);
    }

    let geometry = rebuild_path(source.payload(), &evaluated_points)?;
    let fingerprint = geometry.fingerprint(source.coordinate_space())
        .map_err(BindingEvaluationError::Fingerprint)?;

    Ok(EvaluatedBindingGeometry {
        geometry,
        source_stamp: source.stamp(),
        cage_geometry_fingerprint: *current_cage.geometry_fingerprint(),
        fingerprint,
        mapped_features,
        clamped_features,
        extrapolated_features,
        unbound_features,
    })
}

fn evaluate_mapping(
    locator: SourceFeatureLocator,
    face_index: u32,
    subface_index: u8,
    control_point_ids: &[crate::topology::ControlPointId],
    coordinates: FaceCoordinates,
    cage: &ValidatedCageGeometry,
) -> Result<Point2, BindingEvaluationError> {
    if subface_index != 0 {
        return Err(BindingEvaluationError::UnsupportedSubface(subface_index));
    }

    let index = face_index as usize;
    let face = cage.face(index)
        .ok_or(BindingEvaluationError::FaceIndexOutOfRange(index))?;
    let face_ids = face.control_point_ids();
    if face_ids != control_point_ids {
        return Err(BindingEvaluationError::ControlPointIdOrderMismatch { face_index: index });
    }

    let mut positions = Vec::with_capacity(face_ids.len());
    for id in face_ids {
        let position = cage.position(*id)
            .ok_or(BindingEvaluationError::MissingControlPoint(*id))?;
        positions.push(position);
    }

    let (x, y) = match (face, coordinates) {
        (TopologyFace::Triangle(_), FaceCoordinates::TriangleBarycentric { weights }) => {
            if weights.iter().any(|weight| !weight.is_finite()) {
                return Err(BindingEvaluationError::NonFiniteMapping(locator));
            }
            let sum = weights.iter().sum::<f64>();
            if !sum.is_finite() || (sum - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
                return Err(BindingEvaluationError::InvalidBarycentricWeights(locator));
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
                return Err(BindingEvaluationError::NonFiniteMapping(locator));
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
        _ => return Err(BindingEvaluationError::FaceMappingKindMismatch { face_index: index }),
    };

    Point2::new(x, y).map_err(|_| BindingEvaluationError::NonFiniteResult(locator))
}

fn rebuild_path(
    source: &Path2D,
    points: &BTreeMap<SourceFeatureLocator, Point2>,
) -> Result<Path2D, BindingEvaluationError> {
    let mut commands = Vec::with_capacity(source.commands().len());

    for (index, command) in source.commands().iter().enumerate() {
        let command_index = u32::try_from(index)
            .map_err(|_| BindingEvaluationError::TooManyCommands)?;
        let point = |role| {
            let locator = SourceFeatureLocator { command_index, role };
            points.get(&locator).copied()
                .ok_or(BindingEvaluationError::MissingEvaluatedFeature(locator))
        };

        commands.push(match command {
            PathCommand::MoveTo(_) => PathCommand::MoveTo(point(crate::binding::PathFeatureRole::MoveToEndpoint)?),
            PathCommand::LineTo(_) => PathCommand::LineTo(point(crate::binding::PathFeatureRole::LineToEndpoint)?),
            PathCommand::QuadraticTo { .. } => PathCommand::QuadraticTo {
                control: point(crate::binding::PathFeatureRole::QuadraticControl)?,
                to: point(crate::binding::PathFeatureRole::QuadraticEndpoint)?,
            },
            PathCommand::CubicTo { .. } => PathCommand::CubicTo {
                control1: point(crate::binding::PathFeatureRole::CubicControl1)?,
                control2: point(crate::binding::PathFeatureRole::CubicControl2)?,
                to: point(crate::binding::PathFeatureRole::CubicEndpoint)?,
            },
            PathCommand::Close => PathCommand::Close,
        });
    }

    Path2D::new(commands).map_err(BindingEvaluationError::InvalidOutputPath)
}

#[derive(Clone, Debug, PartialEq)]
pub enum BindingEvaluationError {
    InvalidCurrentSnapshot(ContractError),
    Binding(BindingError),
    SourceFeatures(SourceFeatureError),
    StaleCageTopology,
    IncompleteFeatureCoverage,
    InvalidFeatureLocator(SourceFeatureLocator),
    UnsupportedSubface(u8),
    FaceIndexOutOfRange(usize),
    ControlPointIdOrderMismatch { face_index: usize },
    MissingControlPoint(crate::topology::ControlPointId),
    FaceMappingKindMismatch { face_index: usize },
    NonFiniteMapping(SourceFeatureLocator),
    InvalidBarycentricWeights(SourceFeatureLocator),
    NonFiniteResult(SourceFeatureLocator),
    TooManyCommands,
    MissingEvaluatedFeature(SourceFeatureLocator),
    InvalidOutputPath(Path2DError),
    Fingerprint(ContractError),
}

impl std::fmt::Display for BindingEvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCurrentSnapshot(error) => write!(f, "current source snapshot is invalid: {error}"),
            Self::Binding(error) => write!(f, "binding is stale or invalid: {error}"),
            Self::SourceFeatures(error) => write!(f, "source feature enumeration failed: {error}"),
            Self::StaleCageTopology => write!(f, "current cage topology identity or revision is stale"),
            Self::IncompleteFeatureCoverage => write!(f, "validated binding does not cover the current source features"),
            Self::InvalidFeatureLocator(locator) => write!(f, "invalid or duplicate feature locator ({}, {:?})", locator.command_index, locator.role),
            Self::UnsupportedSubface(index) => write!(f, "unsupported cage subface index {index}"),
            Self::FaceIndexOutOfRange(index) => write!(f, "binding face index {index} is out of range"),
            Self::ControlPointIdOrderMismatch { face_index } => write!(f, "binding control-point order does not match face {face_index}"),
            Self::MissingControlPoint(id) => write!(f, "binding references missing control point {}", id.get()),
            Self::FaceMappingKindMismatch { face_index } => write!(f, "binding coordinate kind does not match face {face_index}"),
            Self::NonFiniteMapping(locator) => write!(f, "binding coordinates are non-finite for feature ({}, {:?})", locator.command_index, locator.role),
            Self::InvalidBarycentricWeights(locator) => write!(f, "barycentric weights are invalid for feature ({}, {:?})", locator.command_index, locator.role),
            Self::NonFiniteResult(locator) => write!(f, "evaluated position is non-finite for feature ({}, {:?})", locator.command_index, locator.role),
            Self::TooManyCommands => write!(f, "Path2D command count exceeds u32 locator capacity"),
            Self::MissingEvaluatedFeature(locator) => write!(f, "evaluated feature ({}, {:?}) is missing", locator.command_index, locator.role),
            Self::InvalidOutputPath(error) => write!(f, "evaluated Path2D is invalid: {error}"),
            Self::Fingerprint(error) => write!(f, "evaluated geometry fingerprint failed: {error}"),
        }
    }
}
impl std::error::Error for BindingEvaluationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::{
        BindingRecord, BindingStamp, FeatureBindingEntry, MappingPolicy, OutsideCagePolicy,
        SourceFeatureBinding, ValidatedCageGeometry, enumerate_source_features,
    };
    use crate::binding_persistence::PersistedBinding;
    use crate::embedding::validate_cage_embedding;
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceDescriptor, CoordinateSpaceId,
        CoordinateSpaceKind, GeometryRevision, HostDocumentId, HostNamespace, HostTargetId,
        HostTargetKind, HostTargetRef, TopologyRevision,
    };
    use crate::path2d::PATH2D_PAYLOAD_KIND;
    use crate::point_location::{locate_point, PointLocationOutcome};
    use crate::topology::{
        ControlPointId, ControlPointPosition, ControlPointState, ControlStateRevision,
        DeformationControlState, DeformationTopologyId, DeformationTopologyRevision,
        IrregularCageTopology, TopologyFace, TopologyView,
    };

    fn point(x: f64, y: f64) -> Point2 { Point2::new(x, y).unwrap() }

    fn target() -> HostTargetRef {
        HostTargetRef {
            namespace: HostNamespace::new("anicraft").unwrap(),
            document_id: HostDocumentId::new("evaluation-doc").unwrap(),
            target_kind: HostTargetKind::new("shape").unwrap(),
            target_id: HostTargetId::new("shape-1").unwrap(),
            generation: 1,
        }
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

    fn path() -> Path2D {
        Path2D::new(vec![
            PathCommand::MoveTo(point(0.25, 0.25)),
            PathCommand::LineTo(point(1.0, 0.25)),
            PathCommand::CubicTo {
                control1: point(1.25, 0.5),
                control2: point(1.25, 1.0),
                to: point(0.5, 1.0),
            },
            PathCommand::Close,
        ]).unwrap()
    }

    fn cage() -> (DeformationTopology, DeformationControlState) {
        let ids = (1..=4).map(|value| ControlPointId::new(value).unwrap()).collect::<Vec<_>>();
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: DeformationTopologyId::new(1).unwrap(),
            revision: DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids: ids.clone(),
            faces: vec![
                TopologyFace::Triangle([ids[0], ids[1], ids[2]]),
                TopologyFace::Triangle([ids[2], ids[1], ids[3]]),
            ],
        });
        let positions = [(0.0,0.0),(2.0,0.0),(0.0,2.0),(2.0,2.0)];
        let state = DeformationControlState {
            topology_id: topology.topology_id(),
            topology_revision: topology.topology_revision(),
            revision: ControlStateRevision::new(1).unwrap(),
            points: ids.into_iter().zip(positions).map(|(id,(x,y))| ControlPointState {
                id, position: ControlPointPosition::new(x,y).unwrap(), pinned:false,
            }).collect(),
        };
        (topology,state)
    }

    fn snapshot(path: Path2D) -> GeometrySnapshot {
        let space = space();
        let fingerprint = path.fingerprint(&space).unwrap();
        GeometrySnapshot::new(target(), GeometryRevision(1), TopologyRevision(1),
            fingerprint, space, PATH2D_PAYLOAD_KIND, 1, path).unwrap()
    }

    fn validated_fixture() -> (GeometrySnapshot, DeformationTopology, DeformationControlState, ValidatedBindingRecord) {
        let source = snapshot(path());
        let (topology, rest_state) = cage();
        let rest_certificate = validate_cage_embedding(&topology, &rest_state).unwrap();
        let rest_geometry = ValidatedCageGeometry::new(&topology, &rest_state, &rest_certificate).unwrap();
        let stamp = BindingStamp::from_snapshot(&source, &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::UnboundZeroInfluence).unwrap();

        let mut entries = Vec::new();
        for feature in enumerate_source_features(source.payload()).unwrap() {
            let status = match locate_point(feature.point, &rest_geometry).unwrap() {
                PointLocationOutcome::Located(location) => {
                    let face = topology.face_at(location.face_index).unwrap();
                    SourceFeatureBinding::Mapped(crate::binding::MappingRecord {
                        locator: feature.locator,
                        face_index: location.face_index as u32,
                        subface_index: 0,
                        control_point_ids: face.control_point_ids().to_vec(),
                        coordinates: location.coordinates,
                    })
                }
                PointLocationOutcome::OutsideCage => SourceFeatureBinding::Unbound,
            };
            entries.push(FeatureBindingEntry { locator: feature.locator, status });
        }

        let persisted = PersistedBinding::new(BindingRecord::new(stamp, entries)).unwrap();
        let validated = persisted.validate_against(&source, &topology, &rest_state).unwrap();
        (source, topology, rest_state, validated)
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
    fn triangle_bindings_follow_current_cage_pose_without_accumulation() {
        let (source, topology, rest_state, validated) = validated_fixture();
        let moved = translated_state(&topology, &rest_state, 10.0, -3.0);
        let certificate = validate_cage_embedding(&topology, &moved).unwrap();
        let current_geometry = ValidatedCageGeometry::new(&topology, &moved, &certificate).unwrap();

        let first = evaluate_binding(&validated, &source, &topology, &current_geometry).unwrap();
        let second = evaluate_binding(&validated, &source, &topology, &current_geometry).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.geometry().commands().len(), source.payload().commands().len());
        assert_eq!(first.mapped_features(), enumerate_source_features(source.payload()).unwrap().len());
        assert_eq!(first.unbound_features(), 0);

        let PathCommand::MoveTo(moved_start) = first.geometry().commands()[0] else { panic!("expected MoveTo"); };
        assert!((moved_start.x() - 10.25).abs() < 1.0e-9);
        assert!((moved_start.y() + 2.75).abs() < 1.0e-9);
        assert_eq!(source.payload(), &path());
    }

    #[test]
    fn stale_source_and_wrong_cage_topology_are_rejected() {
        let (source, topology, rest_state, validated) = validated_fixture();
        let moved = translated_state(&topology, &rest_state, 0.5, 0.0);
        let certificate = validate_cage_embedding(&topology, &moved).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology, &moved, &certificate).unwrap();

        let stale_source_path = Path2D::new(vec![PathCommand::MoveTo(point(0.3, 0.25))]).unwrap();
        let stale_space = space();
        let stale_source = GeometrySnapshot::new(target(), GeometryRevision(2), TopologyRevision(1),
            stale_source_path.fingerprint(&stale_space).unwrap(), stale_space,
            PATH2D_PAYLOAD_KIND, 1, stale_source_path).unwrap();
        assert!(matches!(evaluate_binding(&validated, &stale_source, &topology, &geometry),
            Err(BindingEvaluationError::Binding(BindingError::StaleGeometryRevision))));

        let wrong_topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: DeformationTopologyId::new(99).unwrap(),
            revision: DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids: topology.control_point_ids().to_vec(),
            faces: vec![topology.face_at(0).unwrap(), topology.face_at(1).unwrap()],
        });
        assert_eq!(evaluate_binding(&validated, &source, &wrong_topology, &geometry),
            Err(BindingEvaluationError::Binding(BindingError::StaleCageTopology)));
    }

    #[test]
    fn unbound_features_keep_their_authored_positions() {
        let (source, topology, rest_state, validated) = validated_fixture();
        let moved = translated_state(&topology, &rest_state, 4.0, 7.0);
        let certificate = validate_cage_embedding(&topology, &moved).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology, &moved, &certificate).unwrap();

        let mut entries = validated.binding().entries().to_vec();
        entries[0].status = SourceFeatureBinding::Unbound;
        let stamp = validated.binding().stamp().clone();
        let persisted = PersistedBinding::new(BindingRecord::new(stamp, entries)).unwrap();
        let unbound = persisted.validate_against(&source, &topology, &rest_state).unwrap();

        let result = evaluate_binding(&unbound, &source, &topology, &geometry).unwrap();
        let PathCommand::MoveTo(start) = result.geometry().commands()[0] else { panic!("expected MoveTo"); };
        assert_eq!(start, point(0.25, 0.25));
        assert_eq!(result.unbound_features(), 1);
    }
    #[test]
    fn quad_bilinear_binding_uses_all_four_current_control_points() {
        let ids = (1..=4).map(|value| ControlPointId::new(value).unwrap()).collect::<Vec<_>>();
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: DeformationTopologyId::new(2).unwrap(),
            revision: DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids: ids.clone(),
            faces: vec![TopologyFace::Quad([ids[0], ids[1], ids[2], ids[3]])],
        });
        let make_state = |revision, positions: [(f64, f64); 4]| DeformationControlState {
            topology_id: topology.topology_id(),
            topology_revision: topology.topology_revision(),
            revision: ControlStateRevision::new(revision).unwrap(),
            points: ids.iter().copied().zip(positions).map(|(id, (x, y))| ControlPointState {
                id,
                position: ControlPointPosition::new(x, y).unwrap(),
                pinned: false,
            }).collect(),
        };
        let rest_state = make_state(1, [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)]);
        let source_path = Path2D::new(vec![PathCommand::MoveTo(point(0.5, 0.5))]).unwrap();
        let source = snapshot(source_path);
        let rest_certificate = validate_cage_embedding(&topology, &rest_state).unwrap();
        let rest_geometry = ValidatedCageGeometry::new(&topology, &rest_state, &rest_certificate).unwrap();
        let feature = enumerate_source_features(source.payload()).unwrap()[0];
        let PointLocationOutcome::Located(location) = locate_point(feature.point, &rest_geometry).unwrap() else {
            panic!("expected point inside quad");
        };
        assert!(matches!(location.coordinates, FaceCoordinates::QuadBilinear { .. }));
        let stamp = BindingStamp::from_snapshot(
            &source,
            &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::UnboundZeroInfluence,
        ).unwrap();
        let record = BindingRecord::new(stamp, vec![FeatureBindingEntry {
            locator: feature.locator,
            status: SourceFeatureBinding::Mapped(crate::binding::MappingRecord {
                locator: feature.locator,
                face_index: 0,
                subface_index: 0,
                control_point_ids: ids,
                coordinates: location.coordinates,
            }),
        }]);
        let persisted = PersistedBinding::new(record).unwrap();
        let validated = persisted.validate_against(&source, &topology, &rest_state).unwrap();

        let current_state = make_state(2, [(0.0, 0.0), (4.0, 0.0), (3.0, 3.0), (0.0, 2.0)]);
        let current_certificate = validate_cage_embedding(&topology, &current_state).unwrap();
        let current_geometry = ValidatedCageGeometry::new(&topology, &current_state, &current_certificate).unwrap();
        let evaluated = evaluate_binding(&validated, &source, &topology, &current_geometry).unwrap();

        let PathCommand::MoveTo(result) = evaluated.geometry().commands()[0] else {
            panic!("expected evaluated MoveTo");
        };
        assert!((result.x() - 0.9375).abs() < 1.0e-9);
        assert!((result.y() - 0.9375).abs() < 1.0e-9);
    }

}
