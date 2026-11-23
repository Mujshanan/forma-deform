//! Host-independent single-operator evaluation orchestration.
//!
//! This module validates captured state against a caller-supplied current state,
//! invokes one operator through the shared dispatch contract, and creates an
//! immutable evaluated result. It never reads or mutates host state.

use crate::composition_evaluation::{
    DeformerOperator, EvaluationInterfaceError, GeometryValue, OperatorContext,
    OperatorInvocationError, evaluate_operator,
};
use crate::evaluation_contract::{
    EvaluatedGeometryResult, EvaluationContractError, EvaluationRequest, GeometryOutputOperation,
};
use crate::host_contract::{GeometrySnapshot, HostTargetRef, SnapshotFreshness};
use crate::persistence::DeformationDefinition;

/// Stable diagnostic identities. The enum order is also the primary sort order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvaluationDiagnosticCode {
    InvalidCapturedRequest,
    InvalidCurrentState,
    InvalidOperatorContext,
    ModelRevisionMismatch,
    TimeSampleMismatch,
    DependencyStateMismatch,
    StageMismatch,
    CompositionGraphMismatch,
    SourceSetMismatch,
    MissingCurrentSource,
    UnexpectedCurrentSource,
    TargetGenerationChanged,
    SourceRevisionRegressed,
    SourceFingerprintIntegrityError,
    StaleSourceSnapshot,
    OperatorRejected,
    ResultRejected,
}

/// Structured diagnostic data. No display string participates in identity or
/// ordering; callers can localize messages using the stable diagnostic code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationDiagnostic {
    code: EvaluationDiagnosticCode,
    target: Option<HostTargetRef>,
    contract_error: Option<EvaluationContractError>,
    operator_error: Option<OperatorInvocationError>,
    interface_error: Option<EvaluationInterfaceError>,
}

impl EvaluationDiagnostic {
    fn new(code: EvaluationDiagnosticCode) -> Self {
        Self {
            code,
            target: None,
            contract_error: None,
            operator_error: None,
            interface_error: None,
        }
    }

    fn for_target(code: EvaluationDiagnosticCode, target: &HostTargetRef) -> Self {
        Self {
            code,
            target: Some(target.clone()),
            contract_error: None,
            operator_error: None,
            interface_error: None,
        }
    }

    fn contract(code: EvaluationDiagnosticCode, error: EvaluationContractError) -> Self {
        Self {
            code,
            target: None,
            contract_error: Some(error),
            operator_error: None,
            interface_error: None,
        }
    }

    fn operator(error: OperatorInvocationError) -> Self {
        Self {
            code: EvaluationDiagnosticCode::OperatorRejected,
            target: None,
            contract_error: None,
            operator_error: Some(error),
            interface_error: None,
        }
    }

    fn interface(error: EvaluationInterfaceError) -> Self {
        Self {
            code: EvaluationDiagnosticCode::InvalidOperatorContext,
            target: None,
            contract_error: None,
            operator_error: None,
            interface_error: Some(error),
        }
    }

    pub fn code(&self) -> EvaluationDiagnosticCode {
        self.code
    }

    pub fn target(&self) -> Option<&HostTargetRef> {
        self.target.as_ref()
    }

    pub fn contract_error(&self) -> Option<EvaluationContractError> {
        self.contract_error.clone()
    }

    pub fn operator_error(&self) -> Option<OperatorInvocationError> {
        self.operator_error
    }

    pub fn interface_error(&self) -> Option<EvaluationInterfaceError> {
        self.interface_error
    }
}

/// A failed evaluation with diagnostics in a stable, deterministic order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationFailure {
    diagnostics: Vec<EvaluationDiagnostic>,
}

impl EvaluationFailure {
    fn new(mut diagnostics: Vec<EvaluationDiagnostic>) -> Self {
        diagnostics.sort_by(|left, right| {
            left.code
                .cmp(&right.code)
                .then_with(|| target_sort_key(&left.target).cmp(&target_sort_key(&right.target)))
        });
        Self { diagnostics }
    }

