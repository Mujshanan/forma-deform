//! Snapshot-bound source-to-cage binding contracts and validation.
//!
//! This module deliberately stops before point location. It validates mapping
//! records supplied by a later point-location stage and never discovers faces.

use crate::embedding::{CageEmbeddingCertificate, EmbeddingError};
use crate::host_contract::{
    CoordinateSpaceDescriptor, GeometrySnapshot, HostTargetRef,
};
use crate::path2d::{PathCommand, Path2D, Point2, PATH2D_PAYLOAD_KIND, PATH2D_SCHEMA_VERSION};
use crate::topology::{
    ControlPointId, ControlPointPosition, DeformationControlState, DeformationTopology,
    DeformationTopologyId, DeformationTopologyRevision, TopologyFace, TopologyView,
};
use std::collections::{BTreeMap, BTreeSet};

pub const BINDING_SCHEMA_NAME: &str = "forma-deform-path2d-binding";
pub const BINDING_SCHEMA_VERSION: u32 = 2;
pub const LEGACY_BINDING_SCHEMA_VERSION: u32 = 1;
pub const MAPPING_POLICY_VERSION: u32 = 1;
pub const MAPPING_NUMERICAL_POLICY_VERSION: u32 = 1;
pub const OUTSIDE_CAGE_POLICY_VERSION: u32 = 1;

// Centralized v1 acceptance tolerances; changing them requires a policy version bump.
const WEIGHT_SUM_TOLERANCE: f64 = 1.0e-9;
const SPATIAL_RELATIVE_TOLERANCE: f64 = 1.0e-9;
const MIN_SPATIAL_TOLERANCE: f64 = 1.0e-12;

