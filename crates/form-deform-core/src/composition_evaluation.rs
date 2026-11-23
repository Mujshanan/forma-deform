//! Host-neutral interfaces for deterministic FDM composition evaluation.
//!
//! The operator trait is the single extension point for deformation algorithms.
//! Dispatch through `evaluate_operator` to enforce definition, source, coordinate,
//! input-kind and output-kind contracts around a pure operator invocation. This
//! module does not schedule host work or mutate AniCraft geometry.

use crate::evaluation_contract::{DependencyStamp, EvaluationRequest};
use crate::geometry::{require_compatible_coordinate_spaces, GeometryInput};
use crate::host_contract::HostTargetRef;
use crate::path2d::Path2D;
use crate::persistence::{DeformationDefinition, DeformationKind, GeometryValueKind};

/// A single typed geometry value carried between graph nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct GeometryValue {
    kind: GeometryValueKind,
    geometry: Path2D,
}

impl GeometryValue {
    pub fn new(kind: GeometryValueKind, geometry: Path2D) -> Self {
        Self { kind, geometry }
    }

    pub fn kind(&self) -> GeometryValueKind { self.kind }
    pub fn geometry(&self) -> &Path2D { &self.geometry }
    pub fn into_geometry(self) -> Path2D { self.geometry }
}

/// One explicitly identified result from a parallel branch.
#[derive(Clone, Debug, PartialEq)]
pub struct BranchValue {
    branch_node_id: String,
    value: GeometryValue,
}

impl BranchValue {
    pub fn new(branch_node_id: impl Into<String>, value: GeometryValue) -> Result<Self, EvaluationInterfaceError> {
        let branch_node_id = branch_node_id.into();
        require_id(&branch_node_id)?;
        Ok(Self { branch_node_id, value })
    }

    pub fn branch_node_id(&self) -> &str { &self.branch_node_id }
    pub fn value(&self) -> &GeometryValue { &self.value }
}

/// Runtime cardinality is explicit: a normal node yields one geometry, while a
/// Parallel node yields an ordered set of independently evaluated branches.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeValue {
    Geometry(GeometryValue),
    Branches(Vec<BranchValue>),
}

/// Read-only context supplied to every operator invocation. The request carries
/// immutable source snapshots, time sample, model revision and dependencies.
pub struct OperatorContext<'a> {
    request: &'a EvaluationRequest,
}

impl<'a> OperatorContext<'a> {
    pub fn new(request: &'a EvaluationRequest) -> Result<Self, EvaluationInterfaceError> {
        request.validate().map_err(|_| EvaluationInterfaceError::InvalidEvaluationRequest)?;
        Ok(Self { request })
    }

    pub fn request(&self) -> &'a EvaluationRequest { self.request }
    pub fn dependencies(&self) -> &'a [DependencyStamp] { self.request.dependencies() }

    /// Resolve the immutable root snapshot for an operator's authored target.
    /// Intermediate composition values remain explicit inputs and are not
    /// silently substituted for the host-owned source snapshot.
    pub fn source_geometry(
        &self,
        target: &HostTargetRef,
    ) -> Result<GeometryInput<'a>, EvaluationInterfaceError> {
        let snapshot = self.request.sources().iter()
            .find(|source| source.target() == target)
            .ok_or(EvaluationInterfaceError::MissingSourceGeometry)?;
        GeometryInput::new(snapshot).map_err(|_| EvaluationInterfaceError::InvalidSourceGeometry)
    }
}

/// Plug-in implementation for one authored deformation kind.
///
/// Implementations must be deterministic for identical definition, input,
/// request and algorithm version. They must be pure with respect to host state:
/// no document reads, writes, global playhead access or hidden mutable cache.
/// Call through `evaluate_operator` so the surrounding contracts are checked.
pub trait DeformerOperator: Send + Sync {
    fn kind(&self) -> DeformationKind;
    fn algorithm_id(&self) -> &'static str;
    fn algorithm_version(&self) -> u32;

    fn evaluate(
        &self,
        definition: &DeformationDefinition,
        input: &GeometryValue,
        context: &OperatorContext<'_>,
    ) -> Result<GeometryValue, OperatorFailure>;
}

/// Host-neutral lookup boundary. The registry owns operator instances; the
/// composition engine only requests an implementation by its authored kind.
pub trait OperatorRegistry {
    fn resolve(&self, kind: DeformationKind) -> Option<&dyn DeformerOperator>;
}

