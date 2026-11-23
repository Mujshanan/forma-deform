//! Durable cage-binding records and fail-closed source freshness checks.
//!
//! The record stores the cage identity contract and barycentric attachments,
//! not evaluated geometry or runtime caches. Host revisions and canonical
//! fingerprints are supplied by the host adapter and compared on load/evaluate.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::host_contract::{
    CoordinateSpaceDescriptor, GeometrySnapshotStamp, HostTargetRef, SnapshotFreshness,
};

pub const MAX_CAGE_BINDING_RECORDS: usize = 10_000;
pub const MAX_BOUND_POINTS_PER_CAGE: usize = 1_000_000;
pub const MAX_CAGE_VERTICES_PER_BINDING: usize = 100_000;
pub const MAX_CAGE_TRIANGLES_PER_BINDING: usize = 200_000;
const WEIGHT_SUM_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedCageBinding {
    id: String,
    revision: u64,
    target: HostTargetRef,
    reference_geometry: GeometrySnapshotStamp,
    coordinate_space: CoordinateSpaceDescriptor,
    cage_vertex_ids: Vec<u64>,
    /// Canonical sorted stable vertex IDs for each triangle; order-independent.
    cage_triangles: Vec<[u64; 3]>,
    points: Vec<PersistedBoundPoint>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistedBoundPoint {
    pub point_id: u64,
    pub cage_vertex_ids: [u64; 3],
    pub barycentric: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingFreshness {
    Current,
    GeometryChanged,
    TopologyChanged,
    FingerprintChanged,
    GeometryRevisionRegressed,
    TopologyRevisionRegressed,
    FingerprintIntegrityError,
    TargetMismatch,
}

impl BindingFreshness {
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
        cage_vertex_ids: Vec<u64>,
        cage_triangles: Vec<[u64; 3]>,
        points: Vec<PersistedBoundPoint>,
    ) -> Result<Self, CageBindingPersistenceError> {
        let value = Self {
            id: id.into(), revision, target, reference_geometry, coordinate_space,
            cage_vertex_ids, cage_triangles, points,
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
            || self.cage_vertex_ids.len() > MAX_CAGE_VERTICES_PER_BINDING
            || self.cage_triangles.is_empty()
            || self.cage_triangles.len() > MAX_CAGE_TRIANGLES_PER_BINDING
            || self.points.len() > MAX_BOUND_POINTS_PER_CAGE {
            return Err(CageBindingPersistenceError::ResourceLimitExceeded);
        }
        let mut vertices = BTreeSet::new();
        for id in &self.cage_vertex_ids {
            if !vertices.insert(*id) { return Err(CageBindingPersistenceError::DuplicateCageVertexId); }
        }
        if !self.cage_vertex_ids.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(CageBindingPersistenceError::NonCanonicalVertexIds);
        }
        let mut triangles = BTreeSet::new();
        for triangle in &self.cage_triangles {
            if triangle[0] >= triangle[1] || triangle[1] >= triangle[2]
                || triangle.iter().any(|id| !vertices.contains(id)) {
                return Err(CageBindingPersistenceError::InvalidTriangleIdentity);
            }
            if !triangles.insert(*triangle) {
                return Err(CageBindingPersistenceError::DuplicateTriangleIdentity);
            }
        }
        let mut point_ids = BTreeSet::new();
        for point in &self.points {
            if !point_ids.insert(point.point_id) {
                return Err(CageBindingPersistenceError::DuplicatePointId);
            }
            let mut support = point.cage_vertex_ids;
            support.sort_unstable();
            if support[0] == support[1] || support[1] == support[2]
                || !triangles.contains(&support) {
                return Err(CageBindingPersistenceError::InvalidPointSupport);
            }
            if point.barycentric.iter().any(|weight| !weight.is_finite() || *weight < -WEIGHT_SUM_TOLERANCE)
                || (point.barycentric.iter().sum::<f64>() - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
                return Err(CageBindingPersistenceError::InvalidBarycentricWeights);
            }
        }
        Ok(())
    }

    pub fn freshness_against(&self, current: &GeometrySnapshotStamp) -> BindingFreshness {
        self.freshness_against_cage(current, &self.cage_vertex_ids, &self.cage_triangles)
    }

    /// Compare both the host snapshot stamp and the actual cage identity
    /// envelope. This catches adapters that accidentally fail to advance a
    /// topology revision when the cage's stable topology has changed.
    pub fn freshness_against_cage(
        &self,
        current: &GeometrySnapshotStamp,
        current_vertex_ids: &[u64],
        current_triangles: &[[u64; 3]],
    ) -> BindingFreshness {
        if self.target != *current.target() { return BindingFreshness::TargetMismatch; }
        let mut vertex_ids = current_vertex_ids.to_vec();
        vertex_ids.sort_unstable();
        if vertex_ids != self.cage_vertex_ids { return BindingFreshness::CageVertexSetChanged; }
        let mut triangles = current_triangles.to_vec();
        for triangle in &mut triangles { triangle.sort_unstable(); }
        triangles.sort_unstable();
        if triangles != self.cage_triangles { return BindingFreshness::CageTopologyChanged; }
        let Ok(freshness) = SnapshotFreshness::compare(&self.reference_geometry, current) else {
            return BindingFreshness::TargetMismatch;
        };
        if freshness.geometry_revision_regressed { return BindingFreshness::GeometryRevisionRegressed; }
        if freshness.topology_revision_regressed { return BindingFreshness::TopologyRevisionRegressed; }
        if freshness.is_fingerprint_integrity_error() { return BindingFreshness::FingerprintIntegrityError; }
        if freshness.topology_changed { return BindingFreshness::TopologyChanged; }
        if freshness.geometry_changed { return BindingFreshness::GeometryChanged; }
        if freshness.fingerprint_changed { return BindingFreshness::FingerprintChanged; }
        BindingFreshness::Current
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn target(&self) -> &HostTargetRef { &self.target }
    pub fn reference_geometry(&self) -> &GeometrySnapshotStamp { &self.reference_geometry }
    pub fn coordinate_space(&self) -> &CoordinateSpaceDescriptor { &self.coordinate_space }
    pub fn cage_vertex_ids(&self) -> &[u64] { &self.cage_vertex_ids }
    pub fn cage_triangles(&self) -> &[[u64; 3]] { &self.cage_triangles }
    pub fn points(&self) -> &[PersistedBoundPoint] { &self.points }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CageBindingPersistenceError {
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
}

impl std::fmt::Display for CageBindingPersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cage binding persistence validation failed: {self:?}")
    }
}
impl std::error::Error for CageBindingPersistenceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceId, CoordinateSpaceKind,
        GeometryFingerprint, GeometryRevision, HostDocumentId, HostNamespace,
        HostTargetId, HostTargetKind, TopologyRevision,
    };

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
            id: CoordinateSpaceId::new("local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest, dimensions: 2,
            linear_unit: "px".into(), canonical_unit: "px".into(), unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown, origin: CoordinateOrigin::TopLeft,
        }
    }
    fn stamp(geometry: u64, topology: u64, digest: u8) -> GeometrySnapshotStamp {
        GeometrySnapshotStamp::new(target(), GeometryRevision(geometry), TopologyRevision(topology),
            GeometryFingerprint { algorithm: "sha256-cage-test".into(), digest: vec![digest; 32] }).unwrap()
    }
    fn record() -> PersistedCageBinding {
        PersistedCageBinding::new("binding-1", 1, target(), stamp(1, 1, 7), space(),
            vec![10, 20, 30], vec![[10, 20, 30]],
            vec![PersistedBoundPoint { point_id: 5, cage_vertex_ids: [10, 20, 30], barycentric: [0.2, 0.3, 0.5] }]).unwrap()
    }

    #[test]
    fn record_validates_and_round_trips() {
        let value = record();
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<PersistedCageBinding>(&json).unwrap(), value);
    }
    #[test]
    fn freshness_distinguishes_current_geometry_topology_and_fingerprint() {
        let value = record();
        assert_eq!(value.freshness_against(&stamp(1, 1, 7)), BindingFreshness::Current);
        assert_eq!(value.freshness_against_cage(&stamp(1, 1, 7), &[10, 20, 30], &[[10, 20, 30]]), BindingFreshness::Current);
        assert_eq!(value.freshness_against_cage(&stamp(1, 1, 7), &[10, 20, 40], &[[10, 20, 40]]), BindingFreshness::CageVertexSetChanged);
        assert_eq!(value.freshness_against_cage(&stamp(1, 1, 7), &[10, 20, 30], &[[10, 20, 40]]), BindingFreshness::CageTopologyChanged);
        assert_eq!(value.freshness_against(&stamp(2, 1, 7)), BindingFreshness::GeometryChanged);
        assert_eq!(value.freshness_against(&stamp(1, 2, 7)), BindingFreshness::TopologyChanged);
        assert_eq!(value.freshness_against(&stamp(1, 1, 8)), BindingFreshness::FingerprintIntegrityError);
        let older = PersistedCageBinding::new(
            "binding-older", 1, target(), stamp(2, 2, 7), space(),
            vec![10, 20, 30], vec![[10, 20, 30]],
            vec![PersistedBoundPoint { point_id: 5, cage_vertex_ids: [10, 20, 30], barycentric: [0.2, 0.3, 0.5] }],
        ).unwrap();
        assert_eq!(older.freshness_against(&stamp(1, 2, 7)), BindingFreshness::GeometryRevisionRegressed);
        assert_eq!(older.freshness_against(&stamp(2, 1, 7)), BindingFreshness::TopologyRevisionRegressed);
    }
    #[test]
    fn invalid_weights_and_support_fail_closed() {
        let mut value = record();
        value.points[0].barycentric = [0.2, 0.3, 0.6];
        assert_eq!(value.validate(), Err(CageBindingPersistenceError::InvalidBarycentricWeights));
        let mut value = record();
        value.points[0].cage_vertex_ids = [10, 20, 99];
        assert_eq!(value.validate(), Err(CageBindingPersistenceError::InvalidPointSupport));
    }
}
