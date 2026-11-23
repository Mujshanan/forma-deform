//! FDM-05.3 assembly of authoritative structured geometry and refined curves.
//!
//! The structured Path2D remains authoritative. Refined output-space samples
//! are attached by original command/subpath identity and never replace it.

use crate::binding::{OutsideCagePolicy, ValidatedCageGeometry};
use crate::binding_persistence::ValidatedBindingRecord;
use crate::binding_evaluation::EvaluatedBindingGeometry;
use crate::host_contract::GeometrySnapshot;
use crate::output_space_refinement::{
    refine_path_curves, OutputRefinementError, OutputRefinementPolicy,
    RefinedCurveSampleSet, OUTPUT_REFINEMENT_POLICY_VERSION,
};
use crate::path2d::{PathCommand, Path2D};
use crate::topology::DeformationTopology;

#[derive(Clone, Debug, PartialEq)]
pub struct AssembledDeformationOutput {
    structured: EvaluatedBindingGeometry,
    refined_curves: RefinedCurveSampleSet,
}

impl AssembledDeformationOutput {
    /// Build the derived representation from the same validated inputs that
    /// produced the structured result. Refinement is performed inside this
    /// constructor so callers cannot accidentally pair samples from another
    /// source, binding, or cage pose.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        structured: EvaluatedBindingGeometry,
        validated: &ValidatedBindingRecord,
        source: &GeometrySnapshot,
        topology: &DeformationTopology,
        rest_cage: &ValidatedCageGeometry,
        current_cage: &ValidatedCageGeometry,
        outside_policy: OutsideCagePolicy,
        policy: OutputRefinementPolicy,
    ) -> Result<Self, DeformationOutputAssemblyError> {
        if structured.source_stamp() != &source.stamp() {
            return Err(DeformationOutputAssemblyError::SourceStampMismatch);
        }
        if structured.binding_stamp() != validated.binding().stamp() {
            return Err(DeformationOutputAssemblyError::BindingStampMismatch);
        }
        if structured.rest_cage_fingerprint() != rest_cage.geometry_fingerprint()
            || validated.rest_cage_fingerprint() != rest_cage.geometry_fingerprint()
        {
            return Err(DeformationOutputAssemblyError::RestCageMismatch);
        }
        if structured.cage_geometry_fingerprint() != current_cage.geometry_fingerprint() {
            return Err(DeformationOutputAssemblyError::CurrentCageMismatch);
        }

        let refined_curves = refine_path_curves(
            validated, source, topology, rest_cage, current_cage, outside_policy, policy,
        ).map_err(DeformationOutputAssemblyError::Refinement)?;
        validate_refined_alignment(structured.geometry(), &refined_curves)?;
        Ok(Self { structured, refined_curves })
    }

    /// Authoritative command-preserving evaluated geometry.
    pub fn structured_path(&self) -> &Path2D {
        self.structured.geometry()
    }

    /// Derived samples, grouped by original command and subpath.
    pub fn refined_curves(&self) -> &RefinedCurveSampleSet {
        &self.refined_curves
    }

    pub fn structured(&self) -> &EvaluatedBindingGeometry {
        &self.structured
    }
}