    pub fn diagnostics(&self) -> &[EvaluationDiagnostic] {
        &self.diagnostics
    }
}

impl std::fmt::Display for EvaluationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FDM evaluation failed with {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for EvaluationFailure {}

/// Stateless evaluator for one operator invocation.
#[derive(Clone, Copy, Debug, Default)]
pub struct SingleOperatorEvaluationEngine;

impl SingleOperatorEvaluationEngine {
    /// Evaluate one operator using a captured request and a fresh state snapshot.
    ///
    /// The current request may have a different request ID, but its model,
    /// time sample, dependencies, stage, composition graph and source snapshots
    /// must still agree with the captured request. The host must repeat freshness
    /// validation atomically when it applies the returned result.
    pub fn evaluate(
        &self,
        operator: &dyn DeformerOperator,
        definition: &DeformationDefinition,
        input: &GeometryValue,
        captured: &EvaluationRequest,
        current: &EvaluationRequest,
        result_id: impl Into<String>,
        operation: GeometryOutputOperation,
    ) -> Result<EvaluatedGeometryResult, EvaluationFailure> {
        captured.validate().map_err(|error| {
            EvaluationFailure::new(vec![EvaluationDiagnostic::contract(
                EvaluationDiagnosticCode::InvalidCapturedRequest,
                error,
            )])
        })?;

        current.validate().map_err(|error| {
            EvaluationFailure::new(vec![EvaluationDiagnostic::contract(
                EvaluationDiagnosticCode::InvalidCurrentState,
                error,
            )])
        })?;

        let diagnostics = compare_evaluation_state(captured, current);
        if !diagnostics.is_empty() {
            return Err(EvaluationFailure::new(diagnostics));
        }

        let context = OperatorContext::new(captured).map_err(|error| {
            EvaluationFailure::new(vec![EvaluationDiagnostic::interface(error)])
        })?;
        let output = evaluate_operator(operator, definition, input, &context)
            .map_err(|error| EvaluationFailure::new(vec![EvaluationDiagnostic::operator(error)]))?;

        EvaluatedGeometryResult::new(
            result_id,
            captured,
            definition.target().clone(),
            operation,
            output.into_geometry(),
        )
        .map_err(|error| {
            EvaluationFailure::new(vec![EvaluationDiagnostic::contract(
                EvaluationDiagnosticCode::ResultRejected,
                error,
            )])
        })
    }
}

fn compare_evaluation_state(
    captured: &EvaluationRequest,
    current: &EvaluationRequest,
) -> Vec<EvaluationDiagnostic> {
    let mut diagnostics = Vec::new();

    if captured.model() != current.model() {
        diagnostics.push(EvaluationDiagnostic::new(
            EvaluationDiagnosticCode::ModelRevisionMismatch,
        ));
    }
    if captured.time() != current.time() {
        diagnostics.push(EvaluationDiagnostic::new(
            EvaluationDiagnosticCode::TimeSampleMismatch,
        ));
    }
    if captured.dependencies() != current.dependencies() {
        diagnostics.push(EvaluationDiagnostic::new(
            EvaluationDiagnosticCode::DependencyStateMismatch,
        ));
    }
    if captured.stage_id() != current.stage_id() {
        diagnostics.push(EvaluationDiagnostic::new(
            EvaluationDiagnosticCode::StageMismatch,
        ));
    }
    if captured.composition_graph() != current.composition_graph() {
        diagnostics.push(EvaluationDiagnostic::new(
            EvaluationDiagnosticCode::CompositionGraphMismatch,
        ));
    }

    compare_source_snapshots(captured.sources(), current.sources(), &mut diagnostics);
    diagnostics
}

fn compare_source_snapshots(
    captured: &[GeometrySnapshot],
    current: &[GeometrySnapshot],
    diagnostics: &mut Vec<EvaluationDiagnostic>,
) {
    if captured.len() != current.len() {
        diagnostics.push(EvaluationDiagnostic::new(
            EvaluationDiagnosticCode::SourceSetMismatch,
        ));
    }

    for captured_source in captured {
        let captured_target = captured_source.target();
        let Some(current_source) = current
            .iter()
            .find(|source| same_target_identity(source.target(), captured_target))
        else {
            diagnostics.push(EvaluationDiagnostic::for_target(
                EvaluationDiagnosticCode::MissingCurrentSource,
                captured_target,
            ));
            continue;
        };

        match SnapshotFreshness::compare(&captured_source.stamp(), &current_source.stamp()) {
            Err(_) => diagnostics.push(EvaluationDiagnostic::for_target(
                EvaluationDiagnosticCode::TargetGenerationChanged,
                captured_target,
            )),
            Ok(freshness) if freshness.is_fingerprint_integrity_error() => {
                diagnostics.push(EvaluationDiagnostic::for_target(
                    EvaluationDiagnosticCode::SourceFingerprintIntegrityError,
                    captured_target,
                ));
            }
            Ok(freshness)
                if freshness.geometry_revision_regressed
                    || freshness.topology_revision_regressed =>
            {
                diagnostics.push(EvaluationDiagnostic::for_target(
                    EvaluationDiagnosticCode::SourceRevisionRegressed,
                    captured_target,
                ));
            }
            Ok(freshness) if !freshness.is_current() => {
                diagnostics.push(EvaluationDiagnostic::for_target(
                    EvaluationDiagnosticCode::StaleSourceSnapshot,
                    captured_target,
                ));
            }
            Ok(_) => {}
        }
    }

    for current_source in current {
        if !captured
            .iter()
            .any(|source| same_target_identity(source.target(), current_source.target()))
        {
            diagnostics.push(EvaluationDiagnostic::for_target(
                EvaluationDiagnosticCode::UnexpectedCurrentSource,
                current_source.target(),
            ));
        }
    }
}

fn same_target_identity(left: &HostTargetRef, right: &HostTargetRef) -> bool {
    left.namespace == right.namespace
        && left.document_id == right.document_id
        && left.target_kind == right.target_kind
        && left.target_id == right.target_id
}

fn target_sort_key(target: &Option<HostTargetRef>) -> Option<(&str, &str, &str, &str, u64)> {
    target.as_ref().map(|value| {
        (
            value.namespace.as_str(),
            value.document_id.as_str(),
            value.target_kind.as_str(),
            value.target_id.as_str(),
            value.generation,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition_evaluation::{GeometryValue, OperatorFailure};
    use crate::evaluation_contract::{
        DependencyStamp, EvaluationTimeSample, ModelRevisionStamp, TimeSampleKind,
    };
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceDescriptor, CoordinateSpaceId,
        CoordinateSpaceKind, GeometryRevision, GeometrySnapshot, HostDocumentId, HostNamespace,
        HostTargetId, HostTargetKind, TopologyRevision,
    };
    use crate::path2d::{PATH2D_PAYLOAD_KIND, PATH2D_SCHEMA_VERSION, Path2D, PathCommand, Point2};
    use crate::persistence::{
        DeformationDefinition, DeformationKind, GeometryValueKind, ParameterValue,
    };
    use std::collections::BTreeMap;

    const ALGORITHM_ID: &str = "fdm.single-handle-displacement";
    const ALGORITHM_VERSION: u32 = 1;

    fn target(id: &str, generation: u64) -> HostTargetRef {
        HostTargetRef {
            namespace: HostNamespace::new("anicraft").unwrap(),
            document_id: HostDocumentId::new("document-1").unwrap(),
            target_kind: HostTargetKind::new("shape").unwrap(),
            target_id: HostTargetId::new(id).unwrap(),
            generation,
        }
    }

    fn coordinate_space() -> CoordinateSpaceDescriptor {
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

    fn path(offset: f64) -> Path2D {
        Path2D::new(vec![
            PathCommand::MoveTo(Point2::new(offset, 0.0).unwrap()),
            PathCommand::LineTo(Point2::new(offset + 5.0, 0.0).unwrap()),
        ])
        .unwrap()
    }

    fn snapshot(target: HostTargetRef, revision: u64, payload: Path2D) -> GeometrySnapshot {
        let space = coordinate_space();
        let fingerprint = payload.fingerprint(&space).unwrap();
        GeometrySnapshot::new(
            target,
            GeometryRevision(revision),
            TopologyRevision(1),
            fingerprint,
            space,
            PATH2D_PAYLOAD_KIND,
            PATH2D_SCHEMA_VERSION,
            payload,
        )
        .unwrap()
    }

    fn request(
        request_id: &str,
        sources: Vec<GeometrySnapshot>,
        model_revision: u64,
        frame: u32,
        dependency_revision: u64,
        stage_id: &str,
    ) -> EvaluationRequest {
        let dependencies = vec![DependencyStamp::Hierarchy {
            document_id: HostDocumentId::new("document-1").unwrap(),
            revision: dependency_revision,
        }];
        EvaluationRequest::new(
            request_id,
            sources,
            ModelRevisionStamp::new(
                "model-1",
                model_revision,
                1,
                ALGORITHM_ID,
                ALGORITHM_VERSION,
            )
            .unwrap(),
            EvaluationTimeSample::new(
                "frames",
                format!("frame-{frame}"),
                frame,
                0.0,
                1,
                TimeSampleKind::EvaluatedPose,
            )
            .unwrap(),
            dependencies,
            stage_id,
            None,
        )
        .unwrap()
    }

    fn request_with_graph(
        request_id: &str,
        sources: Vec<GeometrySnapshot>,
        graph_id: &str,
    ) -> EvaluationRequest {
        EvaluationRequest::new(
            request_id,
            sources,
            ModelRevisionStamp::new("model-1", 1, 1, ALGORITHM_ID, ALGORITHM_VERSION).unwrap(),
            EvaluationTimeSample::new(
                "frames",
                "frame-10",
                10,
                0.0,
                1,
                TimeSampleKind::EvaluatedPose,
            )
            .unwrap(),
            vec![DependencyStamp::Hierarchy {
                document_id: HostDocumentId::new("document-1").unwrap(),
                revision: 1,
            }],
            "stage-1",
            Some(CompositionGraphStamp::new(graph_id, 1, 1).unwrap()),
        )
        .unwrap()
    }

    fn definition(source: &GeometrySnapshot) -> DeformationDefinition {
        let parameters = BTreeMap::from([
            ("handle_x".into(), ParameterValue::Number(0.0)),
            ("handle_y".into(), ParameterValue::Number(0.0)),
            ("displacement_x".into(), ParameterValue::Number(2.0)),
            ("displacement_y".into(), ParameterValue::Number(0.0)),
            ("influence_radius".into(), ParameterValue::Number(10.0)),
        ]);
        DeformationDefinition::new(
            "deformer-1",
            1,
            "Single Handle",
            DeformationKind::PinDeformation,
            source.target().clone(),
            source.stamp(),
            source.coordinate_space().clone(),
            GeometryValueKind::AbsoluteGeometry,
            GeometryValueKind::AbsoluteGeometry,
            parameters,
        )
        .unwrap()
    }

    fn fixture() -> (EvaluationRequest, DeformationDefinition, GeometryValue) {
        let source = snapshot(target("shape-1", 1), 1, path(0.0));
        let request = request("captured-1", vec![source.clone()], 1, 10, 1, "stage-1");
        let definition = definition(&source);
        let input = GeometryValue::new(
            GeometryValueKind::AbsoluteGeometry,
            source.payload().clone(),
        );
        (request, definition, input)
    }

    #[test]
    fn successful_evaluation_returns_validated_immutable_result() {
        let (captured, definition, input) = fixture();
        let current = request(
            "current-2",
            captured.sources().to_vec(),
            1,
            10,
            1,
            "stage-1",
        );
        let result = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap();

        assert_eq!(result.result_id(), "result-1");
        assert_eq!(
            result.operation(),
            GeometryOutputOperation::ReplaceSourceGeometry
        );
        result.validate_against(&captured).unwrap();
        assert_ne!(result.payload(), input.geometry());
    }

    #[test]
    fn stale_source_is_rejected_before_operator_execution() {
        let (captured, definition, input) = fixture();
        let stale_source = snapshot(target("shape-1", 1), 2, path(0.0));
        let current = request("current-2", vec![stale_source], 1, 10, 1, "stage-1");

        let failure = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        assert_eq!(
            failure.diagnostics()[0].code(),
            EvaluationDiagnosticCode::StaleSourceSnapshot
        );
    }

    #[test]
    fn same_revision_with_changed_fingerprint_is_an_integrity_error() {
        let (captured, definition, input) = fixture();
        let changed_source = snapshot(target("shape-1", 1), 1, path(1.0));
        let current = request("current-2", vec![changed_source], 1, 10, 1, "stage-1");

        let failure = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        assert_eq!(
            failure.diagnostics()[0].code(),
            EvaluationDiagnosticCode::SourceFingerprintIntegrityError
        );
    }

    #[test]
    fn target_generation_change_is_not_treated_as_same_source() {
        let (captured, definition, input) = fixture();
        let replaced_source = snapshot(target("shape-1", 2), 1, path(0.0));
        let current = request("current-2", vec![replaced_source], 1, 10, 1, "stage-1");

        let failure = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        assert_eq!(
            failure.diagnostics()[0].code(),
            EvaluationDiagnosticCode::TargetGenerationChanged
        );
    }

    #[test]
    fn model_time_dependency_and_stage_changes_are_reported_together_in_stable_order() {
        let (captured, definition, input) = fixture();
        let current = request(
            "current-2",
            captured.sources().to_vec(),
            2,
            11,
            2,
            "stage-2",
        );

        let failure = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        let codes = failure
            .diagnostics()
            .iter()
            .map(EvaluationDiagnostic::code)
            .collect::<Vec<_>>();
        assert_eq!(
            codes,
            vec![
                EvaluationDiagnosticCode::ModelRevisionMismatch,
                EvaluationDiagnosticCode::TimeSampleMismatch,
                EvaluationDiagnosticCode::DependencyStateMismatch,
                EvaluationDiagnosticCode::StageMismatch,
            ]
        );
    }

    #[test]
    fn invalid_operator_parameters_are_reported_as_operator_failure() {
        let (captured, mut definition, input) = fixture();
        let mut parameters = definition.parameters().clone();
        parameters.insert("influence_radius".into(), ParameterValue::Number(-1.0));
        definition = DeformationDefinition::new(
            definition.id(),
            definition.revision(),
            definition.name(),
            definition.kind(),
            definition.target().clone(),
            definition.reference_geometry().clone(),
            definition.coordinate_space().clone(),
            definition.input_kind(),
            definition.output_kind(),
            parameters,
        )
        .unwrap();
        let current = request(
            "current-2",
            captured.sources().to_vec(),
            1,
            10,
            1,
            "stage-1",
        );

        let failure = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        assert_eq!(
            failure.diagnostics()[0].code(),
            EvaluationDiagnosticCode::OperatorRejected
        );
        assert_eq!(
            failure.diagnostics()[0].operator_error(),
            Some(OperatorInvocationError::Operator(
                OperatorFailure::InvalidParameters
            ))
        );
    }

    #[test]
    fn source_revision_regression_is_reported_separately_from_staleness() {
        let captured_source = snapshot(target("shape-1", 1), 3, path(0.0));
        let captured = request("captured-1", vec![captured_source.clone()], 1, 10, 1, "stage-1");
        let definition = definition(&captured_source);
        let input = GeometryValue::new(
            GeometryValueKind::AbsoluteGeometry,
            captured_source.payload().clone(),
        );
        let current_source = snapshot(target("shape-1", 1), 2, path(0.0));
        let current = request("current-2", vec![current_source], 1, 10, 1, "stage-1");

        let failure = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        assert_eq!(
            failure.diagnostics()[0].code(),
            EvaluationDiagnosticCode::SourceRevisionRegressed
        );
    }

    #[test]
    fn missing_and_unexpected_sources_are_both_reported() {
        let (captured, definition, input) = fixture();
        let replacement = snapshot(target("shape-2", 1), 1, path(10.0));
        let current = request("current-2", vec![replacement], 1, 10, 1, "stage-1");

        let failure = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        assert_eq!(
            failure
                .diagnostics()
                .iter()
                .map(EvaluationDiagnostic::code)
                .collect::<Vec<_>>(),
            vec![
                EvaluationDiagnosticCode::MissingCurrentSource,
                EvaluationDiagnosticCode::UnexpectedCurrentSource,
            ]
        );
    }

    #[test]
    fn composition_graph_revision_must_match_captured_state() {
        let source = snapshot(target("shape-1", 1), 1, path(0.0));
        let captured = request_with_graph("captured-1", vec![source.clone()], "graph-a");
        let current = request_with_graph("current-2", vec![source.clone()], "graph-b");
        let definition = definition(&source);
        let input = GeometryValue::new(
            GeometryValueKind::AbsoluteGeometry,
            source.payload().clone(),
        );

        let failure = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        assert_eq!(
            failure.diagnostics()[0].code(),
            EvaluationDiagnosticCode::CompositionGraphMismatch
        );
    }

    #[test]
    fn invalid_result_identity_is_rejected_after_operator_evaluation() {
        let (captured, definition, input) = fixture();
        let current = request(
            "current-2",
            captured.sources().to_vec(),
            1,
            10,
            1,
            "stage-1",
        );

        let failure = SingleOperatorEvaluationEngine
            .evaluate(
                &crate::single_handle_displacement::SingleHandleDisplacementOperator,
                &definition,
                &input,
                &captured,
                &current,
                "  ",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        assert_eq!(failure.diagnostics()[0].code(), EvaluationDiagnosticCode::ResultRejected);
        assert_eq!(
            failure.diagnostics()[0].contract_error(),
            Some(EvaluationContractError::EmptyIdentifier)
        );
    }

    #[test]
    fn diagnostic_order_does_not_depend_on_current_source_order() {
        let first = snapshot(target("shape-a", 1), 2, path(0.0));
        let second = snapshot(target("shape-b", 1), 2, path(10.0));
        let captured = request(
            "captured-1",
            vec![
                snapshot(target("shape-a", 1), 1, path(0.0)),
                snapshot(target("shape-b", 1), 1, path(10.0)),
            ],
            1,
            10,
            1,
            "stage-1",
        );
        let definition = definition(&captured.sources()[0]);
        let input = GeometryValue::new(
            GeometryValueKind::AbsoluteGeometry,
            captured.sources()[0].payload().clone(),
        );
        let current_forward = request(
            "current-forward",
            vec![first.clone(), second.clone()],
            1,
            10,
            1,
            "stage-1",
        );
        let current_reverse = request("current-reverse", vec![second, first], 1, 10, 1, "stage-1");

        let engine = SingleOperatorEvaluationEngine;
        let operator = crate::single_handle_displacement::SingleHandleDisplacementOperator;
        let forward = engine
            .evaluate(
                &operator,
                &definition,
                &input,
                &captured,
                &current_forward,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();
        let reverse = engine
            .evaluate(
                &operator,
                &definition,
                &input,
                &captured,
                &current_reverse,
                "result-1",
                GeometryOutputOperation::ReplaceSourceGeometry,
            )
            .unwrap_err();

        assert_eq!(forward, reverse);
        assert_eq!(
            forward
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.target().unwrap().target_id.as_str())
                .collect::<Vec<_>>(),
            vec!["shape-a", "shape-b"]
        );
    }
}
