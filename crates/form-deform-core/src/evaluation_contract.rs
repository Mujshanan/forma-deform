//! Host-neutral, deterministic evaluation request/result contracts for FDM v2 M1.
//!
//! These values are prepared by Forma Deform from immutable host snapshots. They do
//! not read or mutate AniCraft documents, schedule evaluation, or apply results.

use crate::host_contract::{
    CoordinateSpaceDescriptor, ContractError, GeometryFingerprint, GeometrySnapshot,
    GeometrySnapshotStamp, HostDocumentId, HostTargetRef, SnapshotFreshness,
};
use crate::path2d::{Path2D, PATH2D_FINGERPRINT_ALGORITHM};

pub const EVALUATED_GEOMETRY_SCHEMA: &str = "forma-deform-evaluated-geometry";
pub const EVALUATED_GEOMETRY_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeSampleKind {
    KeyedPoseOnly,
    EvaluatedPose,
}

/// An explicit host-owned time sample. Frame identity is never inferred from
/// the current global playhead.
#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationTimeSample {
    timebase_id: String,
    sample_token: String,
    frame: u32,
    subframe: f64,
    revision: u64,
    kind: TimeSampleKind,
}

impl EvaluationTimeSample {
    pub fn new(
        timebase_id: impl Into<String>,
        sample_token: impl Into<String>,
        frame: u32,
        subframe: f64,
        revision: u64,
        kind: TimeSampleKind,
    ) -> Result<Self, EvaluationContractError> {
        let value = Self {
            timebase_id: timebase_id.into(),
            sample_token: sample_token.into(),
            frame,
            subframe,
            revision,
            kind,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EvaluationContractError> {
        require_id(&self.timebase_id)?;
        require_id(&self.sample_token)?;
        if self.revision == 0 {
            return Err(EvaluationContractError::InvalidRevision);
        }
        if !self.subframe.is_finite() || !(0.0..1.0).contains(&self.subframe) {
            return Err(EvaluationContractError::InvalidSubframe);
        }
        Ok(())
    }

    pub fn timebase_id(&self) -> &str { &self.timebase_id }
    pub fn sample_token(&self) -> &str { &self.sample_token }
    pub fn frame(&self) -> u32 { self.frame }
    pub fn subframe(&self) -> f64 { self.subframe }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn kind(&self) -> TimeSampleKind { self.kind }
}

/// Revision of the authored FDM model and the algorithm that evaluated it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelRevisionStamp {
    model_id: String,
    revision: u64,
    schema_version: u32,
    algorithm_id: String,
    algorithm_version: u32,
}

impl ModelRevisionStamp {
    pub fn new(
        model_id: impl Into<String>,
        revision: u64,
        schema_version: u32,
        algorithm_id: impl Into<String>,
        algorithm_version: u32,
    ) -> Result<Self, EvaluationContractError> {
        let value = Self {
            model_id: model_id.into(),
            revision,
            schema_version,
            algorithm_id: algorithm_id.into(),
            algorithm_version,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EvaluationContractError> {
        require_id(&self.model_id)?;
        require_id(&self.algorithm_id)?;
        if self.revision == 0 || self.schema_version == 0 || self.algorithm_version == 0 {
            return Err(EvaluationContractError::InvalidRevision);
        }
        Ok(())
    }

    pub fn model_id(&self) -> &str { &self.model_id }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn schema_version(&self) -> u32 { self.schema_version }
    pub fn algorithm_id(&self) -> &str { &self.algorithm_id }
    pub fn algorithm_version(&self) -> u32 { self.algorithm_version }
}

/// Identity/revision of an explicitly authored deformation composition graph.
/// This stamp does not create or imply a composition graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionGraphStamp {
    graph_id: String,
    revision: u64,
    schema_version: u32,
}

impl CompositionGraphStamp {
    pub fn new(
        graph_id: impl Into<String>,
        revision: u64,
        schema_version: u32,
    ) -> Result<Self, EvaluationContractError> {
        let value = Self { graph_id: graph_id.into(), revision, schema_version };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EvaluationContractError> {
        require_id(&self.graph_id)?;
        if self.revision == 0 || self.schema_version == 0 {
            return Err(EvaluationContractError::InvalidRevision);
        }
        Ok(())
    }

    pub fn graph_id(&self) -> &str { &self.graph_id }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn schema_version(&self) -> u32 { self.schema_version }
}

/// Host-owned inputs other than source Path2D geometry. Each kind has its
/// own revision domain; unrelated edits must not be folded into geometry revs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DependencyStamp {
    Transform { target: HostTargetRef, revision: u64 },
    Hierarchy { document_id: HostDocumentId, revision: u64 },
    NativeAssetWarp { target: HostTargetRef, revision: u64 },
    UpstreamResult { result_id: String, fingerprint: GeometryFingerprint },
}

impl DependencyStamp {
    pub fn validate(&self) -> Result<(), EvaluationContractError> {
        match self {
            Self::Transform { target, revision }
            | Self::NativeAssetWarp { target, revision } => {
                target.validate().map_err(EvaluationContractError::HostContract)?;
                if *revision == 0 { return Err(EvaluationContractError::InvalidRevision); }
            }
            Self::Hierarchy { revision, .. } => {
                if *revision == 0 { return Err(EvaluationContractError::InvalidRevision); }
            }
            Self::UpstreamResult { result_id, fingerprint } => {
                require_id(result_id)?;
                fingerprint.validate().map_err(EvaluationContractError::HostContract)?;
            }
        }
        Ok(())
    }

