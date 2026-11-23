use form_deform_core::binding::{
    BindingRecord, BindingStamp, FeatureBindingEntry, MappingPolicy, OutsideCagePolicy,
    SourceFeatureBinding,
};
use form_deform_core::binding_persistence::{
    load_validated_binding, persist_binding, BindingDiagnostic, BindingHostAdapter,
    HostBindingContext, LoadBindingError, PersistBindingError, PersistedBinding,
};
use form_deform_core::embedding::validate_cage_embedding;
use form_deform_core::binding::ValidatedCageGeometry;
use form_deform_core::curve_sampling::{
    map_curve_samples, sample_path_curves, CurveMappingError, CurveSampleMapping,
    CurveSamplingPolicy,
};
use form_deform_core::host_contract::{
    AxisOrientation, CoordinateOrigin, CoordinateSpaceDescriptor, CoordinateSpaceId,
    CoordinateSpaceKind, GeometryRevision, GeometrySnapshot, HostDocumentId, HostNamespace,
    HostTargetId, HostTargetKind, HostTargetRef, TopologyRevision,
};
use form_deform_core::path2d::{
    Path2D, PathCommand, Point2, PATH2D_PAYLOAD_KIND, PATH2D_SCHEMA_VERSION,
};
use form_deform_core::topology::{
    ControlPointId, ControlPointPosition, ControlPointState, ControlStateRevision,
    DeformationControlState, DeformationTopology, DeformationTopologyId,
    DeformationTopologyRevision, IrregularCageTopology, TopologyFace,
};

fn point(x: f64, y: f64) -> Point2 {
    Point2::new(x, y).expect("finite fixture point")
}

fn target(generation: u64) -> HostTargetRef {
    HostTargetRef {
        namespace: HostNamespace::new("anicraft").unwrap(),
        document_id: HostDocumentId::new("integration-doc").unwrap(),
        target_kind: HostTargetKind::new("shape").unwrap(),
        target_id: HostTargetId::new("shape-1").unwrap(),
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

fn source_path(offset: f64) -> Path2D {
    Path2D::new(vec![
        PathCommand::MoveTo(point(0.5 + offset, 0.5)),
        PathCommand::LineTo(point(1.0 + offset, 0.5)),
        PathCommand::CubicTo {
            control1: point(1.25 + offset, 0.75),
            control2: point(0.75 + offset, 1.25),
            to: point(0.5 + offset, 1.0),
        },
    ])
    .unwrap()
}

fn snapshot(
    target_ref: HostTargetRef,
    geometry_revision: u64,
    topology_revision: u64,
    path: Path2D,
) -> GeometrySnapshot {
    let space = coordinate_space();
    let fingerprint = path.fingerprint(&space).unwrap();
    GeometrySnapshot::new(
        target_ref,
        GeometryRevision(geometry_revision),
        TopologyRevision(topology_revision),
        fingerprint,
        space,
        PATH2D_PAYLOAD_KIND,
        PATH2D_SCHEMA_VERSION,
        path,
    )
    .unwrap()
}

fn cage(
    topology_id: u64,
    topology_revision: u64,
) -> (DeformationTopology, DeformationControlState) {
    let id = DeformationTopologyId::new(topology_id).unwrap();
    let revision = DeformationTopologyRevision::new(topology_revision).unwrap();
    let ids = (1..=4)
        .map(|value| ControlPointId::new(value).unwrap())
        .collect::<Vec<_>>();
    let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
        id,
        revision,
        control_point_ids: ids.clone(),
        faces: vec![
            TopologyFace::Triangle([ids[0], ids[1], ids[2]]),
            TopologyFace::Triangle([ids[2], ids[1], ids[3]]),
        ],
    });
    let positions = [(0.0, 0.0), (2.0, 0.0), (0.0, 2.0), (2.0, 2.0)];
    let state = DeformationControlState {
        topology_id: id,
        topology_revision: revision,
        revision: ControlStateRevision::new(1).unwrap(),
        points: ids
            .into_iter()
            .zip(positions)
            .map(|(id, (x, y))| ControlPointState {
                id,
                position: ControlPointPosition::new(x, y).unwrap(),
                pinned: false,
            })
            .collect(),
    };
    (topology, state)
}

fn fixture() -> (HostBindingContext, PersistedBinding) {
    let target_ref = target(1);
    let source = snapshot(target_ref, 1, 1, source_path(0.0));
    let (topology, control_state) = cage(1, 1);
    let stamp = BindingStamp::from_snapshot(
        &source,
        &topology,
        MappingPolicy::TriangleBarycentricQuadBilinearInsideV1,
        OutsideCagePolicy::UnboundZeroInfluence,
    )
    .unwrap();
    let entries = form_deform_core::binding::enumerate_source_features(source.payload())
        .unwrap()
        .into_iter()
        .map(|feature| FeatureBindingEntry {
            locator: feature.locator,
            status: SourceFeatureBinding::Unbound,
        })
        .collect();
    let persisted = PersistedBinding::new(BindingRecord::new(stamp, entries)).unwrap();
    (
        HostBindingContext {
            snapshot: source,
            topology,
            control_state,
        },
        persisted,
    )
}

#[derive(Clone)]
struct MemoryAdapter {
    context: HostBindingContext,
    payload: Option<String>,
}

impl BindingHostAdapter for MemoryAdapter {
    type Error = &'static str;

    fn current_binding_context(
        &self,
        _target: &HostTargetRef,
    ) -> Result<HostBindingContext, Self::Error> {
        Ok(HostBindingContext {
            snapshot: self.context.snapshot.clone(),
            topology: self.context.topology.clone(),
            control_state: self.context.control_state.clone(),
        })
    }

