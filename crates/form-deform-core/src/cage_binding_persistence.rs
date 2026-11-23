//! Durable generic point-array cage bindings over the canonical FDM topology.
//!
//! The host owns document storage and transactions. This module owns the
//! versioned payload, freshness diagnostics, and validation capability.

use serde::{Deserialize, Serialize};

use crate::cage_binding::{
    BindingPoint, BoundPoint, CageBinding, CageBindingError,
    canonical_triangle_identities, canonical_vertex_ids, validate_topology_view,
};
use crate::host_contract::{
    CoordinateSpaceDescriptor, GeometrySnapshot, GeometrySnapshotStamp,
    HostTargetRef, SnapshotFreshness,
};
use crate::topology::{
    ControlPointId, DeformationTopology,
    DeformationTopologyId, DeformationTopologyRevision, TopologyView,
};
use crate::binding::ValidatedCageGeometry;

pub const CAGE_BINDING_SCHEMA_NAME: &str = "forma-deform-point-cage-binding";
pub const CAGE_BINDING_SCHEMA_VERSION: u32 = 1;
pub const MAX_CAGE_BINDING_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_BOUND_POINTS: usize = 1_000_000;
pub const MAX_CAGE_VERTICES: usize = 100_000;
pub const MAX_CAGE_TRIANGLES: usize = 200_000;
const WEIGHT_SUM_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CageBindingEnvelope {
    schema_name: String,
    schema_version: u32,
    binding: PersistedCageBinding,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedCageBinding {
    id: String,
    revision: u64,
    target: HostTargetRef,
    reference_geometry: GeometrySnapshotStamp,
    coordinate_space: CoordinateSpaceDescriptor,
    topology_id: DeformationTopologyId,
    topology_revision: DeformationTopologyRevision,
    cage_vertex_ids: Vec<ControlPointId>,
    cage_triangles: Vec<[ControlPointId; 3]>,
    points: Vec<PersistedBoundPoint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedBoundPoint {
    pub point_id: u64,
    pub cage_vertex_ids: [ControlPointId; 3],
    pub barycentric: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CageBindingFreshness {
    Current,
    GeometryChanged,
    TopologyChanged,
    FingerprintChanged,
    GeometryRevisionRegressed,
    TopologyRevisionRegressed,
    FingerprintIntegrityError,
    TargetMismatch,
    CageTopologyChanged,
    CageVertexSetChanged,
}

impl CageBindingFreshness {
    pub fn is_current(self) -> bool { self == Self::Current }
    pub fn requires_rebind(self) -> bool { self != Self::Current }
}

impl PersistedCageBinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        revision: u64,
        target: HostTargetRef,
        reference_geometry: GeometrySnapshotStamp,
        coordinate_space: CoordinateSpaceDescriptor,
        topology_id: DeformationTopologyId,
        topology_revision: DeformationTopologyRevision,
        mut cage_vertex_ids: Vec<ControlPointId>,
        mut cage_triangles: Vec<[ControlPointId; 3]>,
        mut points: Vec<PersistedBoundPoint>,
    ) -> Result<Self, CageBindingPersistenceError> {
        cage_vertex_ids.sort_unstable();
        for triangle in &mut cage_triangles { triangle.sort_unstable(); }
        cage_triangles.sort_unstable();
        for point in &mut points {
            let mut pairs = [
                (point.cage_vertex_ids[0], point.barycentric[0]),
                (point.cage_vertex_ids[1], point.barycentric[1]),
                (point.cage_vertex_ids[2], point.barycentric[2]),
            ];
            pairs.sort_by_key(|pair| pair.0);
            point.cage_vertex_ids = [pairs[0].0, pairs[1].0, pairs[2].0];
            point.barycentric = [pairs[0].1, pairs[1].1, pairs[2].1];
        }
        let value = Self {
            id: id.into(), revision, target, reference_geometry, coordinate_space,
            topology_id, topology_revision, cage_vertex_ids, cage_triangles, points,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), CageBindingPersistenceError> {
        if self.id.trim().is_empty() { return Err(CageBindingPersistenceError::EmptyId); }
        if self.revision == 0 { return Err(CageBindingPersistenceError::InvalidRevision); }
        self.target.validate().map_err(|_| CageBindingPersistenceError::InvalidTarget)?;
        self.reference_geometry.validate().map_err(|_| CageBindingPersistenceError::InvalidReferenceGeometry)?;
        self.coordinate_space.validate().map_err(|_| CageBindingPersistenceError::InvalidCoordinateSpace)?;
        if self.reference_geometry.target() != &self.target {
            return Err(CageBindingPersistenceError::ReferenceTargetMismatch);
        }
        if self.cage_vertex_ids.is_empty()
            || self.cage_vertex_ids.len() > MAX_CAGE_VERTICES
            || self.cage_triangles.is_empty()
            || self.cage_triangles.len() > MAX_CAGE_TRIANGLES
            || self.points.len() > MAX_BOUND_POINTS
        {
            return Err(CageBindingPersistenceError::ResourceLimitExceeded);
        }
        let mut vertices = std::collections::BTreeSet::new();
        for id in &self.cage_vertex_ids {
            if !vertices.insert(*id) { return Err(CageBindingPersistenceError::DuplicateCageVertexId); }
        }
        if !self.cage_vertex_ids.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(CageBindingPersistenceError::NonCanonicalVertexIds);
        }
        let mut triangles = std::collections::BTreeSet::new();
        for triangle in &self.cage_triangles {
            if triangle[0] >= triangle[1] || triangle[1] >= triangle[2]
                || triangle.iter().any(|id| !vertices.contains(id))
            {
                return Err(CageBindingPersistenceError::InvalidTriangleIdentity);
            }
            if !triangles.insert(*triangle) {
                return Err(CageBindingPersistenceError::DuplicateTriangleIdentity);
            }
        }
        let mut point_ids = std::collections::BTreeSet::new();
        for point in &self.points {
            if !point_ids.insert(point.point_id) {
                return Err(CageBindingPersistenceError::DuplicatePointId);
            }
            if point.cage_vertex_ids[0] >= point.cage_vertex_ids[1]
                || point.cage_vertex_ids[1] >= point.cage_vertex_ids[2]
                || !triangles.contains(&point.cage_vertex_ids)
            {
                return Err(CageBindingPersistenceError::InvalidPointSupport);
            }
            if point.barycentric.iter().any(|weight| !weight.is_finite() || *weight < -WEIGHT_SUM_TOLERANCE)
                || (point.barycentric.iter().sum::<f64>() - 1.0).abs() > WEIGHT_SUM_TOLERANCE
            {
                return Err(CageBindingPersistenceError::InvalidBarycentricWeights);
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, CageBindingPersistenceError> {
        self.validate()?;
        let envelope = CageBindingEnvelope {
            schema_name: CAGE_BINDING_SCHEMA_NAME.to_owned(),
            schema_version: CAGE_BINDING_SCHEMA_VERSION,
            binding: self.clone(),
        };
        let json = serde_json::to_string(&envelope)
            .map_err(|_| CageBindingPersistenceError::Serialization)?;
        if json.len() > MAX_CAGE_BINDING_BYTES {
            return Err(CageBindingPersistenceError::ResourceLimitExceeded);
        }
        Ok(json)
    }

    pub fn from_json(input: &str) -> Result<Self, CageBindingPersistenceError> {
        if input.len() > MAX_CAGE_BINDING_BYTES {
            return Err(CageBindingPersistenceError::ResourceLimitExceeded);
        }
        let header: serde_json::Value = serde_json::from_str(input)
            .map_err(|_| CageBindingPersistenceError::MalformedPayload)?;
        if header.get("schema_name").and_then(serde_json::Value::as_str)
            != Some(CAGE_BINDING_SCHEMA_NAME)
        {
            return Err(CageBindingPersistenceError::UnsupportedSchemaName);
        }
        if header.get("schema_version").and_then(serde_json::Value::as_u64)
            != Some(u64::from(CAGE_BINDING_SCHEMA_VERSION))
        {
            return Err(CageBindingPersistenceError::UnsupportedSchemaVersion);
        }
        let envelope: CageBindingEnvelope = serde_json::from_value(header)
            .map_err(|_| CageBindingPersistenceError::MalformedPayload)?;
        envelope.binding.validate()?;
        Ok(envelope.binding)
    }

    pub fn freshness_against(
        &self,
        current: &GeometrySnapshotStamp,
        topology: &DeformationTopology,
    ) -> CageBindingFreshness {
        if self.target != *current.target() { return CageBindingFreshness::TargetMismatch; }
        if self.topology_id != topology.topology_id()
            || self.topology_revision != topology.topology_revision()
        {
            return CageBindingFreshness::CageTopologyChanged;
        }
        if canonical_vertex_ids(topology) != self.cage_vertex_ids {
            return CageBindingFreshness::CageVertexSetChanged;
        }
        let Ok(triangles) = canonical_triangle_identities(topology) else {
            return CageBindingFreshness::CageTopologyChanged;
        };
        if triangles != self.cage_triangles {
            return CageBindingFreshness::CageTopologyChanged;
        }
        let Ok(freshness) = SnapshotFreshness::compare(&self.reference_geometry, current) else {
            return CageBindingFreshness::TargetMismatch;
        };
        if freshness.geometry_revision_regressed { return CageBindingFreshness::GeometryRevisionRegressed; }
        if freshness.topology_revision_regressed { return CageBindingFreshness::TopologyRevisionRegressed; }
        if freshness.is_fingerprint_integrity_error() { return CageBindingFreshness::FingerprintIntegrityError; }
        if freshness.topology_changed { return CageBindingFreshness::TopologyChanged; }
        if freshness.geometry_changed { return CageBindingFreshness::GeometryChanged; }
        if freshness.fingerprint_changed { return CageBindingFreshness::FingerprintChanged; }
        CageBindingFreshness::Current
    }

    /// Validate persisted supports against the exact current source points and
    /// a freshly certified rest cage. This never rebinds stale data.
    pub fn validate_against(
        &self,
        snapshot: &GeometrySnapshot,
        topology: &DeformationTopology,
        rest_cage: &ValidatedCageGeometry,
        source_points: &[BindingPoint],
    ) -> Result<ValidatedCageBinding, CageBindingDiagnostic> {
        snapshot.validate_envelope()
            .map_err(|_| CageBindingDiagnostic::InvalidCurrentSnapshot)?;
        self.validate().map_err(CageBindingDiagnostic::Persistence)?;
        if snapshot.target() != &self.target
            || snapshot.coordinate_space() != &self.coordinate_space
        {
            return Err(CageBindingDiagnostic::TargetOrCoordinateSpaceMismatch);
        }
        match self.freshness_against(&snapshot.stamp(), topology) {
            CageBindingFreshness::Current => {}
            stale => return Err(CageBindingDiagnostic::Stale(stale)),
        }
        validate_topology_view(topology, rest_cage)
            .map_err(CageBindingDiagnostic::RuntimeBinding)?;
        if rest_cage.topology_id() != self.topology_id
            || rest_cage.topology_revision() != self.topology_revision
        {
            return Err(CageBindingDiagnostic::Stale(CageBindingFreshness::CageTopologyChanged));
        }

        let mut source_by_id = std::collections::BTreeMap::new();
        for point in source_points {
            if source_by_id.insert(point.id, point.position).is_some() {
                return Err(CageBindingDiagnostic::RuntimeBinding(
                    CageBindingError::DuplicatePointId { point: point.id },
                ));
            }
        }
        if source_by_id.len() != self.points.len()
            || self.points.iter().any(|point| !source_by_id.contains_key(&point.point_id))
        {
            return Err(CageBindingDiagnostic::PointSetMismatch);
        }

        let runtime_points = self.points.iter().map(|point| BoundPoint {
            id: point.point_id,
            cage_vertex_ids: point.cage_vertex_ids,
            barycentric: point.barycentric,
        }).collect();
        let runtime = CageBinding::from_persisted_parts(
            self.topology_id,
            self.topology_revision,
            self.cage_vertex_ids.clone(),
            self.cage_triangles.clone(),
            runtime_points,
        );
        let reconstructed = runtime.evaluate(topology, rest_cage)
            .map_err(CageBindingDiagnostic::RuntimeBinding)?;
        let tolerance = spatial_tolerance(rest_cage, &self.cage_vertex_ids)?;
        for point in reconstructed {
            let expected = source_by_id.get(&point.id).ok_or(CageBindingDiagnostic::PointSetMismatch)?;
            let distance = (point.position.x() - expected.x())
                .hypot(point.position.y() - expected.y());
            if !distance.is_finite() || distance > tolerance {
                return Err(CageBindingDiagnostic::PointReconstructionMismatch { point: point.id });
            }
        }

        Ok(ValidatedCageBinding {
            binding: runtime,
            rest_cage_fingerprint: *rest_cage.geometry_fingerprint(),
        })
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn target(&self) -> &HostTargetRef { &self.target }
    pub fn reference_geometry(&self) -> &GeometrySnapshotStamp { &self.reference_geometry }
    pub fn coordinate_space(&self) -> &CoordinateSpaceDescriptor { &self.coordinate_space }
    pub fn topology_id(&self) -> DeformationTopologyId { self.topology_id }
    pub fn topology_revision(&self) -> DeformationTopologyRevision { self.topology_revision }
    pub fn cage_vertex_ids(&self) -> &[ControlPointId] { &self.cage_vertex_ids }
    pub fn cage_triangles(&self) -> &[[ControlPointId; 3]] { &self.cage_triangles }
    pub fn points(&self) -> &[PersistedBoundPoint] { &self.points }
}

fn spatial_tolerance(
    geometry: &ValidatedCageGeometry,
    ids: &[ControlPointId],
) -> Result<f64, CageBindingDiagnostic> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for id in ids {
        let point = geometry.position(*id)
            .ok_or(CageBindingDiagnostic::RuntimeBinding(CageBindingError::MissingControlPoint(*id)))?;
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    let diagonal = (max_x - min_x).hypot(max_y - min_y);
    if !diagonal.is_finite() {
        return Err(CageBindingDiagnostic::InvalidRestCageGeometry);
    }
    Ok((diagonal * 1.0e-9).max(1.0e-12))
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedCageBinding {
    binding: CageBinding,
    rest_cage_fingerprint: [u8; 32],
}

impl ValidatedCageBinding {
    pub fn binding(&self) -> &CageBinding { &self.binding }
    pub fn rest_cage_fingerprint(&self) -> &[u8; 32] { &self.rest_cage_fingerprint }

    pub fn evaluate(
        &self,
        topology: &DeformationTopology,
        current_cage: &ValidatedCageGeometry,
    ) -> Result<Vec<BindingPoint>, CageBindingError> {
        self.binding.evaluate(topology, current_cage)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CageBindingDiagnostic {
    Stale(CageBindingFreshness),
    InvalidCurrentSnapshot,
    TargetOrCoordinateSpaceMismatch,
    PointSetMismatch,
    PointReconstructionMismatch { point: u64 },
    InvalidRestCageGeometry,
    RuntimeBinding(CageBindingError),
    Persistence(CageBindingPersistenceError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CageBindingPersistenceError {
    MalformedPayload,
    UnsupportedSchemaName,
    UnsupportedSchemaVersion,
    EmptyId,
    InvalidRevision,
    InvalidTarget,
    InvalidReferenceGeometry,
    InvalidCoordinateSpace,
    ReferenceTargetMismatch,
    DuplicateCageVertexId,
    NonCanonicalVertexIds,
    InvalidTriangleIdentity,
    DuplicateTriangleIdentity,
    DuplicatePointId,
    InvalidPointSupport,
    InvalidBarycentricWeights,
    ResourceLimitExceeded,
    Serialization,
}

impl std::fmt::Display for CageBindingPersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cage binding persistence failed: {self:?}")
    }
}
impl std::error::Error for CageBindingPersistenceError {}

impl std::fmt::Display for CageBindingDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cage binding validation failed: {self:?}")
    }
}
impl std::error::Error for CageBindingDiagnostic {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::validate_cage_embedding;
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceId, CoordinateSpaceKind,
        GeometryFingerprint, GeometryRevision, HostDocumentId, HostNamespace,
        HostTargetId, HostTargetKind, TopologyRevision,
    };
    use crate::path2d::{Path2D, PathCommand, Point2};
    use crate::topology::{
        ControlPointPosition, ControlPointState, ControlStateRevision,
        DeformationControlState, IrregularCageTopology,
    };

    fn id(value: u64) -> ControlPointId { ControlPointId::new(value).unwrap() }
    fn point(x: f64, y: f64) -> Point2 { Point2::new(x, y).unwrap() }

    fn target() -> HostTargetRef {
        HostTargetRef {
            namespace: HostNamespace::new("anicraft").unwrap(),
            document_id: HostDocumentId::new("doc-1").unwrap(),
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

    fn snapshot(revision: u64) -> GeometrySnapshot {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(2.0, 3.0)),
            PathCommand::LineTo(point(3.0, 3.0)),
            PathCommand::LineTo(point(3.0, 4.0)),
            PathCommand::Close,
        ]).unwrap();
        let coordinate_space = space();
        let fingerprint = path.fingerprint(&coordinate_space).unwrap();
        GeometrySnapshot::new(
            target(),
            GeometryRevision(revision),
            TopologyRevision(1),
            fingerprint,
            coordinate_space,
            crate::path2d::PATH2D_PAYLOAD_KIND,
            crate::path2d::PATH2D_SCHEMA_VERSION,
            path,
        ).unwrap()
    }

    fn cage() -> (DeformationTopology, ValidatedCageGeometry) {
        let topology_id = DeformationTopologyId::new(80).unwrap();
        let topology_revision = DeformationTopologyRevision::new(1).unwrap();
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: topology_id,
            revision: topology_revision,
            control_point_ids: vec![id(10), id(20), id(30), id(40)],
            faces: vec![
                TopologyFace::Triangle([id(10), id(20), id(30)]),
                TopologyFace::Triangle([id(10), id(30), id(40)]),
            ],
        });
        let state = DeformationControlState {
            topology_id,
            topology_revision,
            revision: ControlStateRevision::new(1).unwrap(),
            points: vec![
                (10, 0.0, 0.0), (20, 10.0, 0.0), (30, 10.0, 10.0), (40, 0.0, 10.0),
            ].into_iter().map(|(id_value, x, y)| ControlPointState {
                id: id(id_value),
                position: ControlPointPosition::new(x, y).unwrap(),
                pinned: false,
            }).collect(),
        };
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology, &state, &certificate).unwrap();
        (topology, geometry)
    }

    fn record() -> (GeometrySnapshot, DeformationTopology, ValidatedCageGeometry, PersistedCageBinding) {
        let source = snapshot(1);
        let (topology, geometry) = cage();
        let points = [BindingPoint { id: 7, position: point(2.0, 3.0) }];
        let runtime = CageBinding::bind(&topology, &geometry, &points).unwrap();
        let persisted = runtime.to_persisted(
            "point-binding-1", 1, target(), source.stamp(), space(),
        ).unwrap();
        (source, topology, geometry, persisted)
    }

    #[test]
    fn point_binding_payload_round_trips_and_validates_against_certified_rest_cage() {
        let (source, topology, geometry, persisted) = record();
        let json = persisted.to_json().unwrap();
        let decoded = PersistedCageBinding::from_json(&json).unwrap();
        assert_eq!(decoded, persisted);
        let validated = decoded.validate_against(
            &source,
            &topology,
            &geometry,
            &[BindingPoint { id: 7, position: point(2.0, 3.0) }],
        ).unwrap();
        assert_eq!(validated.binding().points()[0].id, 7);
        assert_eq!(validated.rest_cage_fingerprint(), geometry.geometry_fingerprint());
    }

    #[test]
    fn stale_source_and_reconstruction_mismatch_fail_closed() {
        let (source, topology, geometry, persisted) = record();
        assert_eq!(
            persisted.freshness_against(&snapshot(2).stamp(), &topology),
            CageBindingFreshness::GeometryChanged,
        );
        assert!(matches!(
            persisted.validate_against(
                &source, &topology, &geometry,
                &[BindingPoint { id: 7, position: point(8.0, 8.0) }],
            ),
            Err(CageBindingDiagnostic::PointReconstructionMismatch { point: 7 })
        ));
    }

    #[test]
    fn invalid_support_and_weights_are_rejected() {
        let (_, _, _, mut persisted) = record();
        persisted.points[0].barycentric = [0.2, 0.3, 0.6];
        assert_eq!(
            persisted.validate(),
            Err(CageBindingPersistenceError::InvalidBarycentricWeights),
        );
        let (_, _, _, mut persisted) = record();
        persisted.points[0].cage_vertex_ids = [id(10), id(20), id(99)];
        assert_eq!(
            persisted.validate(),
            Err(CageBindingPersistenceError::InvalidPointSupport),
        );
    }
}