/// Stable failure categories suitable for diagnostics and tests. Detailed
/// human-readable context may be attached by the caller, not used as identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorFailure {
    UnsupportedDefinition,
    InputKindMismatch,
    CoordinateSpaceMismatch,
    InvalidParameters,
    TopologyMismatch,
    NonFiniteResult,
    ResourceLimitExceeded,
    ArithmeticFailure,
}

/// Contract-level failures before or around operator execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationInterfaceError {
    EmptyIdentifier,
    InvalidEvaluationRequest,
    MissingOperator,
    InvalidOperatorVersion,
    InvalidNodeOutputCardinality,
    IncompatibleValueKinds,
    InvalidDefinition,
    DefinitionKindMismatch,
    MissingSourceGeometry,
    InvalidSourceGeometry,
    CoordinateSpaceMismatch,
    SourceSnapshotMismatch,
    InputKindMismatch,
    OutputKindMismatch,
}

impl std::fmt::Display for EvaluationInterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FDM evaluation interface error: {self:?}")
    }
}
impl std::error::Error for EvaluationInterfaceError {}

/// Distinguishes a rejected invocation contract from an algorithm's own failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorInvocationError {
    Contract(EvaluationInterfaceError),
    Operator(OperatorFailure),
}

impl std::fmt::Display for OperatorInvocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FDM operator invocation failed: {self:?}")
    }
}
impl std::error::Error for OperatorInvocationError {}

fn require_id(value: &str) -> Result<(), EvaluationInterfaceError> {
    if value.trim().is_empty() {
        Err(EvaluationInterfaceError::EmptyIdentifier)
    } else {
        Ok(())
    }
}

/// Validate stable operator metadata before it is admitted to a run.
pub fn validate_operator(operator: &dyn DeformerOperator) -> Result<(), EvaluationInterfaceError> {
    if operator.algorithm_id().trim().is_empty() || operator.algorithm_version() == 0 {
        return Err(EvaluationInterfaceError::InvalidOperatorVersion);
    }
    Ok(())
}