    fn read_binding_payload(
        &self,
        _target: &HostTargetRef,
    ) -> Result<Option<String>, Self::Error> {
        Ok(self.payload.clone())
    }

    fn write_binding_payload(
        &mut self,
        _target: &HostTargetRef,
        payload: String,
    ) -> Result<(), Self::Error> {
        self.payload = Some(payload);
        Ok(())
    }
}

#[test]
fn binding_json_round_trip_revalidates_against_current_certified_geometry() {
    let (context, persisted) = fixture();
    let json = persisted.to_json().unwrap();
    let decoded = PersistedBinding::from_json(&json).unwrap();

    let validated = decoded
        .validate_against(
            &context.snapshot,
            &context.topology,
            &context.control_state,
        )
        .unwrap();

    assert_eq!(validated.binding(), persisted.binding());
    assert_eq!(decoded.to_json().unwrap(), json);
}

#[test]
fn source_revision_topology_revision_and_fingerprint_changes_are_stale() {
    let (context, persisted) = fixture();

    let changed_geometry = snapshot(
        target(1),
        2,
        1,
        context.snapshot.payload().clone(),
    );
    assert!(matches!(
        persisted.validate_against(
            &changed_geometry,
            &context.topology,
            &context.control_state
        ),
        Err(BindingDiagnostic::StaleSourceGeometryRevision)
    ));

    let changed_topology_revision = snapshot(
        target(1),
        1,
        2,
        context.snapshot.payload().clone(),
    );
    assert!(matches!(
        persisted.validate_against(
            &changed_topology_revision,
            &context.topology,
            &context.control_state
        ),
        Err(BindingDiagnostic::StaleSourceTopologyRevision)
    ));

    let changed_fingerprint = snapshot(target(1), 1, 1, source_path(0.1));
    assert!(matches!(
        persisted.validate_against(
            &changed_fingerprint,
            &context.topology,
            &context.control_state
        ),
        Err(BindingDiagnostic::StaleSourceFingerprint)
    ));
}

#[test]
fn changed_cage_topology_identity_or_revision_is_stale() {
    let (context, persisted) = fixture();
    let (changed_identity, changed_identity_state) = cage(2, 1);
    assert!(matches!(
        persisted.validate_against(
            &context.snapshot,
            &changed_identity,
            &changed_identity_state
        ),
        Err(BindingDiagnostic::StaleCageTopology)
    ));

    let (changed_revision, changed_revision_state) = cage(1, 2);
    assert!(matches!(
        persisted.validate_against(
            &context.snapshot,
            &changed_revision,
            &changed_revision_state
        ),
        Err(BindingDiagnostic::StaleCageTopology)
    ));
}

#[test]
fn invalid_current_cage_embedding_is_rejected_before_binding_use() {
    let (context, persisted) = fixture();
    let mut invalid_state = context.control_state.clone();
    invalid_state.points[3].position = ControlPointPosition::new(0.0, 0.0).unwrap();

    assert!(validate_cage_embedding(&context.topology, &invalid_state).is_err());
    assert!(matches!(
        persisted.validate_against(
            &context.snapshot,
            &context.topology,
            &invalid_state
        ),
        Err(BindingDiagnostic::InvalidCageEmbedding(_))
    ));
}

#[test]
fn adapter_persists_and_loads_only_validated_payloads_for_exact_target_generation() {
    let (context, persisted) = fixture();
    let target_ref = context.snapshot.target().clone();
    let mut adapter = MemoryAdapter {
        context,
        payload: None,
    };

    persist_binding(&mut adapter, &target_ref, &persisted).unwrap();
    let loaded = load_validated_binding(&adapter, &target_ref)
        .unwrap()
        .expect("persisted binding should load");
    assert_eq!(loaded.binding(), persisted.binding());

    let wrong_generation = target(2);
    assert!(matches!(
        persist_binding(&mut adapter, &wrong_generation, &persisted),
        Err(PersistBindingError::Diagnostic(BindingDiagnostic::StaleTarget))
    ));
    assert!(matches!(
        load_validated_binding(&adapter, &wrong_generation),
        Err(LoadBindingError::Diagnostic(BindingDiagnostic::StaleTarget))
    ));
}


#[test]
fn adaptive_curve_samples_map_through_certified_cage_without_replacing_path2d() {
    let (context, _) = fixture();
    let source_path = context.snapshot.payload().clone();
    let samples = sample_path_curves(&source_path, CurveSamplingPolicy::default()).unwrap();

    assert_eq!(samples.segments.len(), 1);
    assert_eq!(samples.segments[0].command_index, 2);
    assert_eq!(samples.segments[0].samples.first().unwrap().t, 0.0);
    assert_eq!(samples.segments[0].samples.last().unwrap().t, 1.0);

    let certificate =
        validate_cage_embedding(&context.topology, &context.control_state).unwrap();
    let geometry =
        ValidatedCageGeometry::new(&context.topology, &context.control_state, &certificate)
            .unwrap();
    let mapped = map_curve_samples(
        &samples,
        &geometry,
        OutsideCagePolicy::UnboundZeroInfluence,
    )
    .unwrap();

    assert_eq!(mapped.policy_version, samples.policy_version);
    assert_eq!(mapped.samples.len(), samples.total_samples);
    assert!(mapped
        .samples
        .iter()
        .all(|sample| matches!(sample.mapping, CurveSampleMapping::Mapped { .. })));
    assert_eq!(context.snapshot.payload(), &source_path);

    let mut invalid_samples = samples.clone();
    invalid_samples.policy_version += 1;
    assert!(matches!(
        map_curve_samples(
            &invalid_samples,
            &geometry,
            OutsideCagePolicy::UnboundZeroInfluence
        ),
        Err(CurveMappingError::InvalidSampleSet)
    ));
}
