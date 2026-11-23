//! Durable binding payloads, explicit freshness diagnostics, and the host adapter boundary.
//!
//! FDM owns the binding schema and validates its contents. The host owns document
//! storage, transactions, dirty state, and save/load orchestration.

use serde::{Deserialize, Serialize};

use crate::binding::{BindingError, BindingRecord, ValidatedCageGeometry};
use crate::embedding::{validate_cage_embedding, EmbeddingError};
use crate::host_contract::HostTargetRef;
use crate::topology::{DeformationControlState, DeformationTopology};

pub const BINDING_DOCUMENT_SCHEMA: &str = "forma-deform-binding";
pub const BINDING_DOCUMENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_BINDING_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_BINDING_ENTRIES: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingEnvelope {
    schema_name: String,
    schema_version: u32,
    binding: BindingRecord,
}

/// A binding payload ready for host-owned persistence.
///
/// This is deliberately separate from FdmDocument: AniCraft decides where the
/// payload lives in its document model and when it is committed.
#[derive(Clone, Debug, PartialEq)]
pub struct PersistedBinding {
    binding: BindingRecord,
}

impl PersistedBinding {
    pub fn new(binding: BindingRecord) -> Result<Self, BindingPersistenceError> {
        validate_payload_shape(&binding)?;
        Ok(Self { binding })
    }

    pub fn binding(&self) -> &BindingRecord {
        &self.binding
    }

    pub fn into_binding(self) -> BindingRecord {
        self.binding
    }

    pub fn to_json(&self) -> Result<String, BindingPersistenceError> {
        validate_payload_shape(&self.binding)?;
        let envelope = BindingEnvelope {
            schema_name: BINDING_DOCUMENT_SCHEMA.to_owned(),
            schema_version: BINDING_DOCUMENT_SCHEMA_VERSION,
            binding: self.binding.clone(),
        };
        let json = serde_json::to_string(&envelope)
            .map_err(|_| BindingPersistenceError::Serialization)?;
        if json.len() > MAX_BINDING_DOCUMENT_BYTES {
            return Err(BindingPersistenceError::ResourceLimitExceeded);
        }
        Ok(json)
    }

    /// Deserialization establishes structural validity only. Call
    /// `validate_against` before treating the binding as usable.
    pub fn from_json(input: &str) -> Result<Self, BindingPersistenceError> {
        if input.len() > MAX_BINDING_DOCUMENT_BYTES {
            return Err(BindingPersistenceError::ResourceLimitExceeded);
        }
        let envelope: BindingEnvelope = serde_json::from_str(input)
            .map_err(|_| BindingPersistenceError::MalformedPayload)?;
        if envelope.schema_name != BINDING_DOCUMENT_SCHEMA {
            return Err(BindingPersistenceError::UnsupportedSchemaName);
        }
        if envelope.schema_version != BINDING_DOCUMENT_SCHEMA_VERSION {
            return Err(BindingPersistenceError::UnsupportedSchemaVersion);
        }
        validate_payload_shape(&envelope.binding)?;
        Ok(Self { binding: envelope.binding })
    }

    pub fn validate_against(
        &self,
        snapshot: &crate::host_contract::GeometrySnapshot,
        topology: &DeformationTopology,
        control_state: &DeformationControlState,
    ) -> Result<ValidatedBindingRecord, BindingDiagnostic> {
        snapshot.validate_envelope()
            .map_err(|_| BindingDiagnostic::InvalidCurrentSnapshot)?;
        let certificate = validate_cage_embedding(topology, control_state)
            .map_err(BindingDiagnostic::InvalidCageEmbedding)?;
        let geometry = ValidatedCageGeometry::new(topology, control_state, &certificate)
            .map_err(BindingDiagnostic::InvalidBinding)?;
        self.binding.validate_against(snapshot, topology, &geometry)
            .map_err(BindingDiagnostic::from)?;
        Ok(ValidatedBindingRecord { binding: self.binding.clone() })
    }
}

/// Capability wrapper: only produced after the binding has passed validation
/// against the exact current source snapshot and certified cage geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedBindingRecord {
    binding: BindingRecord,
}

impl ValidatedBindingRecord {
    pub fn binding(&self) -> &BindingRecord {
        &self.binding
    }
}

fn validate_payload_shape(binding: &BindingRecord) -> Result<(), BindingPersistenceError> {
    binding.stamp().validate().map_err(BindingPersistenceError::InvalidStamp)?;
    if binding.entries().len() > MAX_BINDING_ENTRIES {
        return Err(BindingPersistenceError::ResourceLimitExceeded);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub enum BindingPersistenceError {
    MalformedPayload,
    UnsupportedSchemaName,
    UnsupportedSchemaVersion,
    InvalidStamp(BindingError),
    Serialization,
    ResourceLimitExceeded,
}

impl std::fmt::Display for BindingPersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "binding persistence failed: {self:?}")
    }
}
impl std::error::Error for BindingPersistenceError {}