fn validate_refined_alignment(
    path: &Path2D,
    refined: &RefinedCurveSampleSet,
) -> Result<(), DeformationOutputAssemblyError> {
    if refined.policy_version != OUTPUT_REFINEMENT_POLICY_VERSION
        || refined.policy.validate().is_err()
        || !refined.max_observed_deviation.is_finite()
        || refined.max_observed_deviation < 0.0
        || refined.max_observed_deviation > refined.policy.max_deviation
    {
        return Err(DeformationOutputAssemblyError::InvalidRefinementMetadata);
    }

    let mut expected = Vec::new();
    let mut subpath = 0u32;
    let mut saw_move = false;
    let mut current = false;
    for (index, command) in path.commands().iter().enumerate() {
        match command {
            PathCommand::MoveTo(_) => {
                if saw_move {
                    subpath = subpath.checked_add(1)
                        .ok_or(DeformationOutputAssemblyError::SubpathIndexOverflow)?;
                }
                saw_move = true;
                current = true;
            }
            PathCommand::LineTo(_) => {
                if !current {
                    return Err(DeformationOutputAssemblyError::InvalidPathSequence { command_index: index });
                }
            }
            PathCommand::QuadraticTo { .. } | PathCommand::CubicTo { .. } => {
                if !current {
                    return Err(DeformationOutputAssemblyError::InvalidPathSequence { command_index: index });
                }
                let command_index = u32::try_from(index)
                    .map_err(|_| DeformationOutputAssemblyError::CommandIndexOverflow)?;
                expected.push((command_index, subpath));
            }
            PathCommand::Close => {
                if !current {
                    return Err(DeformationOutputAssemblyError::InvalidPathSequence { command_index: index });
                }
                current = false;
            }
        }
    }

    if refined.segments.len() != expected.len() {
        return Err(DeformationOutputAssemblyError::IncompleteCommandCoverage {
            expected: expected.len(),
            actual: refined.segments.len(),
        });
    }

    let mut counted_samples = 0usize;
    for (segment, (command_index, subpath_index)) in refined.segments.iter().zip(expected) {
        if segment.command_index != command_index || segment.subpath_index != subpath_index {
            return Err(DeformationOutputAssemblyError::UnexpectedCommandIdentity {
                expected_command: command_index,
                actual_command: segment.command_index,
            });
        }
        if segment.samples.len() < 2
            || segment.samples.first().is_none_or(|sample| sample.t != 0.0)
            || segment.samples.last().is_none_or(|sample| sample.t != 1.0)
        {
            return Err(DeformationOutputAssemblyError::IncompleteCurveSamples {
                command_index,
            });
        }
        for (sample_index, sample) in segment.samples.iter().enumerate() {
            if !sample.t.is_finite()
                || !(0.0..=1.0).contains(&sample.t)
                || !sample.source_point.x().is_finite()
                || !sample.source_point.y().is_finite()
                || !sample.output_point.x().is_finite()
                || !sample.output_point.y().is_finite()
                || (sample_index > 0 && sample.t <= segment.samples[sample_index - 1].t)
            {
                return Err(DeformationOutputAssemblyError::InvalidSampleOrder {
                    command_index,
                });
            }
        }
        counted_samples = counted_samples.checked_add(segment.samples.len())
            .ok_or(DeformationOutputAssemblyError::CountOverflow)?;
    }
    if counted_samples != refined.total_samples {
        return Err(DeformationOutputAssemblyError::SampleCountMismatch {
            reported: refined.total_samples,
            actual: counted_samples,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeformationOutputAssemblyError {
    SourceStampMismatch,
    BindingStampMismatch,
    RestCageMismatch,
    CurrentCageMismatch,
    Refinement(OutputRefinementError),
    InvalidRefinementMetadata,
    InvalidPathSequence { command_index: usize },
    CommandIndexOverflow,
    SubpathIndexOverflow,
    IncompleteCommandCoverage { expected: usize, actual: usize },
    UnexpectedCommandIdentity { expected_command: u32, actual_command: u32 },
    IncompleteCurveSamples { command_index: u32 },
    InvalidSampleOrder { command_index: u32 },
    CountOverflow,
    SampleCountMismatch { reported: usize, actual: usize },
}

impl std::fmt::Display for DeformationOutputAssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "deformation output assembly failed: {self:?}")
    }
}
impl std::error::Error for DeformationOutputAssemblyError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output_space_refinement::{RefinedCurveSample, RefinedCurveSegment};

    fn point(x: f64, y: f64) -> crate::path2d::Point2 {
        crate::path2d::Point2::new(x, y).unwrap()
    }

    fn segment(command_index: u32, subpath_index: u32) -> RefinedCurveSegment {
        RefinedCurveSegment {
            command_index,
            subpath_index,
            samples: vec![
                RefinedCurveSample { t: 0.0, source_point: point(0.0, 0.0), output_point: point(0.0, 0.0) },
                RefinedCurveSample { t: 0.5, source_point: point(0.5, 0.5), output_point: point(0.5, 0.4) },
                RefinedCurveSample { t: 1.0, source_point: point(1.0, 0.0), output_point: point(1.0, 0.0) },
            ],
        }
    }

    fn refined(segments: Vec<RefinedCurveSegment>) -> RefinedCurveSampleSet {
        let total_samples = segments.iter().map(|segment| segment.samples.len()).sum();
        RefinedCurveSampleSet {
            policy_version: OUTPUT_REFINEMENT_POLICY_VERSION,
            policy: OutputRefinementPolicy::default(),
            segments,
            total_samples,
            total_evaluations: total_samples,
            max_observed_deviation: 0.1,
        }
    }


    // End-to-end constructor fixture: the structured result and all provenance
    // inputs are produced through the public binding/evaluation APIs.
    mod constructor_tests {
        use super::*;
        use crate::binding::{
            enumerate_source_features, BindingRecord, BindingStamp, FeatureBindingEntry,
            MappingPolicy, SourceFeatureBinding,
        };
        use crate::binding_evaluation::evaluate_binding;
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
            IrregularCageTopology, TopologyFace,
        };

        fn source_path() -> Path2D {
            Path2D::new(vec![
                PathCommand::MoveTo(point(0.2, 0.2)),
                PathCommand::LineTo(point(0.8, 0.2)),
                PathCommand::QuadraticTo { control: point(1.0, 0.8), to: point(1.2, 0.5) },
                PathCommand::Close,
                PathCommand::MoveTo(point(0.3, 0.3)),
                PathCommand::CubicTo {
                    control1: point(0.5, 0.6),
                    control2: point(0.8, 0.7),
                    to: point(1.0, 0.8),
                },
            ]).unwrap()
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

        fn target() -> HostTargetRef {
            HostTargetRef {
                namespace: HostNamespace::new("anicraft").unwrap(),
                document_id: HostDocumentId::new("assembly-constructor-doc").unwrap(),
                target_kind: HostTargetKind::new("shape").unwrap(),
                target_id: HostTargetId::new("assembly-shape").unwrap(),
                generation: 1,
            }
        }

        fn snapshot(path: Path2D, revision: u64) -> GeometrySnapshot {
            let coordinate_space = space();
            let fingerprint = path.fingerprint(&coordinate_space).unwrap();
            GeometrySnapshot::new(
                target(), GeometryRevision(revision), TopologyRevision(1), fingerprint,
                coordinate_space, PATH2D_PAYLOAD_KIND, 1, path,
            ).unwrap()
        }

        fn topology_and_state(revision: u64, dx: f64) -> (DeformationTopology, DeformationControlState) {
            let ids = (1..=4).map(|value| ControlPointId::new(value).unwrap()).collect::<Vec<_>>();
            let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
                id: DeformationTopologyId::new(71).unwrap(),
                revision: DeformationTopologyRevision::new(1).unwrap(),
                control_point_ids: ids.clone(),
                faces: vec![
                    TopologyFace::Triangle([ids[0], ids[1], ids[2]]),
                    TopologyFace::Triangle([ids[2], ids[1], ids[3]]),
                ],
            });
            let positions = [(0.0, 0.0), (2.0, 0.0), (0.0, 2.0), (2.0, 2.0)];
            let state = DeformationControlState {
                topology_id: topology.topology_id(),
                topology_revision: topology.topology_revision(),
                revision: ControlStateRevision::new(revision).unwrap(),
                points: ids.into_iter().zip(positions).map(|(id, (x, y))| ControlPointState {
                    id,
                    position: ControlPointPosition::new(x + dx, y).unwrap(),
                    pinned: false,
                }).collect(),
            };
            (topology, state)
        }

        fn validated_binding(
            source: &GeometrySnapshot,
            topology: &DeformationTopology,
            rest_state: &DeformationControlState,
            rest_geometry: &ValidatedCageGeometry,
            outside_policy: OutsideCagePolicy,
        ) -> ValidatedBindingRecord {
            let stamp = BindingStamp::from_snapshot(
                source, topology,
                MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
                outside_policy,
            ).unwrap();
            let entries = enumerate_source_features(source.payload()).unwrap().into_iter().map(|feature| {
                let status = match locate_point(feature.point, rest_geometry).unwrap() {
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
            PersistedBinding::new(BindingRecord::new(stamp, entries)).unwrap()
                .validate_against(source, topology, rest_state).unwrap()
        }

        struct Fixture {
            source: GeometrySnapshot,
            topology: DeformationTopology,
            rest_state: DeformationControlState,
            rest_geometry: ValidatedCageGeometry,
            validated: ValidatedBindingRecord,
            current_geometry: ValidatedCageGeometry,
            structured: EvaluatedBindingGeometry,
        }

        fn fixture() -> Fixture {
            let source = snapshot(source_path(), 1);
            let (topology, rest_state) = topology_and_state(1, 0.0);
            let rest_certificate = validate_cage_embedding(&topology, &rest_state).unwrap();
            let rest_geometry = ValidatedCageGeometry::new(&topology, &rest_state, &rest_certificate).unwrap();
            let validated = validated_binding(
                &source, &topology, &rest_state, &rest_geometry,
                OutsideCagePolicy::UnboundZeroInfluence,
            );
            let (_, current_state) = topology_and_state(2, 0.25);
            let current_certificate = validate_cage_embedding(&topology, &current_state).unwrap();
            let current_geometry = ValidatedCageGeometry::new(&topology, &current_state, &current_certificate).unwrap();
            let structured = evaluate_binding(&validated, &source, &topology, &current_geometry).unwrap();
            Fixture { source, topology, rest_state, rest_geometry, validated, current_geometry, structured }
        }

        #[test]
        fn public_constructor_assembles_matching_evaluated_inputs_and_preserves_authored_path() {
            let f = fixture();
            let original = f.source.payload().clone();
            let assembled = AssembledDeformationOutput::build(
                f.structured, &f.validated, &f.source, &f.topology,
                &f.rest_geometry, &f.current_geometry,
                OutsideCagePolicy::UnboundZeroInfluence, OutputRefinementPolicy::default(),
            ).unwrap();

            assert_eq!(assembled.structured_path().commands(), original.commands());
            assert_eq!(assembled.refined_curves().segments.len(), 2);
            assert_eq!(assembled.refined_curves().segments[0].command_index, 2);
            assert_eq!(assembled.refined_curves().segments[0].subpath_index, 0);
            assert_eq!(assembled.refined_curves().segments[1].command_index, 5);
            assert_eq!(assembled.refined_curves().segments[1].subpath_index, 1);
            assert!(matches!(assembled.structured_path().commands()[2],
                PathCommand::QuadraticTo { control, .. } if control == point(1.0, 0.8)));
            assert!(matches!(assembled.structured_path().commands()[5],
                PathCommand::CubicTo { control1, control2, .. }
                    if control1 == point(0.5, 0.6) && control2 == point(0.8, 0.7)));
            assert_eq!(f.source.payload(), &original);
        }

        #[test]
        fn public_constructor_rejects_source_binding_and_cage_provenance_mismatches() {
            let f = fixture();

            let changed_source = snapshot(source_path(), 2);
            assert_eq!(
                AssembledDeformationOutput::build(
                    f.structured.clone(), &f.validated, &changed_source, &f.topology,
                    &f.rest_geometry, &f.current_geometry,
                    OutsideCagePolicy::UnboundZeroInfluence, OutputRefinementPolicy::default(),
                ).unwrap_err(),
                DeformationOutputAssemblyError::SourceStampMismatch,
            );

            let alternate_validated = validated_binding(
                &f.source, &f.topology, &f.rest_state, &f.rest_geometry,
                OutsideCagePolicy::ClampToBoundary,
            );
            assert_eq!(
                AssembledDeformationOutput::build(
                    f.structured.clone(), &alternate_validated, &f.source, &f.topology,
                    &f.rest_geometry, &f.current_geometry,
                    OutsideCagePolicy::ClampToBoundary, OutputRefinementPolicy::default(),
                ).unwrap_err(),
                DeformationOutputAssemblyError::BindingStampMismatch,
            );

            let (_, alternate_rest_state) = topology_and_state(3, 0.1);
            let alternate_rest_certificate = validate_cage_embedding(&f.topology, &alternate_rest_state).unwrap();
            let alternate_rest = ValidatedCageGeometry::new(
                &f.topology, &alternate_rest_state, &alternate_rest_certificate,
            ).unwrap();
            assert_eq!(
                AssembledDeformationOutput::build(
                    f.structured.clone(), &f.validated, &f.source, &f.topology,
                    &alternate_rest, &f.current_geometry,
                    OutsideCagePolicy::UnboundZeroInfluence, OutputRefinementPolicy::default(),
                ).unwrap_err(),
                DeformationOutputAssemblyError::RestCageMismatch,
            );

            let (_, alternate_current_state) = topology_and_state(4, 0.5);
            let alternate_current_certificate = validate_cage_embedding(&f.topology, &alternate_current_state).unwrap();
            let alternate_current = ValidatedCageGeometry::new(
                &f.topology, &alternate_current_state, &alternate_current_certificate,
            ).unwrap();
            assert_eq!(
                AssembledDeformationOutput::build(
                    f.structured, &f.validated, &f.source, &f.topology,
                    &f.rest_geometry, &alternate_current,
                    OutsideCagePolicy::UnboundZeroInfluence, OutputRefinementPolicy::default(),
                ).unwrap_err(),
                DeformationOutputAssemblyError::CurrentCageMismatch,
            );
        }

        #[test]
        fn public_constructor_rejects_outside_policy_that_differs_from_binding_stamp() {
            let f = fixture();
            assert!(matches!(
                AssembledDeformationOutput::build(
                    f.structured, &f.validated, &f.source, &f.topology,
                    &f.rest_geometry, &f.current_geometry,
                    OutsideCagePolicy::ClampToBoundary, OutputRefinementPolicy::default(),
                ),
                Err(DeformationOutputAssemblyError::Refinement(
                    OutputRefinementError::InvalidSourceSnapshot
                )),
            ));
        }
    }

    #[test]
    fn validates_mixed_commands_and_multiple_subpaths_without_flattening_path() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(1.0, 0.0)),
            PathCommand::QuadraticTo { control: point(1.5, 1.0), to: point(2.0, 0.0) },
            PathCommand::Close,
            PathCommand::MoveTo(point(3.0, 0.0)),
            PathCommand::CubicTo { control1: point(4.0, 1.0), control2: point(5.0, 1.0), to: point(6.0, 0.0) },
            PathCommand::LineTo(point(7.0, 0.0)),
        ]).unwrap();
        let assembled = refined(vec![segment(2, 0), segment(5, 1)]);
        assert_eq!(validate_refined_alignment(&path, &assembled), Ok(()));
        assert_eq!(path.commands().len(), 7);
        assert!(matches!(path.commands()[2], PathCommand::QuadraticTo { .. }));
        assert!(matches!(path.commands()[5], PathCommand::CubicTo { .. }));
    }

    #[test]
    fn rejects_missing_extra_or_misordered_curve_commands() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::QuadraticTo { control: point(1.0, 1.0), to: point(2.0, 0.0) },
            PathCommand::MoveTo(point(3.0, 0.0)),
            PathCommand::CubicTo { control1: point(4.0, 1.0), control2: point(5.0, 1.0), to: point(6.0, 0.0) },
        ]).unwrap();
        assert!(matches!(
            validate_refined_alignment(&path, &refined(vec![segment(1, 0)])),
            Err(DeformationOutputAssemblyError::IncompleteCommandCoverage { .. })
        ));
        assert!(matches!(
            validate_refined_alignment(&path, &refined(vec![segment(3, 1), segment(1, 0)])),
            Err(DeformationOutputAssemblyError::UnexpectedCommandIdentity { .. })
        ));
        assert!(matches!(
            validate_refined_alignment(&path, &refined(vec![segment(1, 0), segment(99, 1)])),
            Err(DeformationOutputAssemblyError::UnexpectedCommandIdentity { .. })
        ));
    }

    #[test]
    fn rejects_non_monotonic_parameters_and_sample_count_mismatch() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::CubicTo { control1: point(1.0, 1.0), control2: point(2.0, 1.0), to: point(3.0, 0.0) },
        ]).unwrap();
        let mut invalid = segment(1, 0);
        invalid.samples[1].t = 0.0;
        assert!(matches!(
            validate_refined_alignment(&path, &refined(vec![invalid])),
            Err(DeformationOutputAssemblyError::InvalidSampleOrder { .. })
        ));
        let mut wrong_count = refined(vec![segment(1, 0)]);
        wrong_count.total_samples += 1;
        assert!(matches!(
            validate_refined_alignment(&path, &wrong_count),
            Err(DeformationOutputAssemblyError::SampleCountMismatch { .. })
        ));
    }
}
