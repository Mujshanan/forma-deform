//! Paired structured-path and derived-curve output contract for FDM-05.
//!
//! The structured Path2D remains authoritative. Evaluated curve samples are a
//! separate, disposable representation for curve-accurate downstream use.

use crate::binding_evaluation::EvaluatedBindingGeometry;
use crate::curve_sample_evaluation::{EvaluatedCurveSample, EvaluatedCurveSampleSet};
use crate::path2d::{Path2D, PathCommand};
use crate::host_contract::GeometrySnapshotStamp;

/// Immutable pairing of the editable Path2D result and its evaluated curve data.
///
/// Construction checks that both products describe the same source snapshot
/// and current cage pose, and that the sample sequence covers every original
/// quadratic/cubic command in deterministic command/subpath order.
#[derive(Clone, Debug, PartialEq)]
pub struct EvaluatedDeformationOutput {
    structured: EvaluatedBindingGeometry,
    curves: EvaluatedCurveSampleSet,
}

impl EvaluatedDeformationOutput {
    pub fn new(
        structured: EvaluatedBindingGeometry,
        curves: EvaluatedCurveSampleSet,
    ) -> Result<Self, DeformationOutputError> {
        if structured.source_stamp() != curves.source_stamp() {
            return Err(DeformationOutputError::SourceStampMismatch);
        }
        if structured.binding_stamp() != curves.binding_stamp() {
            return Err(DeformationOutputError::BindingStampMismatch);
        }
        if structured.cage_geometry_fingerprint() != curves.current_cage_fingerprint() {
            return Err(DeformationOutputError::CurrentCageMismatch);
        }
        validate_curve_alignment(structured.geometry(), curves.samples())?;
        Ok(Self { structured, curves })
    }

    /// Command-preserving evaluated Path2D. This is not replaced by samples.
    pub fn structured_path(&self) -> &Path2D {
        self.structured.geometry()
    }

    /// Derived evaluated samples, grouped by original command and parameter.
    pub fn curve_samples(&self) -> &EvaluatedCurveSampleSet {
        &self.curves
    }

    pub fn source_stamp(&self) -> &GeometrySnapshotStamp {
        self.structured.source_stamp()
    }

    pub fn rest_cage_fingerprint(&self) -> &[u8; 32] {
        self.curves.rest_cage_fingerprint()
    }

    pub fn current_cage_fingerprint(&self) -> &[u8; 32] {
        self.structured.cage_geometry_fingerprint()
    }

    pub fn structured_path_fingerprint(&self) -> &crate::host_contract::GeometryFingerprint {
        self.structured.fingerprint()
    }

    pub fn mapped_features(&self) -> usize { self.structured.mapped_features() }
    pub fn clamped_features(&self) -> usize { self.structured.clamped_features() }
    pub fn extrapolated_features(&self) -> usize { self.structured.extrapolated_features() }
    pub fn unbound_features(&self) -> usize { self.structured.unbound_features() }
}