/// Triangle barycentric and quad bilinear coordinates are intentionally
/// distinct: a quad is never represented by an implicit diagonal split.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FaceCoordinates {
    TriangleBarycentric { weights: [f64; 3] },
    QuadBilinear { u: f64, v: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathFeatureRole {
    MoveToEndpoint,
    LineToEndpoint,
    QuadraticControl,
    QuadraticEndpoint,
    CubicControl1,
    CubicControl2,
    CubicEndpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceFeatureLocator {
    pub command_index: u32,
    pub role: PathFeatureRole,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceFeaturePoint {
    pub locator: SourceFeatureLocator,
    pub point: Point2,
    pub ordinal: u32,
    /// Zero-based subpath ordinal in the immutable Path2D command stream.
    pub subpath_index: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceFeatureError {
    TooManyCommands,
    TooManyFeatures,
    InvalidPathSequence { command_index: usize },
}

impl std::fmt::Display for SourceFeatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyCommands => write!(f, "Path2D command count exceeds u32 locator capacity"),
            Self::TooManyFeatures => write!(f, "Path2D feature count exceeds u32 ordinal capacity"),
            Self::InvalidPathSequence { command_index } => write!(f, "invalid Path2D sequence at command {command_index}"),
        }
    }
}
impl std::error::Error for SourceFeatureError {}

/// Enumerate authored coordinate-bearing command features without flattening
/// or modifying the source Path2D.
pub fn enumerate_source_features(path: &Path2D) -> Result<Vec<SourceFeaturePoint>, SourceFeatureError> {
    if path.commands().len() > u32::MAX as usize {
        return Err(SourceFeatureError::TooManyCommands);
    }
    let mut result = Vec::new();
    let mut subpath_index = 0u32;
    let mut has_subpath = false;
    let mut has_seen_move_to = false;

    for (command_index, command) in path.commands().iter().enumerate() {
        let command_index = command_index as u32;
        match command {
            PathCommand::MoveTo(point) => {
                if has_seen_move_to {
                    subpath_index = subpath_index.checked_add(1).ok_or(SourceFeatureError::TooManyFeatures)?;
                }
                has_seen_move_to = true;
                has_subpath = true;
                push_source_feature(&mut result, command_index, PathFeatureRole::MoveToEndpoint, *point, subpath_index)?;
            }
            PathCommand::LineTo(point) => {
                if !has_subpath { return Err(SourceFeatureError::InvalidPathSequence { command_index: command_index as usize }); }
                push_source_feature(&mut result, command_index, PathFeatureRole::LineToEndpoint, *point, subpath_index)?;
            }
            PathCommand::QuadraticTo { control, to } => {
                if !has_subpath { return Err(SourceFeatureError::InvalidPathSequence { command_index: command_index as usize }); }
                push_source_feature(&mut result, command_index, PathFeatureRole::QuadraticControl, *control, subpath_index)?;
                push_source_feature(&mut result, command_index, PathFeatureRole::QuadraticEndpoint, *to, subpath_index)?;
            }
            PathCommand::CubicTo { control1, control2, to } => {
                if !has_subpath { return Err(SourceFeatureError::InvalidPathSequence { command_index: command_index as usize }); }
                push_source_feature(&mut result, command_index, PathFeatureRole::CubicControl1, *control1, subpath_index)?;
                push_source_feature(&mut result, command_index, PathFeatureRole::CubicControl2, *control2, subpath_index)?;
                push_source_feature(&mut result, command_index, PathFeatureRole::CubicEndpoint, *to, subpath_index)?;
            }
            PathCommand::Close => {
                if !has_subpath { return Err(SourceFeatureError::InvalidPathSequence { command_index: command_index as usize }); }
                has_subpath = false;
            }
        }
    }
    Ok(result)
}

fn push_source_feature(
    result: &mut Vec<SourceFeaturePoint>,
    command_index: u32,
    role: PathFeatureRole,
    point: Point2,
    subpath_index: u32,
) -> Result<(), SourceFeatureError> {
    if result.len() >= u32::MAX as usize {
        return Err(SourceFeatureError::TooManyFeatures);
    }
    result.push(SourceFeaturePoint {
        locator: SourceFeatureLocator { command_index, role },
        point,
        ordinal: result.len() as u32,
        subpath_index,
    });
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MappingPolicy {
    TriangleBarycentricQuadBilinearInsideV1,
}
impl MappingPolicy {
    pub const fn version(self) -> u32 { MAPPING_POLICY_VERSION }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutsideCagePolicy {
    UnboundZeroInfluence,
    ClampToBoundary,
    BoundedExtrapolation { max_distance: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BindingStamp {
    pub target: HostTargetRef,
    pub geometry_revision: u64,
    pub source_topology_revision: u64,
    pub source_fingerprint_algorithm: String,
    pub source_fingerprint_digest: Vec<u8>,
    pub coordinate_space: CoordinateSpaceDescriptor,
    pub payload_kind: String,
    pub payload_schema_version: u32,
    pub topology_id: DeformationTopologyId,
    pub topology_revision: DeformationTopologyRevision,
    pub schema_name: String,
    pub schema_version: u32,
    pub mapping_policy: MappingPolicy,
    pub numerical_policy_version: u32,
    pub outside_policy: OutsideCagePolicy,
    pub outside_policy_version: u32,
}

impl BindingStamp {
    pub fn from_snapshot(
        snapshot: &GeometrySnapshot,
        topology: &DeformationTopology,
        mapping_policy: MappingPolicy,
        outside_policy: OutsideCagePolicy,
    ) -> Result<Self, BindingError> {
        snapshot.validate_envelope().map_err(|_| BindingError::InvalidSourceSnapshot)?;
        validate_outside_policy(outside_policy)?;
        let fingerprint = snapshot.fingerprint();
        Ok(Self {
            target: snapshot.target().clone(),
            geometry_revision: snapshot.geometry_revision().0,
            source_topology_revision: snapshot.topology_revision().0,
            source_fingerprint_algorithm: fingerprint.algorithm.clone(),
            source_fingerprint_digest: fingerprint.digest.clone(),
            coordinate_space: snapshot.coordinate_space().clone(),
            payload_kind: snapshot.payload_kind().to_owned(),
            payload_schema_version: snapshot.payload_schema_version(),
            topology_id: topology.topology_id(),
            topology_revision: topology.topology_revision(),
            schema_name: BINDING_SCHEMA_NAME.to_owned(),
            schema_version: BINDING_SCHEMA_VERSION,
            mapping_policy,
            numerical_policy_version: MAPPING_NUMERICAL_POLICY_VERSION,
            outside_policy,
            outside_policy_version: OUTSIDE_CAGE_POLICY_VERSION,
        })
    }

    pub fn validate(&self) -> Result<(), BindingError> {
        self.target.validate().map_err(|_| BindingError::InvalidStamp)?;
        self.coordinate_space.validate().map_err(|_| BindingError::InvalidStamp)?;
        if self.geometry_revision == 0 || self.source_topology_revision == 0
            || self.source_fingerprint_algorithm.trim().is_empty()
            || self.source_fingerprint_digest.is_empty()
            || self.payload_kind != PATH2D_PAYLOAD_KIND
            || self.payload_schema_version != PATH2D_SCHEMA_VERSION
            || self.schema_name != BINDING_SCHEMA_NAME
            || !(self.schema_version == BINDING_SCHEMA_VERSION
                || (self.schema_version == LEGACY_BINDING_SCHEMA_VERSION
                    && self.outside_policy == OutsideCagePolicy::UnboundZeroInfluence))
            || self.numerical_policy_version != MAPPING_NUMERICAL_POLICY_VERSION
            || self.outside_policy_version != OUTSIDE_CAGE_POLICY_VERSION
        {
            return Err(BindingError::InvalidStamp);
        }
        validate_outside_policy(self.outside_policy)?;
        Ok(())
    }

    /// Compare all rest-space dependencies. Control-state revision is
    /// intentionally absent: it invalidates geometry certificates/cache keys,
    /// not the source-to-rest-cage association.
    pub fn validate_freshness(
        &self,
        snapshot: &GeometrySnapshot,
        topology: &DeformationTopology,
    ) -> Result<(), BindingError> {
        self.validate()?;
        snapshot.validate_envelope().map_err(|_| BindingError::InvalidSourceSnapshot)?;
        if &self.target != snapshot.target() { return Err(BindingError::StaleTarget); }
        if self.geometry_revision != snapshot.geometry_revision().0 { return Err(BindingError::StaleGeometryRevision); }
        if self.source_topology_revision != snapshot.topology_revision().0 { return Err(BindingError::StaleSourceTopologyRevision); }
        if self.source_fingerprint_algorithm != snapshot.fingerprint().algorithm
            || self.source_fingerprint_digest != snapshot.fingerprint().digest
        { return Err(BindingError::StaleSourceFingerprint); }
        if self.coordinate_space != *snapshot.coordinate_space() { return Err(BindingError::StaleCoordinateSpace); }
        if self.payload_kind != snapshot.payload_kind()
            || self.payload_schema_version != snapshot.payload_schema_version()
        { return Err(BindingError::StalePayloadSchema); }
        if self.topology_id != topology.topology_id()
            || self.topology_revision != topology.topology_revision()
        { return Err(BindingError::StaleCageTopology); }
        Ok(())
    }
}

/// Read-only geometry snapshot. Its constructor is the only path that accepts
/// the private-constructor embedding certificate and verifies it against the
/// exact topology/control-state pair.
#[derive(Clone, Debug)]
pub struct ValidatedCageGeometry {
    topology_id: DeformationTopologyId,
    topology_revision: DeformationTopologyRevision,
    positions: BTreeMap<ControlPointId, ControlPointPosition>,
    faces: Vec<TopologyFace>,
    geometry_fingerprint: [u8; 32],
}

impl ValidatedCageGeometry {
    pub fn new(
        topology: &DeformationTopology,
        control_state: &DeformationControlState,
        certificate: &CageEmbeddingCertificate,
    ) -> Result<Self, BindingError> {
        if !certificate.matches(topology, control_state) {
            return Err(BindingError::UncertifiedCageEmbedding);
        }
        control_state.validate_against(topology).map_err(|_| BindingError::TopologyControlStateMismatch)?;
        let mut positions = BTreeMap::new();
        for point in &control_state.points {
            if positions.insert(point.id, point.position).is_some() {
                return Err(BindingError::DuplicateControlPoint(point.id));
            }
        }
        let mut faces = Vec::with_capacity(topology.face_count());
        for index in 0..topology.face_count() {
            faces.push(topology.face_at(index).ok_or(BindingError::FaceIndexOutOfRange(index))?);
        }
        Ok(Self {
            topology_id: topology.topology_id(),
            topology_revision: topology.topology_revision(),
            positions,
            faces,
            geometry_fingerprint: *certificate.geometry_fingerprint(),
        })
    }

    pub fn topology_id(&self) -> DeformationTopologyId { self.topology_id }
    pub fn topology_revision(&self) -> DeformationTopologyRevision { self.topology_revision }
    pub fn geometry_fingerprint(&self) -> &[u8; 32] { &self.geometry_fingerprint }
    pub fn position(&self, id: ControlPointId) -> Option<ControlPointPosition> { self.positions.get(&id).copied() }
    pub fn face(&self, index: usize) -> Option<TopologyFace> { self.faces.get(index).copied() }
    pub fn face_count(&self) -> usize { self.faces.len() }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappingRecord {
    pub locator: SourceFeatureLocator,
    pub face_index: u32,
    /// Reserved revision-bound subface locator. V1 bilinear/barycentric faces use 0.
    pub subface_index: u8,
    pub control_point_ids: Vec<ControlPointId>,
    pub coordinates: FaceCoordinates,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SourceFeatureBinding {
    Mapped(MappingRecord),
    Clamped(MappingRecord),
    Extrapolated(MappingRecord),
    Unbound,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeatureBindingEntry {
    pub locator: SourceFeatureLocator,
    pub status: SourceFeatureBinding,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BindingRecord {
    stamp: BindingStamp,
    entries: Vec<FeatureBindingEntry>,
}

impl BindingRecord {
    /// Construct a candidate record. It becomes trusted only after
    /// validate_against succeeds for the exact snapshot and certified cage.
    pub fn new(stamp: BindingStamp, entries: Vec<FeatureBindingEntry>) -> Self {
        Self { stamp, entries }
    }

    pub fn stamp(&self) -> &BindingStamp { &self.stamp }
    pub fn entries(&self) -> &[FeatureBindingEntry] { &self.entries }

    pub fn validate_against(
        &self,
        snapshot: &GeometrySnapshot,
        topology: &DeformationTopology,
        geometry: &ValidatedCageGeometry,
    ) -> Result<(), BindingError> {
        self.stamp.validate_freshness(snapshot, topology)?;
        if geometry.topology_id != topology.topology_id()
            || geometry.topology_revision != topology.topology_revision()
        { return Err(BindingError::StaleCageTopology); }
        if self.stamp.mapping_policy != MappingPolicy::TriangleBarycentricQuadBilinearInsideV1 {
            return Err(BindingError::UnsupportedMappingPolicy);
        }

        let features = enumerate_source_features(snapshot.payload()).map_err(BindingError::SourceFeatureEnumeration)?;
        if self.entries.len() != features.len() { return Err(BindingError::IncompleteFeatureCoverage); }

        let mut seen = BTreeSet::new();
        for (feature, entry) in features.iter().zip(&self.entries) {
            if feature.locator != entry.locator || !seen.insert(entry.locator) {
                return Err(BindingError::InvalidSourceFeature(entry.locator));
            }
            match &entry.status {
                SourceFeatureBinding::Mapped(mapping) => {
                    if mapping.locator != entry.locator { return Err(BindingError::InvalidSourceFeature(entry.locator)); }
                    validate_mapping(mapping, feature.point, geometry)?;
                }
                SourceFeatureBinding::Clamped(mapping) => {
                    if mapping.locator != entry.locator || self.stamp.outside_policy != OutsideCagePolicy::ClampToBoundary {
                        return Err(BindingError::OutsidePolicyRecordMismatch);
                    }
                    validate_clamped_mapping(mapping, feature.point, geometry)?;
                }
                SourceFeatureBinding::Extrapolated(mapping) => {
                    if mapping.locator != entry.locator { return Err(BindingError::InvalidSourceFeature(entry.locator)); }
                    let OutsideCagePolicy::BoundedExtrapolation { max_distance } = self.stamp.outside_policy else {
                        return Err(BindingError::OutsidePolicyRecordMismatch);
                    };
                    validate_extrapolated_mapping(mapping, feature.point, geometry, max_distance)?;
                }
                SourceFeatureBinding::Unbound => {
                    if self.stamp.outside_policy != OutsideCagePolicy::UnboundZeroInfluence {
                        return Err(BindingError::OutsidePolicyRecordMismatch);
                    }
                }
            }
        }
        if seen.len() != features.len() { return Err(BindingError::IncompleteFeatureCoverage); }
        Ok(())
    }
}

fn validate_mapping(mapping: &MappingRecord, source_point: Point2, geometry: &ValidatedCageGeometry) -> Result<(), BindingError> {
    validate_mapping_domain(mapping, source_point, geometry, false)
}

fn validate_mapping_domain(
    mapping: &MappingRecord,
    source_point: Point2,
    geometry: &ValidatedCageGeometry,
    allow_extrapolation: bool,
) -> Result<(), BindingError> {
    let face_index = mapping.face_index as usize;
    if mapping.subface_index != 0 { return Err(BindingError::UnsupportedSubface(mapping.subface_index)); }
    let face = geometry.face(face_index).ok_or(BindingError::FaceIndexOutOfRange(face_index))?;
    let ids = face.control_point_ids();
    if mapping.control_point_ids.as_slice() != ids {
        return Err(BindingError::ControlPointIdOrderMismatch { face_index });
    }
    let mut points = Vec::with_capacity(ids.len());
    for id in ids {
        points.push(geometry.position(*id).ok_or(BindingError::MissingControlPoint(*id))?);
    }

    let reconstructed = match (&face, mapping.coordinates) {
        (TopologyFace::Triangle(_), FaceCoordinates::TriangleBarycentric { weights }) => {
            if weights.iter().any(|weight| !weight.is_finite()) { return Err(BindingError::NonFiniteCoordinate); }
            if !allow_extrapolation && weights.iter().any(|weight| *weight < 0.0 || *weight > 1.0) {
                return Err(BindingError::CoordinateOutsideDomain);
            }
            let sum = weights.iter().sum::<f64>();
            if !sum.is_finite() || (sum - 1.0).abs() > weight_sum_tolerance() {
                return Err(BindingError::WeightSumMismatch);
            }
            weighted_sum(&points, &weights)?
        }
        (TopologyFace::Quad(_), FaceCoordinates::QuadBilinear { u, v }) => {
            if !u.is_finite() || !v.is_finite() { return Err(BindingError::NonFiniteCoordinate); }
            if !allow_extrapolation && (!(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v)) {
                return Err(BindingError::CoordinateOutsideDomain);
            }
            bilinear_point(&points, u, v)?
        }
        _ => return Err(BindingError::FaceMappingKindMismatch { face_index }),
    };

    let tolerance = spatial_tolerance(&points)?;
    let dx = reconstructed.x - source_point.x();
    let dy = reconstructed.y - source_point.y();
    if !dx.is_finite() || !dy.is_finite() || dx.hypot(dy) > tolerance {
        return Err(BindingError::SourcePointReconstructionMismatch { face_index });
    }
    Ok(())
}

 
fn coordinates_are_extrapolated(coordinates: FaceCoordinates) -> bool {
    match coordinates {
        FaceCoordinates::TriangleBarycentric { weights } =>
            weights.iter().any(|weight| *weight < 0.0 || *weight > 1.0),
        FaceCoordinates::QuadBilinear { u, v } =>
            !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v),
    }
}

fn validate_outside_policy(policy: OutsideCagePolicy) -> Result<(), BindingError> {
    if let OutsideCagePolicy::BoundedExtrapolation { max_distance } = policy {
        if !max_distance.is_finite() || max_distance <= 0.0 {
            return Err(BindingError::InvalidOutsidePolicyParameter);
        }
    }
    Ok(())
}

fn validate_clamped_mapping(
    mapping: &MappingRecord,
    source_point: Point2,
    geometry: &ValidatedCageGeometry,
) -> Result<(), BindingError> {
    use crate::outside_policy::{resolve_outside_point, OutsideCageResolution, OutsideCageResolutionPolicy};
    match crate::point_location::locate_point(source_point, geometry) {
        Ok(crate::point_location::PointLocationOutcome::OutsideCage) => {}
        _ => return Err(BindingError::OutsidePolicyRecordMismatch),
    }
    let OutsideCageResolution::Clamped(projection) = resolve_outside_point(
        source_point, geometry, OutsideCageResolutionPolicy::ClampToBoundary,
    ).map_err(|_| BindingError::OutsidePolicyRecordMismatch)? else {
        return Err(BindingError::OutsidePolicyRecordMismatch);
    };
    if mapping.face_index != projection.face_index || mapping.coordinates != projection.coordinates {
        return Err(BindingError::OutsidePolicyRecordMismatch);
    }
    validate_mapping_domain(mapping, projection.point, geometry, false)
}

fn validate_extrapolated_mapping(
    mapping: &MappingRecord,
    source_point: Point2,
    geometry: &ValidatedCageGeometry,
    max_distance: f64,
) -> Result<(), BindingError> {
    use crate::outside_policy::{resolve_outside_point, OutsideCageResolution, OutsideCageResolutionPolicy};
    if !max_distance.is_finite() || max_distance <= 0.0 {
        return Err(BindingError::InvalidOutsidePolicyParameter);
    }
    match crate::point_location::locate_point(source_point, geometry) {
        Ok(crate::point_location::PointLocationOutcome::OutsideCage) => {}
        _ => return Err(BindingError::OutsidePolicyRecordMismatch),
    }
    let OutsideCageResolution::Extrapolated { boundary, coordinates, .. } = resolve_outside_point(
        source_point, geometry, OutsideCageResolutionPolicy::BoundedExtrapolation { max_distance },
    ).map_err(|_| BindingError::OutsidePolicyRecordMismatch)? else {
        return Err(BindingError::OutsidePolicyRecordMismatch);
    };
    if mapping.face_index != boundary.face_index || mapping.coordinates != coordinates
        || !coordinates_are_extrapolated(mapping.coordinates) {
        return Err(BindingError::OutsidePolicyRecordMismatch);
    }
    validate_mapping_domain(mapping, source_point, geometry, true)
}

fn weighted_sum(points: &[ControlPointPosition], weights: &[f64; 3]) -> Result<Point2, BindingError> {
    let x = points[0].x * weights[0] + points[1].x * weights[1] + points[2].x * weights[2];
    let y = points[0].y * weights[0] + points[1].y * weights[1] + points[2].y * weights[2];
    Point2::new(x, y).map_err(|_| BindingError::NonFiniteCoordinate)
}

fn bilinear_point(points: &[ControlPointPosition], u: f64, v: f64) -> Result<Point2, BindingError> {
    // Authored quad order is P00, P10, P11, P01.
    let weights = [(1.0-u)*(1.0-v), u*(1.0-v), u*v, (1.0-u)*v];
    let x = points.iter().zip(weights).map(|(p, w)| p.x*w).sum();
    let y = points.iter().zip(weights).map(|(p, w)| p.y*w).sum();
    Point2::new(x, y).map_err(|_| BindingError::NonFiniteCoordinate)
}

fn weight_sum_tolerance() -> f64 { WEIGHT_SUM_TOLERANCE }

fn spatial_tolerance(points: &[ControlPointPosition]) -> Result<f64, BindingError> {
    let first = *points.first().ok_or(BindingError::InvalidFaceGeometry)?;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x); min_y = min_y.min(point.y);
        max_x = max_x.max(point.x); max_y = max_y.max(point.y);
    }
    let diagonal = (max_x-min_x).hypot(max_y-min_y);
    let tolerance = (diagonal * SPATIAL_RELATIVE_TOLERANCE).max(MIN_SPATIAL_TOLERANCE);
    if !diagonal.is_finite() || !tolerance.is_finite() || diagonal <= 0.0 {
        return Err(BindingError::InvalidFaceGeometry);
    }
    Ok(tolerance)
}

#[derive(Clone, Debug, PartialEq)]
pub enum BindingError {
    InvalidSourceSnapshot,
    InvalidStamp,
    UnsupportedOutsidePolicy(OutsideCagePolicy),
    InvalidOutsidePolicyParameter,
    OutsidePolicyRecordMismatch,
    UnsupportedMappingPolicy,
    StaleTarget,
    StaleGeometryRevision,
    StaleSourceTopologyRevision,
    StaleSourceFingerprint,
    StaleCoordinateSpace,
    StalePayloadSchema,
    StaleCageTopology,
    TopologyControlStateMismatch,
    UncertifiedCageEmbedding,
    DuplicateControlPoint(ControlPointId),
    MissingControlPoint(ControlPointId),
    FaceIndexOutOfRange(usize),
    UnsupportedSubface(u8),
    InvalidSourceFeature(SourceFeatureLocator),
    SourceFeatureEnumeration(SourceFeatureError),
    FaceMappingKindMismatch { face_index: usize },
    ControlPointIdOrderMismatch { face_index: usize },
    NonFiniteCoordinate,
    CoordinateOutsideDomain,
    WeightSumMismatch,
    SourcePointReconstructionMismatch { face_index: usize },
    InvalidFaceGeometry,
    IncompleteFeatureCoverage,
    Embedding(EmbeddingError),
}

impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSourceSnapshot => write!(f, "source geometry snapshot is invalid"),
            Self::InvalidStamp => write!(f, "binding stamp is malformed or unsupported"),
            Self::UnsupportedOutsidePolicy(policy) => write!(f, "outside-cage policy {policy:?} is not supported"),
            Self::InvalidOutsidePolicyParameter => write!(f, "outside-cage policy parameter is invalid"),
            Self::OutsidePolicyRecordMismatch => write!(f, "binding entry does not match its stamped outside-cage policy"),
            Self::UnsupportedMappingPolicy => write!(f, "mapping policy is not supported"),
            Self::StaleTarget => write!(f, "binding host target identity or generation is stale"),
            Self::StaleGeometryRevision => write!(f, "binding source geometry revision is stale"),
            Self::StaleSourceTopologyRevision => write!(f, "binding source Path2D topology revision is stale"),
            Self::StaleSourceFingerprint => write!(f, "binding source fingerprint is stale"),
            Self::StaleCoordinateSpace => write!(f, "binding coordinate-space descriptor is stale"),
            Self::StalePayloadSchema => write!(f, "binding Path2D payload kind or schema is stale"),
            Self::StaleCageTopology => write!(f, "binding cage topology identity or revision is stale"),
            Self::TopologyControlStateMismatch => write!(f, "cage control state does not match its topology"),
            Self::UncertifiedCageEmbedding => write!(f, "binding requires a current global cage-embedding certificate"),
            Self::DuplicateControlPoint(id) => write!(f, "duplicate cage control point ID {}", id.get()),
            Self::MissingControlPoint(id) => write!(f, "missing cage control point ID {}", id.get()),
            Self::FaceIndexOutOfRange(index) => write!(f, "binding face index {index} is out of range"),
            Self::UnsupportedSubface(index) => write!(f, "face sub-index {index} is unsupported by the v1 unsplit face policy"),
            Self::InvalidSourceFeature(locator) => write!(f, "invalid or duplicate source feature locator ({}, {:?})", locator.command_index, locator.role),
            Self::SourceFeatureEnumeration(error) => write!(f, "source feature enumeration failed: {error}"),
            Self::FaceMappingKindMismatch { face_index } => write!(f, "mapping coordinate kind does not match face {face_index}"),
            Self::ControlPointIdOrderMismatch { face_index } => write!(f, "mapping control-point IDs do not match authored order on face {face_index}"),
            Self::NonFiniteCoordinate => write!(f, "mapping coordinates or reconstruction are non-finite"),
            Self::CoordinateOutsideDomain => write!(f, "mapping coordinates are outside the inside-only domain"),
            Self::WeightSumMismatch => write!(f, "triangle barycentric weights do not sum to one"),
            Self::SourcePointReconstructionMismatch { face_index } => write!(f, "mapping does not reconstruct the source feature on face {face_index}"),
            Self::InvalidFaceGeometry => write!(f, "face geometry has invalid or degenerate bounds"),
            Self::IncompleteFeatureCoverage => write!(f, "binding does not account for every source feature exactly once"),
            Self::Embedding(error) => write!(f, "cage embedding validation failed: {error}"),
        }
    }
}
impl std::error::Error for BindingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::validate_cage_embedding;
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceId, CoordinateSpaceKind,
        GeometryFingerprint, GeometryRevision, HostDocumentId, HostNamespace,
        HostTargetId, HostTargetKind, TopologyRevision,
    };
    use crate::path2d::{PathCommand, Point2, Path2D};
    use crate::topology::{
        ControlPointState, ControlStateRevision, DeformationTopologyId,
        DeformationTopologyRevision, IrregularCageTopology,
    };

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
    fn coordinate_space() -> CoordinateSpaceDescriptor {
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
            PathCommand::MoveTo(point(0.5, 0.5)),
            PathCommand::LineTo(point(1.0, 0.5)),
            PathCommand::QuadraticTo { control: point(1.0, 1.0), to: point(0.5, 1.0) },
            PathCommand::CubicTo { control1: point(0.25, 1.0), control2: point(0.0, 0.75), to: point(0.5, 0.5) },
            PathCommand::Close,
        ]).unwrap()
    }
    fn snapshot() -> GeometrySnapshot {
        let path = path();
        let space = coordinate_space();
        let fingerprint = path.fingerprint(&space).unwrap();
        GeometrySnapshot::new(target(), GeometryRevision(1), TopologyRevision(1),
            fingerprint, space, PATH2D_PAYLOAD_KIND, PATH2D_SCHEMA_VERSION, path).unwrap()
    }
    fn cage() -> (DeformationTopology, DeformationControlState) {
        let topology_id = DeformationTopologyId::new(1).unwrap();
        let topology_revision = DeformationTopologyRevision::new(1).unwrap();
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: topology_id, revision: topology_revision,
            control_point_ids: vec![ControlPointId::new(1).unwrap(), ControlPointId::new(2).unwrap(),
                ControlPointId::new(3).unwrap(), ControlPointId::new(4).unwrap()],
            faces: vec![
                TopologyFace::Triangle([ControlPointId::new(1).unwrap(), ControlPointId::new(2).unwrap(), ControlPointId::new(3).unwrap()]),
                TopologyFace::Triangle([ControlPointId::new(3).unwrap(), ControlPointId::new(2).unwrap(), ControlPointId::new(4).unwrap()]),
            ],
        });
        let positions = [(0.0,0.0),(2.0,0.0),(0.0,2.0),(2.0,2.0)];
        let state = DeformationControlState {
            topology_id, topology_revision, revision: ControlStateRevision::new(1).unwrap(),
            points: positions.iter().enumerate().map(|(i,(x,y))| ControlPointState {
                id: ControlPointId::new(i as u64+1).unwrap(),
                position: ControlPointPosition::new(*x,*y).unwrap(), pinned:false,
            }).collect(),
        };
        (topology,state)
    }

    #[test]
    fn feature_enumeration_is_command_then_role_order_and_close_is_structural() {
        let features = enumerate_source_features(&path()).unwrap();
        assert_eq!(features.len(), 7);
        assert_eq!(features[0].locator, SourceFeatureLocator { command_index:0, role:PathFeatureRole::MoveToEndpoint });
        assert_eq!(features[2].locator, SourceFeatureLocator { command_index:2, role:PathFeatureRole::QuadraticControl });
        assert_eq!(features[4].locator, SourceFeatureLocator { command_index:3, role:PathFeatureRole::CubicControl1 });
        assert_eq!(features[6].locator, SourceFeatureLocator { command_index:3, role:PathFeatureRole::CubicEndpoint });
    }

    #[test]
    fn complete_unbound_record_validates_against_certified_geometry() {
        let (topology, state) = cage();
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology, &state, &certificate).unwrap();
        let source = snapshot();
        let stamp = BindingStamp::from_snapshot(
            &source,
            &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::UnboundZeroInfluence,
        ).unwrap();
        let entries = enumerate_source_features(source.payload()).unwrap().into_iter()
            .map(|feature| FeatureBindingEntry { locator: feature.locator, status: SourceFeatureBinding::Unbound })
            .collect();
        let record = BindingRecord::new(stamp, entries);
        assert_eq!(record.validate_against(&source, &topology, &geometry), Ok(()));
    }

    #[test]
    fn missing_source_feature_status_is_rejected() {
        let (topology, state) = cage();
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology, &state, &certificate).unwrap();
        let source = snapshot();
        let stamp = BindingStamp::from_snapshot(
            &source, &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::UnboundZeroInfluence,
        ).unwrap();
        let mut entries: Vec<_> = enumerate_source_features(source.payload()).unwrap().into_iter()
            .map(|feature| FeatureBindingEntry { locator: feature.locator, status: SourceFeatureBinding::Unbound })
            .collect();
        entries.pop();
        let record = BindingRecord::new(stamp, entries);
        assert_eq!(record.validate_against(&source, &topology, &geometry),
            Err(BindingError::IncompleteFeatureCoverage));
    }

    #[test]
    fn freshness_rejects_changed_source_revision() {
        let (topology, _) = cage();
        let source = snapshot();
        let stamp = BindingStamp::from_snapshot(
            &source, &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::UnboundZeroInfluence,
        ).unwrap();
        let changed = GeometrySnapshot::new(
            source.target().clone(),
            GeometryRevision(2),
            source.topology_revision(),
            source.fingerprint().clone(),
            source.coordinate_space().clone(),
            source.payload_kind(),
            source.payload_schema_version(),
            source.payload().clone(),
        ).unwrap();
        assert_eq!(stamp.validate_freshness(&changed, &topology),
            Err(BindingError::StaleGeometryRevision));
    }

    #[test]
    fn non_finite_and_out_of_domain_coordinates_are_rejected() {
        let (topology, state) = cage();
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology, &state, &certificate).unwrap();
        let ids = topology.face_at(0).unwrap().control_point_ids().to_vec();
        let locator = SourceFeatureLocator { command_index: 0, role: PathFeatureRole::MoveToEndpoint };
        let non_finite = MappingRecord {
            locator, face_index: 0, subface_index: 0, control_point_ids: ids.clone(),
            coordinates: FaceCoordinates::TriangleBarycentric { weights: [f64::NAN, 0.0, 1.0] },
        };
        assert_eq!(validate_mapping(&non_finite, point(0.0, 0.0), &geometry),
            Err(BindingError::NonFiniteCoordinate));
        let outside = MappingRecord {
            locator, face_index: 0, subface_index: 0, control_point_ids: ids,
            coordinates: FaceCoordinates::TriangleBarycentric { weights: [1.01, 0.0, -0.01] },
        };
        assert_eq!(validate_mapping(&outside, point(0.0, 0.0), &geometry),
            Err(BindingError::CoordinateOutsideDomain));
    }

    #[test]
    fn subpath_ordinals_advance_after_close_and_new_move_to() {
        let multi = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(1.0, 0.0)),
            PathCommand::Close,
            PathCommand::MoveTo(point(2.0, 0.0)),
            PathCommand::LineTo(point(3.0, 0.0)),
        ]).unwrap();
        let features = enumerate_source_features(&multi).unwrap();
        assert_eq!(features[0].subpath_index, 0);
        assert_eq!(features[1].subpath_index, 0);
        assert_eq!(features[2].subpath_index, 1);
        assert_eq!(features[3].subpath_index, 1);
    }

    #[test]
    fn quad_coordinates_reconstruct_using_all_four_authored_vertices() {
        let positions = [
            ControlPointPosition::new(0.0,0.0).unwrap(),
            ControlPointPosition::new(2.0,0.0).unwrap(),
            ControlPointPosition::new(2.0,2.0).unwrap(),
            ControlPointPosition::new(0.0,2.0).unwrap(),
        ];
        let reconstructed = bilinear_point(&positions, 0.25, 0.75).unwrap();
        assert!((reconstructed.x()-0.5).abs()<1.0e-12);
        assert!((reconstructed.y()-1.5).abs()<1.0e-12);
    }

    #[test]
    fn stamp_freshness_does_not_depend_on_control_state_revision() {
        let (topology, state) = cage();
        let stamp = BindingStamp::from_snapshot(&snapshot(), &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::UnboundZeroInfluence).unwrap();
        let mut moved = state.clone();
        moved.revision = ControlStateRevision::new(2).unwrap();
        assert_eq!(stamp.topology_revision, topology.topology_revision());
        assert_eq!(moved.revision.get(), 2);
    }

    #[test]
    fn binding_requires_current_embedding_certificate() {
        let (topology, state) = cage();
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        assert!(ValidatedCageGeometry::new(&topology,&state,&certificate).is_ok());
        let mut moved = state.clone();
        moved.revision = ControlStateRevision::new(2).unwrap();
        assert!(matches!(ValidatedCageGeometry::new(&topology,&moved,&certificate),
            Err(BindingError::UncertifiedCageEmbedding)));
    }

    #[test]
    fn outside_policy_parameters_are_stamped_and_validated() {
        let (topology, _) = cage();
        let source = snapshot();
        let stamp = BindingStamp::from_snapshot(&source, &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::BoundedExtrapolation { max_distance: 12.5 }).unwrap();
        assert_eq!(stamp.outside_policy_version, OUTSIDE_CAGE_POLICY_VERSION);
        assert_eq!(stamp.outside_policy, OutsideCagePolicy::BoundedExtrapolation { max_distance: 12.5 });
        assert_eq!(stamp.validate(), Ok(()));

        assert!(matches!(BindingStamp::from_snapshot(&source, &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::BoundedExtrapolation { max_distance: f64::NAN }),
            Err(BindingError::InvalidOutsidePolicyParameter)));
        assert!(matches!(BindingStamp::from_snapshot(&source, &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::BoundedExtrapolation { max_distance: 0.0 }),
            Err(BindingError::InvalidOutsidePolicyParameter)));
    }

    #[test]
    fn triangle_coordinate_validation_rejects_negative_weights_and_bad_sums() {
        let (topology, state) = cage();
        let certificate = validate_cage_embedding(&topology,&state).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology,&state,&certificate).unwrap();
        let ids = topology.face_at(0).unwrap().control_point_ids().to_vec();
        let mapping = MappingRecord {
            locator: SourceFeatureLocator { command_index:0, role:PathFeatureRole::MoveToEndpoint },
            face_index:0, subface_index:0, control_point_ids:ids,
            coordinates:FaceCoordinates::TriangleBarycentric { weights:[1.1,-0.1,0.0] },
        };
        assert_eq!(validate_mapping(&mapping,point(0.0,0.0),&geometry),
            Err(BindingError::CoordinateOutsideDomain));
    }

    #[test]
    fn mapping_face_order_is_checked_against_stable_ids() {
        let (topology, state) = cage();
        let certificate = validate_cage_embedding(&topology,&state).unwrap();
        let geometry = ValidatedCageGeometry::new(&topology,&state,&certificate).unwrap();
        let mapping = MappingRecord {
            locator: SourceFeatureLocator { command_index:0, role:PathFeatureRole::MoveToEndpoint },
            face_index:0,
            subface_index:0,
            control_point_ids:vec![ControlPointId::new(2).unwrap(),ControlPointId::new(1).unwrap(),ControlPointId::new(3).unwrap()],
            coordinates:FaceCoordinates::TriangleBarycentric { weights:[1.0,0.0,0.0] },
        };
        assert!(matches!(validate_mapping(&mapping,point(0.0,0.0),&geometry),
            Err(BindingError::ControlPointIdOrderMismatch { .. })));
    }
}
