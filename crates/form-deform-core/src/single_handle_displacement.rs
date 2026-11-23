//! Deterministic single-handle displacement with compact smooth radial falloff.
//!
//! This is a reference FDM operator, not a replacement for AniCraft Asset Warp.
//! It transforms every stored Path2D point (including Bézier control points)
//! while preserving command order and segment topology.

use crate::composition_evaluation::{
    DeformerOperator, GeometryValue, OperatorContext, OperatorFailure,
};
use crate::path2d::{Path2D, Path2DError, PathCommand, Point2};
use crate::persistence::{
    DeformationDefinition, DeformationKind, GeometryValueKind, ParameterValue,
};

pub const SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_ID: &str = "fdm.single-handle-displacement";
pub const SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_VERSION: u32 = 1;

const HANDLE_X: &str = "handle_x";
const HANDLE_Y: &str = "handle_y";
const DISPLACEMENT_X: &str = "displacement_x";
const DISPLACEMENT_Y: &str = "displacement_y";
const INFLUENCE_RADIUS: &str = "influence_radius";

const REQUIRED_PARAMETERS: [&str; 5] = [
    HANDLE_X,
    HANDLE_Y,
    DISPLACEMENT_X,
    DISPLACEMENT_Y,
    INFLUENCE_RADIUS,
];

/// Stateless operator. Its complete input is the authored definition, typed
/// geometry value, and immutable evaluation context.
#[derive(Clone, Copy, Debug, Default)]
pub struct SingleHandleDisplacementOperator;

#[derive(Clone, Copy, Debug, PartialEq)]
struct HandleParameters {
    center: Point2,
    displacement: (f64, f64),
    influence_radius: f64,
}

impl HandleParameters {
    fn from_definition(definition: &DeformationDefinition) -> Result<Self, OperatorFailure> {
        let parameters = definition.parameters();
        if parameters.len() != REQUIRED_PARAMETERS.len()
            || REQUIRED_PARAMETERS
                .iter()
                .any(|key| !parameters.contains_key(*key))
        {
            return Err(OperatorFailure::InvalidParameters);
        }

        let number = |key: &str| -> Result<f64, OperatorFailure> {
            match parameters.get(key) {
                Some(ParameterValue::Number(value)) if value.is_finite() => Ok(*value),
                _ => Err(OperatorFailure::InvalidParameters),
            }
        };

        let handle_x = number(HANDLE_X)?;
        let handle_y = number(HANDLE_Y)?;
        let displacement_x = number(DISPLACEMENT_X)?;
        let displacement_y = number(DISPLACEMENT_Y)?;
        let influence_radius = number(INFLUENCE_RADIUS)?;

        if influence_radius <= 0.0 {
            return Err(OperatorFailure::InvalidParameters);
        }

        let center =
            Point2::new(handle_x, handle_y).map_err(|_| OperatorFailure::InvalidParameters)?;

        Ok(Self {
            center,
            displacement: (displacement_x, displacement_y),
            influence_radius,
        })
    }
}

impl DeformerOperator for SingleHandleDisplacementOperator {
    fn kind(&self) -> DeformationKind {
        DeformationKind::PinDeformation
    }

    fn algorithm_id(&self) -> &'static str {
        SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_ID
    }

    fn algorithm_version(&self) -> u32 {
        SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_VERSION
    }

    fn evaluate(
        &self,
        definition: &DeformationDefinition,
        input: &GeometryValue,
        _context: &OperatorContext<'_>,
    ) -> Result<GeometryValue, OperatorFailure> {
        if input.kind() != GeometryValueKind::AbsoluteGeometry
            || definition.input_kind() != GeometryValueKind::AbsoluteGeometry
            || definition.output_kind() != GeometryValueKind::AbsoluteGeometry
        {
            return Err(OperatorFailure::InputKindMismatch);
        }

        let parameters = HandleParameters::from_definition(definition)?;
        let geometry = displace_path(input.geometry(), parameters)
            .map_err(|_| OperatorFailure::NonFiniteResult)?;

        Ok(GeometryValue::new(
            GeometryValueKind::AbsoluteGeometry,
            geometry,
        ))
    }
}

