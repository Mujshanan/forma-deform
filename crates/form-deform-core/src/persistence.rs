//! Versioned, serializable FDM-authored document data.
//!
//! This is a plug-in-owned payload contract, not a document store. AniCraft
//! remains responsible for embedding it in the authoritative document,
//! dirty-state tracking, save/load transactions and undo/redo.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::composition_graph::{CompositionGraph, CompositionGraphError};
use crate::cage_binding_persistence::{CageBindingPersistenceError, PersistedCageBinding};
use crate::host_contract::{
    CoordinateSpaceDescriptor, GeometrySnapshotStamp, HostTargetRef,
};

pub const FDM_DOCUMENT_SCHEMA: &str = "forma-deform-document";
pub const FDM_DOCUMENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DEFORMERS: usize = 10_000;
pub const MAX_COMPOSITION_GRAPHS: usize = 1_000;
pub const MAX_CAGE_BINDINGS: usize = crate::cage_binding_persistence::MAX_CAGE_BINDING_RECORDS;
pub const MAX_PARAMETERS_PER_DEFORMER: usize = 256;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FdmDocument {
    schema_name: String,
    schema_version: u32,
    document_id: String,
    revision: u64,
    deformers: Vec<DeformationDefinition>,
    composition_graphs: Vec<CompositionGraph>,
    #[serde(default)]
    cage_bindings: Vec<PersistedCageBinding>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeformationDefinition {
    id: String,
    revision: u64,
    name: String,
    kind: DeformationKind,
    target: HostTargetRef,
    reference_geometry: GeometrySnapshotStamp,
    coordinate_space: CoordinateSpaceDescriptor,
    input_kind: GeometryValueKind,
    output_kind: GeometryValueKind,
    parameters: BTreeMap<String, ParameterValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeformationKind {
    PinDeformation,
    Lattice,
    Cage,
    Mesh,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeometryValueKind {
    AbsoluteGeometry,
    Displacement,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParameterValue {
    Boolean(bool),
    Integer(i64),
    Number(f64),
    Text(String),
    Vector2 { x: f64, y: f64 },
}

impl DeformationDefinition {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        revision: u64,
        name: impl Into<String>,
        kind: DeformationKind,
        target: HostTargetRef,
        reference_geometry: GeometrySnapshotStamp,
        coordinate_space: CoordinateSpaceDescriptor,
        input_kind: GeometryValueKind,
        output_kind: GeometryValueKind,
        parameters: BTreeMap<String, ParameterValue>,
    ) -> Result<Self, PersistenceError> {
        let value = Self {
            id: id.into(),
            revision,
            name: name.into(),
            kind,
            target,
            reference_geometry,
            coordinate_space,
            input_kind,
            output_kind,
            parameters,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), PersistenceError> {
        require_id(&self.id)?;
        require_id(&self.name)?;
        if self.revision == 0 { return Err(PersistenceError::InvalidRevision); }
        self.target.validate().map_err(|_| PersistenceError::InvalidTarget)?;
        self.reference_geometry.validate().map_err(|_| PersistenceError::InvalidReferenceGeometry)?;
        self.coordinate_space.validate().map_err(|_| PersistenceError::InvalidCoordinateSpace)?;
        if self.reference_geometry.target() != &self.target {
            return Err(PersistenceError::ReferenceTargetMismatch);
        }
        if self.parameters.len() > MAX_PARAMETERS_PER_DEFORMER {
            return Err(PersistenceError::ResourceLimitExceeded);
        }
        for (key, value) in &self.parameters {
            require_id(key)?;
            value.validate()?;
        }
        Ok(())
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn name(&self) -> &str { &self.name }
    pub fn kind(&self) -> DeformationKind { self.kind }
    pub fn target(&self) -> &HostTargetRef { &self.target }
    pub fn reference_geometry(&self) -> &GeometrySnapshotStamp { &self.reference_geometry }
    pub fn coordinate_space(&self) -> &CoordinateSpaceDescriptor { &self.coordinate_space }
    pub fn input_kind(&self) -> GeometryValueKind { self.input_kind }
    pub fn output_kind(&self) -> GeometryValueKind { self.output_kind }
    pub fn parameters(&self) -> &BTreeMap<String, ParameterValue> { &self.parameters }
}

impl ParameterValue {
    fn validate(&self) -> Result<(), PersistenceError> {
        match self {
            Self::Number(value) if !value.is_finite() => Err(PersistenceError::NonFiniteParameter),
            Self::Vector2 { x, y } if !x.is_finite() || !y.is_finite() => Err(PersistenceError::NonFiniteParameter),
            _ => Ok(()),
        }
    }
}

impl FdmDocument {
    pub fn new(
        document_id: impl Into<String>,
        revision: u64,
        deformers: Vec<DeformationDefinition>,
        composition_graphs: Vec<CompositionGraph>,
    ) -> Result<Self, PersistenceError> {
        let value = Self {
            schema_name: FDM_DOCUMENT_SCHEMA.into(),
            schema_version: FDM_DOCUMENT_SCHEMA_VERSION,
            document_id: document_id.into(),
            revision,
            deformers,
            composition_graphs,
            cage_bindings: Vec::new(),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn new_with_cage_bindings(
        document_id: impl Into<String>,
        revision: u64,
        deformers: Vec<DeformationDefinition>,
        composition_graphs: Vec<CompositionGraph>,
        cage_bindings: Vec<PersistedCageBinding>,
    ) -> Result<Self, PersistenceError> {
        let value = Self {
            schema_name: FDM_DOCUMENT_SCHEMA.into(),
            schema_version: FDM_DOCUMENT_SCHEMA_VERSION,
            document_id: document_id.into(),
            revision,
            deformers,
            composition_graphs,
            cage_bindings,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), PersistenceError> {
        if self.schema_name != FDM_DOCUMENT_SCHEMA || self.schema_version != FDM_DOCUMENT_SCHEMA_VERSION {
            return Err(PersistenceError::UnsupportedSchemaVersion);
        }
        require_id(&self.document_id)?;
        if self.revision == 0 { return Err(PersistenceError::InvalidRevision); }
        if self.deformers.len() > MAX_DEFORMERS || self.composition_graphs.len() > MAX_COMPOSITION_GRAPHS || self.cage_bindings.len() > MAX_CAGE_BINDINGS {
            return Err(PersistenceError::ResourceLimitExceeded);
        }

        let mut deformer_ids = std::collections::BTreeSet::new();
        for definition in &self.deformers {
            definition.validate()?;
            if !deformer_ids.insert(definition.id.as_str()) {
                return Err(PersistenceError::DuplicateDeformerId);
            }
        }

        let mut binding_ids = std::collections::BTreeSet::new();
        for binding in &self.cage_bindings {
            binding.validate().map_err(PersistenceError::CageBinding)?;
            if !binding_ids.insert(binding.id()) { return Err(PersistenceError::DuplicateCageBindingId); }
        }

        let mut graph_ids = std::collections::BTreeSet::new();
        for graph in &self.composition_graphs {
            if !graph_ids.insert(graph.graph_id()) {
                return Err(PersistenceError::DuplicateCompositionGraphId);
            }
            graph.validate_against(&self.deformers).map_err(PersistenceError::CompositionGraph)?;
        }
        Ok(())
    }

    /// Serialize the validated authored model. Runtime snapshots, caches and
    /// evaluated outputs are not members of this document schema.
    pub fn to_json(&self) -> Result<String, PersistenceError> {
        self.validate()?;
        let encoded = serde_json::to_string(self).map_err(|_| PersistenceError::Serialization)?;
        if encoded.len() > MAX_DOCUMENT_BYTES {
            return Err(PersistenceError::ResourceLimitExceeded);
        }
        Ok(encoded)
    }

    /// Parse and validate an FDM document. Unknown fields and unsupported
    /// schema versions fail closed instead of being silently discarded.
    pub fn from_json(input: &str) -> Result<Self, PersistenceError> {
        if input.len() > MAX_DOCUMENT_BYTES {
            return Err(PersistenceError::ResourceLimitExceeded);
        }
        let value: Self = serde_json::from_str(input).map_err(|_| PersistenceError::MalformedDocument)?;
        value.validate()?;
        Ok(value)
    }

    /// Migration entry point. Version 1 is the first persisted schema; no
    /// historical FDM document version is silently upgraded or guessed.
    /// Future schema changes must register a named, deterministic migration.
    pub fn migrate_json(input: &str) -> Result<Self, PersistenceError> {
        if input.len() > MAX_DOCUMENT_BYTES {
            return Err(PersistenceError::ResourceLimitExceeded);
        }
        let header: serde_json::Value = serde_json::from_str(input).map_err(|_| PersistenceError::MalformedDocument)?;
        let schema = header.get("schema_name").and_then(serde_json::Value::as_str)
            .ok_or(PersistenceError::MalformedDocument)?;
        let version = header.get("schema_version").and_then(serde_json::Value::as_u64)
            .ok_or(PersistenceError::MalformedDocument)?;
        if schema != FDM_DOCUMENT_SCHEMA {
            return Err(PersistenceError::UnsupportedSchemaName);
        }
        if version != u64::from(FDM_DOCUMENT_SCHEMA_VERSION) {
            return Err(PersistenceError::UnsupportedSchemaVersion);
        }
        // Named migration registry is intentionally empty for v1: this is the
        // initial schema, not a fabricated conversion from an unknown legacy format.
        Self::from_json(input)
    }

    pub fn schema_name(&self) -> &str { &self.schema_name }
    pub fn schema_version(&self) -> u32 { self.schema_version }
    pub fn document_id(&self) -> &str { &self.document_id }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn deformers(&self) -> &[DeformationDefinition] { &self.deformers }
    pub fn composition_graphs(&self) -> &[CompositionGraph] { &self.composition_graphs }
    pub fn cage_bindings(&self) -> &[PersistedCageBinding] { &self.cage_bindings }
}

fn require_id(value: &str) -> Result<(), PersistenceError> {
    if value.trim().is_empty() { Err(PersistenceError::EmptyIdentifier) } else { Ok(()) }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistenceError {
    EmptyIdentifier,
    InvalidRevision,
    InvalidTarget,
    InvalidReferenceGeometry,
    InvalidCoordinateSpace,
    ReferenceTargetMismatch,
    NonFiniteParameter,
    DuplicateDeformerId,
    DuplicateCompositionGraphId,
    DuplicateCageBindingId,
    CageBinding(CageBindingPersistenceError),
    UnsupportedSchemaName,
    UnsupportedSchemaVersion,
    MalformedDocument,
    Serialization,
    ResourceLimitExceeded,
    CompositionGraph(CompositionGraphError),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FDM persistence validation failed: {self:?}")
    }
}
impl std::error::Error for PersistenceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition_graph::{CompositionNode, WeightedInput};
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
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        }
    }

    fn reference() -> GeometrySnapshotStamp {
        GeometrySnapshotStamp::new(
            target(), GeometryRevision(1), TopologyRevision(1),
            GeometryFingerprint { algorithm: "sha256-test".into(), digest: vec![1; 32] },
        ).unwrap()
    }

    fn definition(id: &str) -> DeformationDefinition {
        let mut parameters = BTreeMap::new();
        parameters.insert("gain".into(), ParameterValue::Number(0.75));
        DeformationDefinition::new(
            id, 1, format!("Definition {id}"), DeformationKind::PinDeformation,
            target(), reference(), space(), GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement, parameters,
        ).unwrap()
    }

    fn sample_document() -> FdmDocument {
        let definitions = vec![definition("deformer-a"), definition("deformer-b")];
        let graph = CompositionGraph::new(
            "graph-main", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("node-a", "deformer-a"),
                CompositionNode::deformer("node-b", "deformer-b"),
                CompositionNode::weighted_blend("blend", vec![
                    WeightedInput::new("node-a", 0.4).unwrap(),
                    WeightedInput::new("node-b", 0.6).unwrap(),
                ]),
            ],
            vec!["blend".into()],
        ).unwrap();
        FdmDocument::new("fdm-doc-1", 1, definitions, vec![graph]).unwrap()
    }

    #[test]
    fn authored_document_round_trips_deterministically() {
        let document = sample_document();
        let first = document.to_json().unwrap();
        let loaded = FdmDocument::from_json(&first).unwrap();
        let second = loaded.to_json().unwrap();
        assert_eq!(first, second);
        assert_eq!(loaded.document_id(), "fdm-doc-1");
        assert_eq!(loaded.deformers().len(), 2);
        assert_eq!(loaded.composition_graphs().len(), 1);
    }

    #[test]
    fn migration_entry_point_accepts_current_schema_and_rejects_unknown_versions() {
        let json = sample_document().to_json().unwrap();
        assert!(FdmDocument::migrate_json(&json).is_ok());
        let unknown = json.replace("\"schema_version\":1", "\"schema_version\":99");
        assert_eq!(FdmDocument::migrate_json(&unknown), Err(PersistenceError::UnsupportedSchemaVersion));
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let json = sample_document().to_json().unwrap();
        let changed = json.replacen("{", "{\"unrecognized\":true,", 1);
        assert_eq!(FdmDocument::from_json(&changed), Err(PersistenceError::MalformedDocument));
    }

    #[test]
    fn deserialized_host_references_are_revalidated() {
        let json = sample_document().to_json().unwrap();
        let invalid = json.replace(r#""target_id":"shape-1""#, r#""target_id":" ""#);
        assert_eq!(FdmDocument::from_json(&invalid), Err(PersistenceError::InvalidTarget));
    }

    #[test]
    fn non_finite_parameters_are_rejected() {
        assert_eq!(
            DeformationDefinition::new(
                "bad", 1, "Bad", DeformationKind::PinDeformation, target(), reference(),
                space(), GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement,
                BTreeMap::from([("gain".into(), ParameterValue::Number(f64::NAN))]),
            ),
            Err(PersistenceError::NonFiniteParameter)
        );
    }

    #[test]
    fn document_rejects_duplicate_deformer_ids() {
        assert_eq!(
            FdmDocument::new("doc", 1, vec![definition("same"), definition("same")], vec![]),
            Err(PersistenceError::DuplicateDeformerId)
        );
    }

    #[test]
    fn document_rejects_invalid_graph_reference_or_type() {
        let definitions = vec![definition("deformer-a")];
        let graph = CompositionGraph::new(
            "graph", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![CompositionNode::deformer("node", "missing")],
            vec!["node".into()],
        ).unwrap();
        assert!(matches!(
            FdmDocument::new("doc", 1, definitions, vec![graph]),
            Err(PersistenceError::CompositionGraph(CompositionGraphError::MissingDeformer))
        ));
    }

    #[test]
    fn document_rejects_invalid_schema_and_resource_limits() {
        let mut document = sample_document();
        document.schema_version = 99;
        assert_eq!(document.validate(), Err(PersistenceError::UnsupportedSchemaVersion));
        let too_large = " ".repeat(MAX_DOCUMENT_BYTES + 1);
        assert_eq!(FdmDocument::from_json(&too_large), Err(PersistenceError::ResourceLimitExceeded));
    }
    #[test]
    fn cage_bindings_round_trip_with_the_authored_document() {
        use crate::cage_binding_persistence::{PersistedBoundPoint, PersistedCageBinding};
        let binding = PersistedCageBinding::new(
            "binding-main", 1, target(), reference(), space(),
            vec![10, 20, 30], vec![[10, 20, 30]],
            vec![PersistedBoundPoint {
                point_id: 7,
                cage_vertex_ids: [10, 20, 30],
                barycentric: [0.25, 0.25, 0.5],
            }],
        ).unwrap();
        let mut document = sample_document();
        document.cage_bindings.push(binding);
        document.validate().unwrap();
        let encoded = document.to_json().unwrap();
        let loaded = FdmDocument::from_json(&encoded).unwrap();
        assert_eq!(loaded.cage_bindings().len(), 1);
        assert_eq!(loaded.to_json().unwrap(), encoded);
    }

    #[test]
    fn legacy_document_without_cage_bindings_loads_as_empty() {
        let json = sample_document().to_json().unwrap();
        let legacy = json.replace(",\"cage_bindings\":[]", "");
        let loaded = FdmDocument::from_json(&legacy).unwrap();
        assert!(loaded.cage_bindings().is_empty());
    }

    #[test]
    fn duplicate_persisted_cage_binding_ids_are_rejected() {
        use crate::cage_binding_persistence::{PersistedBoundPoint, PersistedCageBinding};
        let binding = || PersistedCageBinding::new(
            "duplicate", 1, target(), reference(), space(),
            vec![10, 20, 30], vec![[10, 20, 30]],
            vec![PersistedBoundPoint { point_id: 1, cage_vertex_ids: [10, 20, 30], barycentric: [0.2, 0.3, 0.5] }],
        ).unwrap();
        assert_eq!(
            FdmDocument::new_with_cage_bindings("doc", 1, vec![], vec![], vec![binding(), binding()]),
            Err(PersistenceError::DuplicateCageBindingId),
        );
    }

}
