//! Cross-module regression tests for the FDM-01 geometry and evaluation contracts.
//!
//! These tests deliberately exercise the public contracts together rather than
//! retesting individual helper functions in isolation.

use std::collections::BTreeMap;

use form_deform_core::composition_evaluation::GeometryValue;
use form_deform_core::evaluation_contract::{
    EvaluationRequest, EvaluationTimeSample, GeometryOutputOperation, ModelRevisionStamp,
    TimeSampleKind,
};
use form_deform_core::evaluation_engine::{
    EvaluationDiagnosticCode, SingleOperatorEvaluationEngine,
};
use form_deform_core::host_contract::{
    AxisOrientation, CoordinateOrigin, CoordinateSpaceDescriptor, CoordinateSpaceId,
    CoordinateSpaceKind, GeometryRevision, GeometrySnapshot, HostDocumentId, HostNamespace,
    HostTargetId, HostTargetKind, HostTargetRef, TopologyRevision,
};
use form_deform_core::path2d::{
    PATH2D_PAYLOAD_KIND, PATH2D_SCHEMA_VERSION, Path2D, PathCommand, Point2,
};
use form_deform_core::persistence::{
    DeformationDefinition, DeformationKind, GeometryValueKind, ParameterValue,
};
use form_deform_core::single_handle_displacement::{
    SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_ID, SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_VERSION,
    SingleHandleDisplacementOperator,
};

fn point(x: f64, y: f64) -> Point2 {
    Point2::new(x, y).expect("test coordinates are finite")
}

fn target() -> HostTargetRef {
    HostTargetRef {
        namespace: HostNamespace::new("anicraft").unwrap(),
        document_id: HostDocumentId::new("regression-document").unwrap(),
        target_kind: HostTargetKind::new("shape").unwrap(),
        target_id: HostTargetId::new("curved-shape").unwrap(),
        generation: 1,
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

fn curved_path() -> Path2D {
    Path2D::new(vec![
        PathCommand::MoveTo(point(-8.0, 0.0)),
        PathCommand::LineTo(point(-4.0, 0.0)),
        PathCommand::QuadraticTo {
            control: point(0.0, 8.0),
            to: point(4.0, 0.0),
        },
        PathCommand::CubicTo {
            control1: point(6.0, -8.0),
            control2: point(10.0, -8.0),
            to: point(12.0, 0.0),
        },
        PathCommand::Close,
    ])
    .unwrap()
}

fn snapshot(path: Path2D, geometry_revision: u64) -> GeometrySnapshot {
    let space = coordinate_space();
    let fingerprint = path.fingerprint(&space).unwrap();
    GeometrySnapshot::new(
        target(),
        GeometryRevision(geometry_revision),
        TopologyRevision(1),
        fingerprint,
        space,
        PATH2D_PAYLOAD_KIND,
        PATH2D_SCHEMA_VERSION,
        path,
    )
    .unwrap()
}

fn request(source: GeometrySnapshot, request_id: &str) -> EvaluationRequest {
    EvaluationRequest::new(
        request_id,
        vec![source],
        ModelRevisionStamp::new(
            "regression-model",
            1,
            1,
            SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_ID,
            SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_VERSION,
        )
        .unwrap(),
        EvaluationTimeSample::new(
            "frames",
            "frame-12",
            12,
            0.0,
            1,
            TimeSampleKind::EvaluatedPose,
        )
        .unwrap(),
        vec![],
        "stage-regression",
        None,
    )
    .unwrap()
}

fn definition(source: &GeometrySnapshot) -> DeformationDefinition {
    let parameters = BTreeMap::from([
        ("handle_x".into(), ParameterValue::Number(0.0)),
        ("handle_y".into(), ParameterValue::Number(0.0)),
        ("displacement_x".into(), ParameterValue::Number(3.0)),
        ("displacement_y".into(), ParameterValue::Number(-2.0)),
        ("influence_radius".into(), ParameterValue::Number(32.0)),
    ]);

    DeformationDefinition::new(
        "regression-deformer",
        1,
        "Regression Deformer",
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

#[test]
fn engine_preserves_path2d_topology_and_returns_validated_independent_outputs() {
    let original_path = curved_path();
    let source = snapshot(original_path.clone(), 1);
    let captured = request(source.clone(), "captured");
    let current = request(source.clone(), "current");
    let authored_definition = definition(&source);
    let input = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, original_path.clone());
    let operator = SingleHandleDisplacementOperator;
    let engine = SingleOperatorEvaluationEngine;

    let replacement = engine
        .evaluate(
            &operator,
            &authored_definition,
            &input,
            &captured,
            &current,
            "replacement-result",
            GeometryOutputOperation::ReplaceSourceGeometry,
        )
        .unwrap();
    let contribution = engine
        .evaluate(
            &operator,
            &authored_definition,
            &input,
            &captured,
            &current,
            "contribution-result",
            GeometryOutputOperation::IndependentDeformationContribution,
        )
        .unwrap();

    // Both result modes preserve the same independently evaluated payload.
    // The engine records the operation; it does not combine contributions.
    assert_eq!(replacement.payload(), contribution.payload());
    assert_eq!(
        replacement.operation(),
        GeometryOutputOperation::ReplaceSourceGeometry
    );
    assert_eq!(
        contribution.operation(),
        GeometryOutputOperation::IndependentDeformationContribution
    );

    for result in [&replacement, &contribution] {
        assert_ne!(result.payload(), &original_path);
        result.validate_against(&captured).unwrap();
        assert_eq!(
            result
                .payload()
                .commands()
                .iter()
                .map(std::mem::discriminant)
                .collect::<Vec<_>>(),
            original_path
                .commands()
                .iter()
                .map(std::mem::discriminant)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            result.payload().commands().len(),
            original_path.commands().len()
        );
        assert!(
            result
                .validate_source_freshness(std::slice::from_ref(&source))
                .unwrap()
                .iter()
                .all(|freshness| freshness.is_current())
        );
    }

    assert_eq!(input.geometry(), &original_path);
    assert_eq!(source.payload(), &original_path);
}

#[test]
fn engine_rejects_a_newer_source_snapshot_before_producing_a_result() {
    let original = snapshot(curved_path(), 1);
    let captured = request(original.clone(), "captured");
    let changed = snapshot(curved_path(), 2);
    let current = request(changed, "current");
    let authored_definition = definition(&original);
    let input = GeometryValue::new(
        GeometryValueKind::AbsoluteGeometry,
        original.payload().clone(),
    );

    let failure = SingleOperatorEvaluationEngine
        .evaluate(
            &SingleHandleDisplacementOperator,
            &authored_definition,
            &input,
            &captured,
            &current,
            "stale-result",
            GeometryOutputOperation::ReplaceSourceGeometry,
        )
        .unwrap_err();

    assert_eq!(
        failure.diagnostics()[0].code(),
        EvaluationDiagnosticCode::StaleSourceSnapshot
    );
    assert_eq!(input.geometry(), original.payload());
}