#[derive(Clone, Debug, PartialEq)]
pub enum BindingDiagnostic {
    StaleTarget,
    StaleSourceGeometryRevision,
    StaleSourceTopologyRevision,
    StaleSourceFingerprint,
    StaleCoordinateSpace,
    StalePayloadSchema,
    StaleCageTopology,
    InvalidCurrentSnapshot,
    InvalidCageEmbedding(EmbeddingError),
    InvalidBinding(BindingError),
}

impl From<BindingError> for BindingDiagnostic {
    fn from(error: BindingError) -> Self {
        match error {
            BindingError::StaleTarget => Self::StaleTarget,
            BindingError::StaleGeometryRevision => Self::StaleSourceGeometryRevision,
            BindingError::StaleSourceTopologyRevision => Self::StaleSourceTopologyRevision,
            BindingError::StaleSourceFingerprint => Self::StaleSourceFingerprint,
            BindingError::StaleCoordinateSpace => Self::StaleCoordinateSpace,
            BindingError::StalePayloadSchema => Self::StalePayloadSchema,
            BindingError::StaleCageTopology => Self::StaleCageTopology,
            other => Self::InvalidBinding(other),
        }
    }
}

impl std::fmt::Display for BindingDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StaleTarget => write!(f, "binding target identity or generation changed"),
            Self::StaleSourceGeometryRevision => write!(f, "source geometry revision changed"),
            Self::StaleSourceTopologyRevision => write!(f, "source topology revision changed"),
            Self::StaleSourceFingerprint => write!(f, "source geometry fingerprint changed"),
            Self::StaleCoordinateSpace => write!(f, "source coordinate-space descriptor changed"),
            Self::StalePayloadSchema => write!(f, "source payload kind or schema changed"),
            Self::StaleCageTopology => write!(f, "deformation cage topology identity or revision changed"),
            Self::InvalidCurrentSnapshot => write!(f, "host supplied an invalid current geometry snapshot"),
            Self::InvalidCageEmbedding(error) => write!(f, "current cage embedding is invalid: {error}"),
            Self::InvalidBinding(error) => write!(f, "persisted binding failed validation: {error}"),
        }
    }
}
impl std::error::Error for BindingDiagnostic {}

/// Current host-owned data needed to validate a persisted binding. The host
/// supplies immutable snapshots and authored cage state; FDM recomputes the
/// global embedding certificate rather than trusting a serialized boolean.
pub struct HostBindingContext {
    pub snapshot: crate::host_contract::GeometrySnapshot,
    pub topology: DeformationTopology,
    pub control_state: DeformationControlState,
}

/// Minimal host integration boundary. The adapter stores opaque FDM JSON in
/// the host's authoritative document; it must make writes atomic with its
/// document transaction and must not persist runtime caches or certificates.
pub trait BindingHostAdapter {
    type Error;

    fn current_binding_context(
        &self,
        target: &HostTargetRef,
    ) -> Result<HostBindingContext, Self::Error>;

    fn read_binding_payload(
        &self,
        target: &HostTargetRef,
    ) -> Result<Option<String>, Self::Error>;

    fn write_binding_payload(
        &mut self,
        target: &HostTargetRef,
        payload: String,
    ) -> Result<(), Self::Error>;
}

/// Load, deserialize, and validate. No stale record is silently rebound.
pub fn load_validated_binding<A: BindingHostAdapter>(
    adapter: &A,
    target: &HostTargetRef,
) -> Result<Option<ValidatedBindingRecord>, LoadBindingError<A::Error>> {
    let Some(payload) = adapter.read_binding_payload(target).map_err(LoadBindingError::Host)? else {
        return Ok(None);
    };
    let persisted = PersistedBinding::from_json(&payload)
        .map_err(LoadBindingError::Persistence)?;
    let context = adapter.current_binding_context(target).map_err(LoadBindingError::Host)?;
    persisted.validate_against(&context.snapshot, &context.topology, &context.control_state)
        .map(Some)
        .map_err(LoadBindingError::Diagnostic)
}

#[derive(Debug)]
pub enum LoadBindingError<E> {
    Host(E),
    Persistence(BindingPersistenceError),
    Diagnostic(BindingDiagnostic),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_payload_before_deserialization() {
        let input = " ".repeat(MAX_BINDING_DOCUMENT_BYTES + 1);
        assert_eq!(
            PersistedBinding::from_json(&input),
            Err(BindingPersistenceError::ResourceLimitExceeded)
        );
    }

    #[test]
    fn rejects_unknown_schema_and_malformed_envelopes() {
        let unknown = r#"{"schema_name":"forma-deform-binding","schema_version":99,"binding":{}}"#;
        assert_eq!(
            PersistedBinding::from_json(unknown),
            Err(BindingPersistenceError::MalformedPayload)
        );
        let malformed = r#"{"schema_name":"forma-deform-binding","schema_version":1,"binding":{}}"#;
        assert_eq!(
            PersistedBinding::from_json(malformed),
            Err(BindingPersistenceError::MalformedPayload)
        );
    }
}