/// Apply the radial displacement to every explicit Path2D point.
///
/// The falloff is compact cubic smoothstep:
///   t = distance / radius
///   w = 1 - (3 t^2 - 2 t^3), for 0 <= t < 1
/// Points at or beyond the influence radius are unchanged. The same mapping is
/// applied independently to anchors and Bézier control points.
fn displace_path(path: &Path2D, parameters: HandleParameters) -> Result<Path2D, Path2DError> {
    let commands = path
        .commands()
        .iter()
        .map(|command| {
            Ok(match command {
                PathCommand::MoveTo(point) => {
                    PathCommand::MoveTo(displace_point(*point, parameters)?)
                }
                PathCommand::LineTo(point) => {
                    PathCommand::LineTo(displace_point(*point, parameters)?)
                }
                PathCommand::QuadraticTo { control, to } => PathCommand::QuadraticTo {
                    control: displace_point(*control, parameters)?,
                    to: displace_point(*to, parameters)?,
                },
                PathCommand::CubicTo {
                    control1,
                    control2,
                    to,
                } => PathCommand::CubicTo {
                    control1: displace_point(*control1, parameters)?,
                    control2: displace_point(*control2, parameters)?,
                    to: displace_point(*to, parameters)?,
                },
                PathCommand::Close => PathCommand::Close,
            })
        })
        .collect::<Result<Vec<_>, Path2DError>>()?;

    Path2D::new(commands)
}