fn validate_curve_alignment(
    path: &Path2D,
    samples: &[EvaluatedCurveSample],
) -> Result<(), DeformationOutputError> {
    let mut cursor = 0usize;
    let mut subpath_index = 0u32;
    let mut saw_move_to = false;

    for (command_index, command) in path.commands().iter().enumerate() {
        match command {
            PathCommand::MoveTo(_) => {
                if saw_move_to {
                    subpath_index = subpath_index.checked_add(1)
                        .ok_or(DeformationOutputError::SubpathIndexOverflow)?;
                }
                saw_move_to = true;
            }
            PathCommand::QuadraticTo { .. } | PathCommand::CubicTo { .. } => {
                let command_index = u32::try_from(command_index)
                    .map_err(|_| DeformationOutputError::CommandIndexOverflow)?;
                let start = cursor;
                while cursor < samples.len()
                    && samples[cursor].command_index == command_index
                {
                    let sample = &samples[cursor];
                    if sample.subpath_index != subpath_index
                        || sample.sample_index != cursor - start
                        || !sample.t.is_finite()
                        || (cursor == start && sample.t != 0.0)
                        || (cursor > start && sample.t <= samples[cursor - 1].t)
                    {
                        return Err(DeformationOutputError::InvalidCurveSampleOrder {
                            command_index,
                        });
                    }
                    cursor += 1;
                }
                if cursor - start < 2 || samples[cursor - 1].t != 1.0 {
                    return Err(DeformationOutputError::IncompleteCurveSamples {
                        command_index,
                    });
                }
            }
            PathCommand::LineTo(_) | PathCommand::Close => {}
        }

        if cursor < samples.len()
            && samples[cursor].command_index < u32::try_from(command_index).unwrap_or(u32::MAX)
        {
            return Err(DeformationOutputError::UnexpectedCurveSampleCommand {
                command_index: samples[cursor].command_index,
            });
        }
    }

    if cursor != samples.len() {
        return Err(DeformationOutputError::UnexpectedCurveSampleCommand {
            command_index: samples[cursor].command_index,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeformationOutputError {
    SourceStampMismatch,
    BindingStampMismatch,
    CurrentCageMismatch,
    CommandIndexOverflow,
    SubpathIndexOverflow,
    InvalidCurveSampleOrder { command_index: u32 },
    IncompleteCurveSamples { command_index: u32 },
    UnexpectedCurveSampleCommand { command_index: u32 },
}

impl std::fmt::Display for DeformationOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceStampMismatch => write!(f, "structured path and curve samples refer to different source snapshots"),
            Self::BindingStampMismatch => write!(f, "structured path and curve samples were evaluated from different validated bindings"),
            Self::CurrentCageMismatch => write!(f, "structured path and curve samples refer to different current cage poses"),
            Self::CommandIndexOverflow => write!(f, "Path2D command index exceeds curve-sample locator capacity"),
            Self::SubpathIndexOverflow => write!(f, "Path2D subpath index exceeds curve-sample locator capacity"),
            Self::InvalidCurveSampleOrder { command_index } => write!(f, "curve samples for command {command_index} are not in canonical parameter order"),
            Self::IncompleteCurveSamples { command_index } => write!(f, "curve command {command_index} does not have a complete endpoint-inclusive sample set"),
            Self::UnexpectedCurveSampleCommand { command_index } => write!(f, "curve samples reference non-curve or missing command {command_index}"),
        }
    }
}
impl std::error::Error for DeformationOutputError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path2d::Point2;

    fn point(x: f64, y: f64) -> Point2 { Point2::new(x, y).unwrap() }

    fn sample(command_index: u32, subpath_index: u32, sample_index: usize, t: f64) -> EvaluatedCurveSample {
        EvaluatedCurveSample {
            command_index,
            subpath_index,
            sample_index,
            t,
            point: point(t, t * t),
        }
    }


    fn integration_fixture() -> (
        crate::host_contract::GeometrySnapshot,
        crate::topology::DeformationTopology,
        crate::topology::DeformationControlState,
        crate::binding::ValidatedCageGeometry,
        crate::binding_persistence::ValidatedBindingRecord,
    ) {
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
        use crate::path2d::{PATH2D_PAYLOAD_KIND, PATH2D_SCHEMA_VERSION};
        use crate::point_location::{locate_point, PointLocationOutcome};
        use crate::topology::{
            ControlPointId, ControlPointPosition, ControlPointState, ControlStateRevision,
            DeformationControlState, DeformationTopology, DeformationTopologyId,
            DeformationTopologyRevision, IrregularCageTopology, TopologyFace,
        };

        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.2, 0.2)),
            PathCommand::CubicTo {
                control1: point(0.4, 0.3),
                control2: point(0.8, 0.9),
                to: point(1.2, 1.0),
            },
        ]).unwrap();
        let coordinate_space = CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("target-local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        };
        let fingerprint = path.fingerprint(&coordinate_space).unwrap();
        let source = crate::host_contract::GeometrySnapshot::new(
            HostTargetRef {
                namespace: HostNamespace::new("anicraft").unwrap(),
                document_id: HostDocumentId::new("fdm05-output-test").unwrap(),
                target_kind: HostTargetKind::new("shape").unwrap(),
                target_id: HostTargetId::new("curve-1").unwrap(),
                generation: 1,
            },
            GeometryRevision(1),
            TopologyRevision(1),
            fingerprint,
            coordinate_space,
            PATH2D_PAYLOAD_KIND,
            PATH2D_SCHEMA_VERSION,
            path,
        ).unwrap();

        let ids = (1..=4).map(|id| ControlPointId::new(id).unwrap()).collect::<Vec<_>>();
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: DeformationTopologyId::new(91).unwrap(),
            revision: DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids: ids.clone(),
            faces: vec![
                TopologyFace::Triangle([ids[0], ids[1], ids[2]]),
                TopologyFace::Triangle([ids[2], ids[1], ids[3]]),
            ],
        });
        let positions = [(0.0, 0.0), (2.0, 0.0), (0.0, 2.0), (2.0, 2.0)];
        let rest_state = DeformationControlState {
            topology_id: topology.topology_id(),
            topology_revision: topology.topology_revision(),
            revision: ControlStateRevision::new(1).unwrap(),
            points: ids.iter().copied().zip(positions).map(|(id, (x, y))| ControlPointState {
                id,
                position: ControlPointPosition::new(x, y).unwrap(),
                pinned: false,
            }).collect(),
        };
        let certificate = validate_cage_embedding(&topology, &rest_state).unwrap();
        let rest_geometry = ValidatedCageGeometry::new(&topology, &rest_state, &certificate).unwrap();
        let stamp = BindingStamp::from_snapshot(
            &source,
            &topology,
            MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
            OutsideCagePolicy::UnboundZeroInfluence,
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

    #[test]
    fn pairs_structured_path_with_curve_samples_from_same_binding_and_pose() {
        use crate::curve_sample_evaluation::evaluate_curve_samples;
        use crate::curve_sampling::{map_curve_samples, sample_path_curves, CurveSamplingPolicy};
        use crate::embedding::validate_cage_embedding;
        use crate::binding_evaluation::evaluate_binding;
        use crate::topology::{ControlPointPosition, ControlStateRevision, TopologyView};

        let (source, topology, rest_state, rest_geometry, validated) = integration_fixture();
        let samples = sample_path_curves(source.payload(), CurveSamplingPolicy::default()).unwrap();
        let mapped = map_curve_samples(
            &samples,
            &rest_geometry,
            validated.binding().stamp().outside_policy,
        ).unwrap();

        let mut current_state = rest_state.clone();
        current_state.revision = ControlStateRevision::new(2).unwrap();
        for point in &mut current_state.points {
            point.position = ControlPointPosition::new(
                point.position.x + 0.25,
                point.position.y - 0.1,
            ).unwrap();
        }
        let certificate = validate_cage_embedding(&topology, &current_state).unwrap();
        let current_geometry = ValidatedCageGeometry::new(&topology, &current_state, &certificate).unwrap();

        let structured = evaluate_binding(&validated, &source, &topology, &current_geometry).unwrap();
        let curves = evaluate_curve_samples(
            &validated, &source, &topology, &rest_geometry, &current_geometry, &mapped,
        ).unwrap();
        let output = EvaluatedDeformationOutput::new(structured, curves).unwrap();

        assert_eq!(output.source_stamp(), &source.stamp());
        assert_eq!(output.rest_cage_fingerprint(), rest_geometry.geometry_fingerprint());
        assert_eq!(output.current_cage_fingerprint(), current_geometry.geometry_fingerprint());
        assert_eq!(output.structured_path().commands().len(), source.payload().commands().len());
        assert!(matches!(output.structured_path().commands()[1], PathCommand::CubicTo { .. }));
        assert!(output.curve_samples().samples().len() >= 2);
        assert_eq!(output.curve_samples().samples().first().unwrap().t, 0.0);
        assert_eq!(output.curve_samples().samples().last().unwrap().t, 1.0);
        assert_eq!(source.payload().commands().len(), 2);
    }

    #[test]
    fn validates_mixed_curve_commands_and_multiple_subpaths() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(1.0, 0.0)),
            PathCommand::QuadraticTo { control: point(2.0, 1.0), to: point(3.0, 0.0) },
            PathCommand::Close,
            PathCommand::MoveTo(point(4.0, 0.0)),
            PathCommand::CubicTo {
                control1: point(5.0, 1.0),
                control2: point(6.0, 1.0),
                to: point(7.0, 0.0),
            },
        ]).unwrap();
        let samples = vec![
            sample(2, 0, 0, 0.0),
            sample(2, 0, 1, 0.5),
            sample(2, 0, 2, 1.0),
            sample(5, 1, 0, 0.0),
            sample(5, 1, 1, 0.5),
            sample(5, 1, 2, 1.0),
        ];
        assert_eq!(validate_curve_alignment(&path, &samples), Ok(()));
    }

    #[test]
    fn rejects_missing_curve_endpoints_and_wrong_subpath_identity() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::CubicTo {
                control1: point(1.0, 1.0),
                control2: point(2.0, 1.0),
                to: point(3.0, 0.0),
            },
        ]).unwrap();
        assert_eq!(
            validate_curve_alignment(&path, &[sample(1, 0, 0, 0.0)]),
            Err(DeformationOutputError::IncompleteCurveSamples { command_index: 1 })
        );
        assert_eq!(
            validate_curve_alignment(&path, &[
                sample(1, 2, 0, 0.0),
                sample(1, 2, 1, 1.0),
            ]),
            Err(DeformationOutputError::InvalidCurveSampleOrder { command_index: 1 })
        );
    }

    #[test]
    fn rejects_samples_for_non_curve_commands() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(1.0, 0.0)),
        ]).unwrap();
        assert_eq!(
            validate_curve_alignment(&path, &[sample(1, 0, 0, 0.0)]),
            Err(DeformationOutputError::UnexpectedCurveSampleCommand { command_index: 1 })
        );
    }
}