/// Execute one operator with mandatory preflight and postflight validation.
///
/// This is deliberately a single-node boundary, not a graph scheduler. It
/// validates that the authored definition, selected operator, immutable source
/// snapshot and typed intermediate value agree. The operator remains responsible
/// for algorithm-specific parameter and numerical validation.
pub fn evaluate_operator(
    operator: &dyn DeformerOperator,
    definition: &DeformationDefinition,
    input: &GeometryValue,
    context: &OperatorContext<'_>,
) -> Result<GeometryValue, OperatorInvocationError> {
    definition.validate().map_err(|_| OperatorInvocationError::Contract(
        EvaluationInterfaceError::InvalidDefinition
    ))?;
    validate_operator(operator).map_err(OperatorInvocationError::Contract)?;

    if operator.kind() != definition.kind() {
        return Err(OperatorInvocationError::Contract(
            EvaluationInterfaceError::DefinitionKindMismatch
        ));
    }
    if input.kind() != definition.input_kind() {
        return Err(OperatorInvocationError::Contract(
            EvaluationInterfaceError::InputKindMismatch
        ));
    }

    let source = context.source_geometry(definition.target())
        .map_err(OperatorInvocationError::Contract)?;
    if source.stamp() != *definition.reference_geometry() {
        return Err(OperatorInvocationError::Contract(
            EvaluationInterfaceError::SourceSnapshotMismatch
        ));
    }
    require_compatible_coordinate_spaces(definition.coordinate_space(), source.coordinate_space())
        .map_err(|_| OperatorInvocationError::Contract(
            EvaluationInterfaceError::CoordinateSpaceMismatch
        ))?;

    let output = operator.evaluate(definition, input, context)
        .map_err(OperatorInvocationError::Operator)?;
    if output.kind() != definition.output_kind() {
        return Err(OperatorInvocationError::Contract(
            EvaluationInterfaceError::OutputKindMismatch
        ));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path2d::{PathCommand, Point2};

    fn geometry() -> Path2D {
        Path2D::new(vec![PathCommand::MoveTo(Point2::new(0.0, 0.0).unwrap())]).unwrap()
    }

    #[test]
    fn geometry_values_retain_their_explicit_semantic_kind() {
        let absolute = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());
        let displacement = GeometryValue::new(GeometryValueKind::Displacement, geometry());
        assert_eq!(absolute.kind(), GeometryValueKind::AbsoluteGeometry);
        assert_eq!(displacement.kind(), GeometryValueKind::Displacement);
    }

    #[test]
    fn branch_values_require_stable_node_identity() {
        let value = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());
        assert_eq!(
            BranchValue::new("  ", value.clone()),
            Err(EvaluationInterfaceError::EmptyIdentifier)
        );
        assert_eq!(BranchValue::new("branch-a", value).unwrap().branch_node_id(), "branch-a");
    }

    #[test]
    fn node_value_distinguishes_single_geometry_from_parallel_branches() {
        let value = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());
        let single = NodeValue::Geometry(value.clone());
        let branches = NodeValue::Branches(vec![
            BranchValue::new("branch-a", value.clone()).unwrap(),
            BranchValue::new("branch-b", value).unwrap(),
        ]);
        assert!(matches!(single, NodeValue::Geometry(_)));
        assert!(matches!(branches, NodeValue::Branches(values) if values.len() == 2));
    }

    struct InvalidOperator;
    impl DeformerOperator for InvalidOperator {
        fn kind(&self) -> DeformationKind { DeformationKind::PinDeformation }
        fn algorithm_id(&self) -> &'static str { " " }
        fn algorithm_version(&self) -> u32 { 0 }
        fn evaluate(
            &self,
            _definition: &DeformationDefinition,
            input: &GeometryValue,
            _context: &OperatorContext<'_>,
        ) -> Result<GeometryValue, OperatorFailure> {
            Ok(input.clone())
        }
    }

    #[test]
    fn operator_metadata_requires_named_nonzero_algorithm_version() {
        assert_eq!(
            validate_operator(&InvalidOperator),
            Err(EvaluationInterfaceError::InvalidOperatorVersion)
        );
    }

    use crate::evaluation_contract::{
        EvaluationRequest, EvaluationTimeSample, ModelRevisionStamp, TimeSampleKind,
    };
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceDescriptor, CoordinateSpaceId,
        CoordinateSpaceKind, GeometryRevision, HostDocumentId, HostNamespace, HostTargetId,
        HostTargetKind, HostTargetRef, TopologyRevision, GeometrySnapshot,
    };
    use crate::persistence::{DeformationDefinition, ParameterValue};
    use std::collections::BTreeMap;

    fn target() -> HostTargetRef {
        HostTargetRef {
            namespace: HostNamespace::new("anicraft").unwrap(),
            document_id: HostDocumentId::new("doc-1").unwrap(),
            target_kind: HostTargetKind::new("shape").unwrap(),
            target_id: HostTargetId::new("shape-1").unwrap(),
            generation: 1,
        }
    }

    fn space() -> crate::host_contract::CoordinateSpaceDescriptor {
        CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("shape-local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        }
    }

    fn fixture(
        coordinate_space: CoordinateSpaceDescriptor,
        input_kind: GeometryValueKind,
        output_kind: GeometryValueKind,
    ) -> (EvaluationRequest, DeformationDefinition) {
        let path = geometry();
        let source_coordinate_space = space();
        let fingerprint = path.fingerprint(&source_coordinate_space).unwrap();
        let target = target();
        let snapshot = GeometrySnapshot::new(
            target.clone(), GeometryRevision(1), TopologyRevision(1), fingerprint,
            source_coordinate_space, crate::path2d::PATH2D_PAYLOAD_KIND,
            crate::path2d::PATH2D_SCHEMA_VERSION, path,
        ).unwrap();
        let reference = snapshot.stamp();
        let request = EvaluationRequest::new(
            "request-1",
            vec![snapshot],
            ModelRevisionStamp::new("model-1", 1, 1, "test-model", 1).unwrap(),
            EvaluationTimeSample::new(
                "frames", "sample-1", 1, 0.0, 1, TimeSampleKind::EvaluatedPose,
            ).unwrap(),
            vec![],
            "stage-1",
            None,
        ).unwrap();
        let definition = DeformationDefinition::new(
            "deformer-1", 1, "Test Deformer", DeformationKind::PinDeformation,
            target, reference, coordinate_space, input_kind, output_kind, BTreeMap::<String, ParameterValue>::new(),
        ).unwrap();
        (request, definition)
    }

    struct TestOperator {
        kind: DeformationKind,
        output_kind: GeometryValueKind,
    }

    impl DeformerOperator for TestOperator {
        fn kind(&self) -> DeformationKind { self.kind }
        fn algorithm_id(&self) -> &'static str { "test-operator" }
        fn algorithm_version(&self) -> u32 { 1 }
        fn evaluate(
            &self,
            _definition: &DeformationDefinition,
            input: &GeometryValue,
            _context: &OperatorContext<'_>,
        ) -> Result<GeometryValue, OperatorFailure> {
            Ok(GeometryValue::new(self.output_kind, input.geometry().clone()))
        }
    }

    #[test]
    fn operator_dispatch_accepts_matching_definition_snapshot_and_value_contracts() {
        let (request, definition) = fixture(
            space(), GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement,
        );
        let context = OperatorContext::new(&request).unwrap();
        let input = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());
        let operator = TestOperator {
            kind: DeformationKind::PinDeformation,
            output_kind: GeometryValueKind::Displacement,
        };

        let output = evaluate_operator(&operator, &definition, &input, &context).unwrap();
        assert_eq!(output.kind(), GeometryValueKind::Displacement);
    }

    #[test]
    fn operator_dispatch_rejects_input_kind_before_running_operator() {
        let (request, definition) = fixture(
            space(), GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement,
        );
        let context = OperatorContext::new(&request).unwrap();
        let input = GeometryValue::new(GeometryValueKind::Displacement, geometry());
        let operator = TestOperator {
            kind: DeformationKind::PinDeformation,
            output_kind: GeometryValueKind::Displacement,
        };

        assert_eq!(
            evaluate_operator(&operator, &definition, &input, &context),
            Err(OperatorInvocationError::Contract(EvaluationInterfaceError::InputKindMismatch))
        );
    }

    #[test]
    fn operator_dispatch_rejects_authored_source_stamp_mismatch() {
        let (request, definition) = fixture(
            space(), GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement,
        );
        let context = OperatorContext::new(&request).unwrap();
        let input = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());
        let captured = definition.reference_geometry();
        let stale_reference = crate::host_contract::GeometrySnapshotStamp::new(
            captured.target().clone(),
            GeometryRevision(captured.geometry_revision().0 + 1),
            captured.topology_revision(),
            captured.fingerprint().clone(),
        ).unwrap();
        let stale_definition = DeformationDefinition::new(
            definition.id(), definition.revision(), definition.name(), definition.kind(),
            definition.target().clone(), stale_reference, definition.coordinate_space().clone(),
            definition.input_kind(), definition.output_kind(), definition.parameters().clone(),
        ).unwrap();
        let operator = TestOperator {
            kind: DeformationKind::PinDeformation,
            output_kind: GeometryValueKind::Displacement,
        };

        assert_eq!(
            evaluate_operator(&operator, &stale_definition, &input, &context),
            Err(OperatorInvocationError::Contract(EvaluationInterfaceError::SourceSnapshotMismatch))
        );
    }

    #[test]
    fn operator_dispatch_rejects_coordinate_space_mismatch() {
        let mut wrong_space = space();
        wrong_space.linear_unit = "mm".into();
        let (request, definition) = fixture(
            wrong_space, GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement,
        );
        let context = OperatorContext::new(&request).unwrap();
        let input = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());
        let operator = TestOperator {
            kind: DeformationKind::PinDeformation,
            output_kind: GeometryValueKind::Displacement,
        };

        // The authored definition is internally valid, but its coordinate
        // interpretation is incompatible with the immutable host snapshot.
        assert_eq!(
            evaluate_operator(&operator, &definition, &input, &context),
            Err(OperatorInvocationError::Contract(EvaluationInterfaceError::CoordinateSpaceMismatch))
        );
    }

    #[test]
    fn operator_dispatch_rejects_wrong_kind_and_wrong_output_kind() {
        let (request, definition) = fixture(
            space(), GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement,
        );
        let context = OperatorContext::new(&request).unwrap();
        let input = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());

        let wrong_operator = TestOperator {
            kind: DeformationKind::Lattice,
            output_kind: GeometryValueKind::Displacement,
        };
        assert_eq!(
            evaluate_operator(&wrong_operator, &definition, &input, &context),
            Err(OperatorInvocationError::Contract(EvaluationInterfaceError::DefinitionKindMismatch))
        );

        let wrong_output = TestOperator {
            kind: DeformationKind::PinDeformation,
            output_kind: GeometryValueKind::AbsoluteGeometry,
        };
        assert_eq!(
            evaluate_operator(&wrong_output, &definition, &input, &context),
            Err(OperatorInvocationError::Contract(EvaluationInterfaceError::OutputKindMismatch))
        );
    }
}