fn displace_point(point: Point2, parameters: HandleParameters) -> Result<Point2, Path2DError> {
    let dx = point.x() - parameters.center.x();
    let dy = point.y() - parameters.center.y();
    let radius = parameters.influence_radius;

    // A subtraction overflow means the point is certainly farther away than
    // any finite positive radius. Avoid allowing an infinite intermediate into
    // the falloff calculation.
    if !dx.is_finite() || !dy.is_finite() || dx.abs() > radius || dy.abs() > radius {
        return Ok(point);
    }

    // Normalizing each component first keeps the hypot input bounded and
    // avoids squaring large coordinates.
    let normalized_distance = (dx / radius).hypot(dy / radius);
    if !normalized_distance.is_finite() || normalized_distance >= 1.0 {
        return Ok(point);
    }

    let t = normalized_distance;
    let smoothstep = t * t * (3.0 - 2.0 * t);
    let weight = 1.0 - smoothstep;
    let x = point.x() + parameters.displacement.0 * weight;
    let y = point.y() + parameters.displacement.1 * weight;
    Point2::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition_evaluation::{OperatorContext, evaluate_operator};
    use crate::evaluation_contract::{
        EvaluationRequest, EvaluationTimeSample, ModelRevisionStamp, TimeSampleKind,
    };
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceDescriptor, CoordinateSpaceId,
        CoordinateSpaceKind, GeometryRevision, GeometrySnapshot, HostDocumentId, HostNamespace,
        HostTargetId, HostTargetKind, HostTargetRef, TopologyRevision,
    };
    use crate::persistence::ParameterValue;
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

    fn space() -> CoordinateSpaceDescriptor {
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

    fn path(commands: Vec<PathCommand>) -> Path2D {
        Path2D::new(commands).unwrap()
    }

    fn point(x: f64, y: f64) -> Point2 {
        Point2::new(x, y).unwrap()
    }

    fn sample_path() -> Path2D {
        path(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(5.0, 0.0)),
            PathCommand::QuadraticTo {
                control: point(5.0, 5.0),
                to: point(10.0, 5.0),
            },
            PathCommand::CubicTo {
                control1: point(10.0, 10.0),
                control2: point(15.0, 10.0),
                to: point(15.0, 5.0),
            },
            PathCommand::Close,
        ])
    }

    fn definition(
        parameters: BTreeMap<String, ParameterValue>,
    ) -> Result<
        (EvaluationRequest, DeformationDefinition),
        crate::persistence::PersistenceError,
    > {
        let source_path = sample_path();
        let coordinate_space = space();
        let fingerprint = source_path.fingerprint(&coordinate_space).unwrap();
        let target = target();
        let snapshot = GeometrySnapshot::new(
            target.clone(),
            GeometryRevision(1),
            TopologyRevision(1),
            fingerprint,
            coordinate_space.clone(),
            crate::path2d::PATH2D_PAYLOAD_KIND,
            crate::path2d::PATH2D_SCHEMA_VERSION,
            source_path,
        )
        .unwrap();
        let reference = snapshot.stamp();
        let request = EvaluationRequest::new(
            "request-1",
            vec![snapshot],
            ModelRevisionStamp::new(
                "model-1",
                1,
                1,
                SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_ID,
                SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_VERSION,
            )
            .unwrap(),
            EvaluationTimeSample::new(
                "frames",
                "sample-1",
                1,
                0.0,
                1,
                TimeSampleKind::EvaluatedPose,
            )
            .unwrap(),
            vec![],
            "stage-1",
            None,
        )
        .unwrap();
        let definition = DeformationDefinition::new(
            "single-handle-1",
            1,
            "Single Handle",
            DeformationKind::PinDeformation,
            target,
            reference,
            coordinate_space,
            GeometryValueKind::AbsoluteGeometry,
            GeometryValueKind::AbsoluteGeometry,
            parameters,
        )?;
        Ok((request, definition))
    }

    fn parameters(
        center: (f64, f64),
        displacement: (f64, f64),
        radius: f64,
    ) -> BTreeMap<String, ParameterValue> {
        BTreeMap::from([
            (HANDLE_X.into(), ParameterValue::Number(center.0)),
            (HANDLE_Y.into(), ParameterValue::Number(center.1)),
            (
                DISPLACEMENT_X.into(),
                ParameterValue::Number(displacement.0),
            ),
            (
                DISPLACEMENT_Y.into(),
                ParameterValue::Number(displacement.1),
            ),
            (INFLUENCE_RADIUS.into(), ParameterValue::Number(radius)),
        ])
    }

    fn run(
        path: &Path2D,
        parameters: BTreeMap<String, ParameterValue>,
    ) -> Result<GeometryValue, crate::composition_evaluation::OperatorInvocationError> {
        let (request, definition) = definition(parameters).unwrap();
        let context = OperatorContext::new(&request).unwrap();
        let input = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, path.clone());
        evaluate_operator(
            &SingleHandleDisplacementOperator,
            &definition,
            &input,
            &context,
        )
    }

    #[test]
    fn operator_has_stable_identity_and_version() {
        let operator = SingleHandleDisplacementOperator;
        assert_eq!(
            operator.algorithm_id(),
            SINGLE_HANDLE_DISPLACEMENT_ALGORITHM_ID
        );
        assert_eq!(operator.algorithm_version(), 1);
        assert_eq!(operator.kind(), DeformationKind::PinDeformation);
    }

    #[test]
    fn handle_center_receives_full_displacement() {
        let input = path(vec![PathCommand::MoveTo(point(0.0, 0.0))]);
        let output = run(&input, parameters((0.0, 0.0), (8.0, -4.0), 10.0)).unwrap();
        assert_eq!(
            output.geometry().commands(),
            &[PathCommand::MoveTo(point(8.0, -4.0))]
        );
    }

    #[test]
    fn smoothstep_midpoint_receives_half_displacement() {
        let input = path(vec![PathCommand::MoveTo(point(5.0, 0.0))]);
        let output = run(&input, parameters((0.0, 0.0), (8.0, 0.0), 10.0)).unwrap();
        assert_eq!(
            output.geometry().commands(),
            &[PathCommand::MoveTo(point(9.0, 0.0))]
        );
    }

    #[test]
    fn influence_boundary_and_exterior_are_unchanged() {
        let input = path(vec![
            PathCommand::MoveTo(point(10.0, 0.0)),
            PathCommand::LineTo(point(11.0, 0.0)),
        ]);
        let output = run(&input, parameters((0.0, 0.0), (8.0, 0.0), 10.0)).unwrap();
        assert_eq!(output.geometry(), &input);
    }

    #[test]
    fn zero_displacement_is_identity() {
        let input = sample_path();
        let output = run(&input, parameters((0.0, 0.0), (0.0, 0.0), 100.0)).unwrap();
        assert_eq!(output.geometry(), &input);
    }

    #[test]
    fn repeated_evaluation_is_deterministic_and_non_accumulating() {
        let input = sample_path();
        let params = parameters((0.0, 0.0), (3.0, -2.0), 20.0);
        let first = run(&input, params.clone()).unwrap();
        let second = run(&input, params).unwrap();
        assert_eq!(first, second);
        assert_eq!(input.commands().len(), first.geometry().commands().len());
    }

    #[test]
    fn preserves_path_command_kinds_order_and_close_commands() {
        let input = sample_path();
        let output = run(&input, parameters((0.0, 0.0), (3.0, -2.0), 100.0)).unwrap();
        let kinds = |path: &Path2D| {
            path.commands()
                .iter()
                .map(|command| match command {
                    PathCommand::MoveTo(_) => 1,
                    PathCommand::LineTo(_) => 2,
                    PathCommand::QuadraticTo { .. } => 3,
                    PathCommand::CubicTo { .. } => 4,
                    PathCommand::Close => 5,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds(output.geometry()), kinds(&input));
    }

    #[test]
    fn quadratic_and_cubic_control_points_are_transformed_too() {
        let input = sample_path();
        let output = run(&input, parameters((0.0, 0.0), (2.0, 0.0), 100.0)).unwrap();
        match (&input.commands()[2], &output.geometry().commands()[2]) {
            (
                PathCommand::QuadraticTo {
                    control: before, ..
                },
                PathCommand::QuadraticTo { control: after, .. },
            ) => {
                assert_ne!(before, after);
            }
            _ => panic!("quadratic command kind changed"),
        }
        match (&input.commands()[3], &output.geometry().commands()[3]) {
            (
                PathCommand::CubicTo {
                    control1: before, ..
                },
                PathCommand::CubicTo {
                    control1: after, ..
                },
            ) => {
                assert_ne!(before, after);
            }
            _ => panic!("cubic command kind changed"),
        }
    }

    #[test]
    fn invalid_radius_and_parameter_schema_are_rejected() {
        let input = sample_path();
        assert!(matches!(
            run(&input, parameters((0.0, 0.0), (1.0, 1.0), 0.0)),
            Err(
                crate::composition_evaluation::OperatorInvocationError::Operator(
                    OperatorFailure::InvalidParameters
                )
            )
        ));
        let mut missing = parameters((0.0, 0.0), (1.0, 1.0), 10.0);
        missing.remove(HANDLE_X);
        assert!(matches!(
            run(&input, missing),
            Err(
                crate::composition_evaluation::OperatorInvocationError::Operator(
                    OperatorFailure::InvalidParameters
                )
            )
        ));
        let mut extra = parameters((0.0, 0.0), (1.0, 1.0), 10.0);
        extra.insert("unexpected".into(), ParameterValue::Boolean(true));
        assert!(matches!(
            run(&input, extra),
            Err(
                crate::composition_evaluation::OperatorInvocationError::Operator(
                    OperatorFailure::InvalidParameters
                )
            )
        ));
    }

    #[test]
    fn non_finite_parameters_are_rejected_by_authored_definition_validation() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(definition(parameters((0.0, 0.0), (invalid, 1.0), 10.0)).is_err());
            assert!(definition(parameters((0.0, 0.0), (1.0, 1.0), invalid)).is_err());
        }
    }

    #[test]
    fn tiny_radius_and_extreme_coordinates_remain_finite_and_deterministic() {
        let input = path(vec![
            PathCommand::MoveTo(point(f64::MAX, 0.0)),
            PathCommand::LineTo(point(-f64::MAX, 0.0)),
        ]);
        let output = run(
            &input,
            parameters((0.0, 0.0), (1.0, 1.0), f64::MIN_POSITIVE),
        )
        .unwrap();
        assert_eq!(output.geometry(), &input);
    }

    #[test]
    fn nonlinear_curve_mapping_is_an_explicit_control_point_approximation() {
        let input = path(vec![
            PathCommand::MoveTo(point(-1.0, 0.0)),
            PathCommand::QuadraticTo {
                control: point(0.0, 2.0),
                to: point(1.0, 0.0),
            },
        ]);
        let parameters = HandleParameters {
            center: point(0.0, 1.0),
            displacement: (0.0, 2.0),
            influence_radius: 3.0,
        };
        let transformed = displace_path(&input, parameters).unwrap();

        let original_curve_midpoint = point(0.0, 1.0);
        let pointwise_mapped_midpoint =
            displace_point(original_curve_midpoint, parameters).unwrap();

        let (mapped_start, mapped_control, mapped_end) = match transformed.commands() {
            [
                PathCommand::MoveTo(start),
                PathCommand::QuadraticTo { control, to: end },
            ] => (*start, *control, *end),
            _ => panic!("quadratic path topology changed"),
        };
        let represented_curve_midpoint = point(
            0.25 * mapped_start.x() + 0.5 * mapped_control.x() + 0.25 * mapped_end.x(),
            0.25 * mapped_start.y() + 0.5 * mapped_control.y() + 0.25 * mapped_end.y(),
        );

        assert_ne!(represented_curve_midpoint, pointwise_mapped_midpoint);
    }

    #[test]
    fn output_overflow_is_rejected() {
        let input = path(vec![PathCommand::MoveTo(point(f64::MAX, 0.0))]);
        assert!(matches!(
            run(&input, parameters((f64::MAX, 0.0), (f64::MAX, 0.0), 10.0)),
            Err(
                crate::composition_evaluation::OperatorInvocationError::Operator(
                    OperatorFailure::NonFiniteResult
                )
            )
        ));
    }
}