    fn key(&self) -> DependencyKey {
        match self {
            Self::Transform { target, .. } => DependencyKey::Transform(target.clone()),
            Self::Hierarchy { document_id, .. } => DependencyKey::Hierarchy(document_id.clone()),
            Self::NativeAssetWarp { target, .. } => DependencyKey::NativeAssetWarp(target.clone()),
            Self::UpstreamResult { result_id, .. } => DependencyKey::UpstreamResult(result_id.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum DependencyKey {
    Transform(HostTargetRef),
    Hierarchy(HostDocumentId),
    NativeAssetWarp(HostTargetRef),
    UpstreamResult(String),
}

/// Immutable request assembled from one consistent set of host snapshots.
#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationRequest {
    request_id: String,
    sources: Vec<GeometrySnapshot>,
    model: ModelRevisionStamp,
    time: EvaluationTimeSample,
    dependencies: Vec<DependencyStamp>,
    stage_id: String,
    composition_graph: Option<CompositionGraphStamp>,
}

impl EvaluationRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: impl Into<String>,
        sources: Vec<GeometrySnapshot>,
        model: ModelRevisionStamp,
        time: EvaluationTimeSample,
        dependencies: Vec<DependencyStamp>,
        stage_id: impl Into<String>,
        composition_graph: Option<CompositionGraphStamp>,
    ) -> Result<Self, EvaluationContractError> {
        let value = Self {
            request_id: request_id.into(),
            sources,
            model,
            time,
            dependencies,
            stage_id: stage_id.into(),
            composition_graph,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EvaluationContractError> {
        require_id(&self.request_id)?;
        require_id(&self.stage_id)?;
        self.model.validate()?;
        self.time.validate()?;
        if self.sources.is_empty() {
            return Err(EvaluationContractError::NoSourceSnapshots);
        }
        let mut targets = std::collections::HashSet::new();
        for source in &self.sources {
            source.validate_envelope().map_err(EvaluationContractError::HostContract)?;
            if !targets.insert(source.target().clone()) {
                return Err(EvaluationContractError::DuplicateSourceTarget);
            }
        }
        validate_dependencies(&self.dependencies)?;
        if let Some(graph) = &self.composition_graph {
            graph.validate()?;
        }
        Ok(())
    }

    pub fn request_id(&self) -> &str { &self.request_id }
    pub fn sources(&self) -> &[GeometrySnapshot] { &self.sources }
    pub fn model(&self) -> &ModelRevisionStamp { &self.model }
    pub fn time(&self) -> &EvaluationTimeSample { &self.time }
    pub fn dependencies(&self) -> &[DependencyStamp] { &self.dependencies }
    pub fn stage_id(&self) -> &str { &self.stage_id }
    pub fn composition_graph(&self) -> Option<&CompositionGraphStamp> { self.composition_graph.as_ref() }

    fn source(&self, target: &HostTargetRef) -> Option<&GeometrySnapshot> {
        self.sources.iter().find(|source| source.target() == target)
    }
}

/// The operation is explicit: a replacement geometry or an independent
/// deformation contribution. Contributions are not automatically combined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryOutputOperation {
    ReplaceSourceGeometry,
    IndependentDeformationContribution,
}

/// A prepared output. It carries every dependency needed for the host to
/// reject stale results before any mutation.
#[derive(Clone, Debug, PartialEq)]
pub struct EvaluatedGeometryResult {
    schema_name: String,
    schema_version: u32,
    result_id: String,
    request_id: String,
    target: HostTargetRef,
    source_stamp: GeometrySnapshotStamp,
    model: ModelRevisionStamp,
    time: EvaluationTimeSample,
    dependencies: Vec<DependencyStamp>,
    stage_id: String,
    composition_graph: Option<CompositionGraphStamp>,
    operation: GeometryOutputOperation,
    coordinate_space: CoordinateSpaceDescriptor,
    payload: Path2D,
    fingerprint: GeometryFingerprint,
}

impl EvaluatedGeometryResult {
    pub fn new(
        result_id: impl Into<String>,
        request: &EvaluationRequest,
        target: HostTargetRef,
        operation: GeometryOutputOperation,
        payload: Path2D,
    ) -> Result<Self, EvaluationContractError> {
        request.validate()?;
        let source = request.source(&target).ok_or(EvaluationContractError::MissingSourceTarget)?;
        let coordinate_space = source.coordinate_space().clone();
        let fingerprint = payload.fingerprint(&coordinate_space)
            .map_err(EvaluationContractError::HostContract)?;
        let value = Self {
            schema_name: EVALUATED_GEOMETRY_SCHEMA.into(),
            schema_version: EVALUATED_GEOMETRY_SCHEMA_VERSION,
            result_id: result_id.into(),
            request_id: request.request_id.clone(),
            target,
            source_stamp: source.stamp(),
            model: request.model.clone(),
            time: request.time.clone(),
            dependencies: request.dependencies.clone(),
            stage_id: request.stage_id.clone(),
            composition_graph: request.composition_graph.clone(),
            operation,
            coordinate_space,
            payload,
            fingerprint,
        };
        value.validate_against(request)?;
        Ok(value)
    }

    pub fn validate_against(
        &self,
        request: &EvaluationRequest,
    ) -> Result<(), EvaluationContractError> {
        request.validate()?;
        require_id(&self.result_id)?;
        if self.schema_name != EVALUATED_GEOMETRY_SCHEMA
            || self.schema_version != EVALUATED_GEOMETRY_SCHEMA_VERSION
        {
            return Err(EvaluationContractError::UnsupportedResultSchema);
        }
        if self.request_id != request.request_id
            || self.model != request.model
            || self.time != request.time
            || self.dependencies != request.dependencies
            || self.stage_id != request.stage_id
            || self.composition_graph != request.composition_graph
        {
            return Err(EvaluationContractError::RequestDependencyMismatch);
        }
        let source = request.source(&self.target).ok_or(EvaluationContractError::MissingSourceTarget)?;
        let current_stamp = source.stamp();
        if self.source_stamp != current_stamp {
            return Err(EvaluationContractError::SourceStampMismatch);
        }
        if self.coordinate_space != *source.coordinate_space() {
            return Err(EvaluationContractError::CoordinateSpaceMismatch);
        }
        self.coordinate_space.validate().map_err(EvaluationContractError::HostContract)?;
        let calculated = self.payload.fingerprint(&self.coordinate_space)
            .map_err(EvaluationContractError::HostContract)?;
        if calculated != self.fingerprint || self.fingerprint.algorithm != PATH2D_FINGERPRINT_ALGORITHM {
            return Err(EvaluationContractError::FingerprintMismatch);
        }
        Ok(())
    }

    /// Compare all host-owned dependencies against their current values before
    /// the host applies this result. This check is not a substitute for atomic
    /// validation inside the host's final transaction.
    pub fn validate_source_freshness(
        &self,
        current_source: &GeometrySnapshot,
    ) -> Result<SnapshotFreshness, EvaluationContractError> {
        current_source.validate_envelope().map_err(EvaluationContractError::HostContract)?;
        SnapshotFreshness::compare(&self.source_stamp, &current_source.stamp())
            .map_err(EvaluationContractError::HostContract)
    }

    pub fn result_id(&self) -> &str { &self.result_id }
    pub fn request_id(&self) -> &str { &self.request_id }
    pub fn target(&self) -> &HostTargetRef { &self.target }
    pub fn source_stamp(&self) -> &GeometrySnapshotStamp { &self.source_stamp }
    pub fn model(&self) -> &ModelRevisionStamp { &self.model }
    pub fn time(&self) -> &EvaluationTimeSample { &self.time }
    pub fn dependencies(&self) -> &[DependencyStamp] { &self.dependencies }
    pub fn stage_id(&self) -> &str { &self.stage_id }
    pub fn composition_graph(&self) -> Option<&CompositionGraphStamp> { self.composition_graph.as_ref() }
    pub fn operation(&self) -> GeometryOutputOperation { self.operation }
    pub fn coordinate_space(&self) -> &CoordinateSpaceDescriptor { &self.coordinate_space }
    pub fn payload(&self) -> &Path2D { &self.payload }
    pub fn fingerprint(&self) -> &GeometryFingerprint { &self.fingerprint }
}

fn validate_dependencies(dependencies: &[DependencyStamp]) -> Result<(), EvaluationContractError> {
    let mut keys = std::collections::HashSet::new();
    for dependency in dependencies {
        dependency.validate()?;
        if !keys.insert(dependency.key()) {
            return Err(EvaluationContractError::DuplicateDependency);
        }
    }
    Ok(())
}

fn require_id(value: &str) -> Result<(), EvaluationContractError> {
    if value.trim().is_empty() {
        Err(EvaluationContractError::EmptyIdentifier)
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvaluationContractError {
    EmptyIdentifier,
    InvalidRevision,
    InvalidSubframe,
    NoSourceSnapshots,
    DuplicateSourceTarget,
    DuplicateDependency,
    MissingSourceTarget,
    RequestDependencyMismatch,
    SourceStampMismatch,
    CoordinateSpaceMismatch,
    FingerprintMismatch,
    UnsupportedResultSchema,
    HostContract(ContractError),
}

impl std::fmt::Display for EvaluationContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIdentifier => write!(f, "evaluation identifiers must not be empty"),
            Self::InvalidRevision => write!(f, "evaluation revisions and schema versions must be non-zero"),
            Self::InvalidSubframe => write!(f, "subframe must be finite and in the interval [0, 1)"),
            Self::NoSourceSnapshots => write!(f, "an evaluation request requires at least one source snapshot"),
            Self::DuplicateSourceTarget => write!(f, "an evaluation request contains a duplicate source target"),
            Self::DuplicateDependency => write!(f, "an evaluation request contains a duplicate dependency stamp"),
            Self::MissingSourceTarget => write!(f, "the requested result target has no matching source snapshot"),
            Self::RequestDependencyMismatch => write!(f, "the result does not match the request's model, time or dependency stamps"),
            Self::SourceStampMismatch => write!(f, "the result source stamp does not match the request snapshot"),
            Self::CoordinateSpaceMismatch => write!(f, "the result coordinate space differs from the source coordinate space"),
            Self::FingerprintMismatch => write!(f, "the evaluated geometry fingerprint does not match its payload"),
            Self::UnsupportedResultSchema => write!(f, "unsupported evaluated geometry result schema"),
            Self::HostContract(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for EvaluationContractError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceId, CoordinateSpaceKind,
        GeometryRevision, HostNamespace, HostTargetId, HostTargetKind, TopologyRevision,
    };
    use crate::path2d::{PathCommand, Point2};

    fn target(generation: u64) -> HostTargetRef {
        HostTargetRef {
            namespace: HostNamespace::new("anicraft").unwrap(),
            document_id: HostDocumentId::new("doc-1").unwrap(),
            target_kind: HostTargetKind::new("shape").unwrap(),
            target_id: HostTargetId::new("element-17").unwrap(),
            generation,
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

    fn path(x: f64) -> Path2D {
        Path2D::new(vec![
            PathCommand::MoveTo(Point2::new(x, 0.0).unwrap()),
            PathCommand::LineTo(Point2::new(x + 10.0, 0.0).unwrap()),
        ]).unwrap()
    }

    fn snapshot(generation: u64, geometry: u64, topology: u64) -> GeometrySnapshot {
        let space = coordinate_space();
        let payload = path(0.0);
        let fingerprint = payload.fingerprint(&space).unwrap();
        GeometrySnapshot::new(
            target(generation),
            GeometryRevision(geometry),
            TopologyRevision(topology),
            fingerprint,
            space,
            "Path2D",
            1,
            payload,
        ).unwrap()
    }

    fn model() -> ModelRevisionStamp {
        ModelRevisionStamp::new("fdm-model-1", 2, 1, "fdm-path-deform", 1).unwrap()
    }

    fn time() -> EvaluationTimeSample {
        EvaluationTimeSample::new("anicraft-24fps", "sample-12", 12, 0.0, 4, TimeSampleKind::KeyedPoseOnly).unwrap()
    }

    fn request() -> EvaluationRequest {
        EvaluationRequest::new(
            "request-1",
            vec![snapshot(1, 5, 2)],
            model(),
            time(),
            vec![
                DependencyStamp::Transform { target: target(1), revision: 8 },
                DependencyStamp::Hierarchy {
                    document_id: HostDocumentId::new("doc-1").unwrap(),
                    revision: 3,
                },
            ],
            "source-rest",
            None,
        ).unwrap()
    }

    #[test]
    fn time_sample_rejects_invalid_revision_and_subframe() {
        assert_eq!(
            EvaluationTimeSample::new("tb", "sample", 1, 1.0, 1, TimeSampleKind::KeyedPoseOnly),
            Err(EvaluationContractError::InvalidSubframe)
        );
        assert_eq!(
            EvaluationTimeSample::new("tb", "sample", 1, 0.0, 0, TimeSampleKind::KeyedPoseOnly),
            Err(EvaluationContractError::InvalidRevision)
        );
    }

    #[test]
    fn request_requires_sources_and_unique_target_identities() {
        assert_eq!(
            EvaluationRequest::new("r", vec![], model(), time(), vec![], "stage", None),
            Err(EvaluationContractError::NoSourceSnapshots)
        );
        assert_eq!(
            EvaluationRequest::new("r", vec![snapshot(1, 1, 1), snapshot(1, 2, 1)], model(), time(), vec![], "stage", None),
            Err(EvaluationContractError::DuplicateSourceTarget)
        );
    }

    #[test]
    fn request_rejects_duplicate_dependency_domains() {
        let dependency = DependencyStamp::Transform { target: target(1), revision: 1 };
        assert_eq!(
            EvaluationRequest::new("r", vec![snapshot(1, 1, 1)], model(), time(), vec![dependency.clone(), dependency], "stage", None),
            Err(EvaluationContractError::DuplicateDependency)
        );
    }

    #[test]
    fn result_is_bound_to_exact_source_model_time_and_dependencies() {
        let request = request();
        let result = EvaluatedGeometryResult::new(
            "result-1",
            &request,
            target(1),
            GeometryOutputOperation::ReplaceSourceGeometry,
            path(2.0),
        ).unwrap();
        assert!(result.validate_against(&request).is_ok());
        assert_eq!(result.operation(), GeometryOutputOperation::ReplaceSourceGeometry);
        assert_eq!(result.coordinate_space(), &coordinate_space());
    }

    #[test]
    fn result_rejects_unknown_target_and_stale_source_stamp() {
        let request = request();
        assert_eq!(
            EvaluatedGeometryResult::new("result", &request, target(2), GeometryOutputOperation::ReplaceSourceGeometry, path(0.0)),
            Err(EvaluationContractError::MissingSourceTarget)
        );
        let result = EvaluatedGeometryResult::new(
            "result",
            &request,
            target(1),
            GeometryOutputOperation::IndependentDeformationContribution,
            path(1.0),
        ).unwrap();
        let changed = EvaluationRequest::new(
            "request-1",
            vec![snapshot(1, 6, 2)],
            model(),
            time(),
            request.dependencies().to_vec(),
            "source-rest",
            None,
        ).unwrap();
        assert_eq!(result.validate_against(&changed), Err(EvaluationContractError::SourceStampMismatch));
    }

    #[test]
    fn result_rejects_changed_time_model_and_external_dependencies() {
        let request = request();
        let result = EvaluatedGeometryResult::new(
            "result",
            &request,
            target(1),
            GeometryOutputOperation::ReplaceSourceGeometry,
            path(2.0),
        ).unwrap();
        let changed_time = EvaluationRequest::new(
            "request-1", request.sources().to_vec(), model(),
            EvaluationTimeSample::new("anicraft-24fps", "sample-13", 13, 0.0, 5, TimeSampleKind::KeyedPoseOnly).unwrap(),
            request.dependencies().to_vec(), "source-rest", None,
        ).unwrap();
        assert_eq!(result.validate_against(&changed_time), Err(EvaluationContractError::RequestDependencyMismatch));

        let changed_deps = EvaluationRequest::new(
            "request-1", request.sources().to_vec(), model(), time(),
            vec![DependencyStamp::Transform { target: target(1), revision: 9 }],
            "source-rest", None,
        ).unwrap();
        assert_eq!(result.validate_against(&changed_deps), Err(EvaluationContractError::RequestDependencyMismatch));
    }

    #[test]
    fn result_fingerprint_is_deterministic_and_source_freshness_is_explicit() {
        let request = request();
        let first = EvaluatedGeometryResult::new("result-a", &request, target(1), GeometryOutputOperation::ReplaceSourceGeometry, path(2.0)).unwrap();
        let second = EvaluatedGeometryResult::new("result-b", &request, target(1), GeometryOutputOperation::ReplaceSourceGeometry, path(2.0)).unwrap();
        assert_eq!(first.fingerprint(), second.fingerprint());
        let changed = snapshot(1, 6, 2);
        assert!(!first.validate_source_freshness(&changed).unwrap().is_current());
    }

    #[test]
    fn composition_graph_stamp_is_explicit_and_part_of_result_identity() {
        let graph = CompositionGraphStamp::new("composition-1", 3, 1).unwrap();
        let request = EvaluationRequest::new(
            "request-2", vec![snapshot(1, 5, 2)], model(), time(), vec![],
            "source-rest", Some(graph.clone()),
        ).unwrap();
        let result = EvaluatedGeometryResult::new(
            "result-2", &request, target(1),
            GeometryOutputOperation::IndependentDeformationContribution, path(2.0),
        ).unwrap();
        assert_eq!(result.composition_graph(), Some(&graph));
    }
}
