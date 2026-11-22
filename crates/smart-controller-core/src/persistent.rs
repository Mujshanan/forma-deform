//! Versioned, host-independent payload for bindings persisted by AniCraft.
//!
//! AniCraft owns storage and document transactions. This module owns the payload
//! schema, validation, and atomic collection edits; it never performs I/O.
use crate::{
    canonical::Controller,
    migration::CanonicalDocument,
    error::CoreError,
    ids::ControllerId,
    input::InputDefinition,
    schema::{SchemaIdentity, SCHEMA_NAME, CURRENT_SCHEMA_VERSION},
};
use serde::{Deserialize, Serialize};

pub const PERSISTED_BINDINGS_SCHEMA_NAME: &str = "forma-deform-bindings";
pub const CURRENT_PERSISTED_BINDINGS_VERSION: u32 = 1;

/// The payload AniCraft stores inside its owning document extension field.
///
/// `revision` is the collection's logical revision, distinct from AniCraft's
/// document revision. AniCraft remains authoritative for undo/redo and dirty state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistentBindingDocument {
    pub schema: SchemaIdentity,
    pub revision: u64,
    pub controller_document: CanonicalDocument,
}

impl Default for PersistentBindingDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistentBindingDocument {
    pub fn new() -> Self {
        Self {
            schema: SchemaIdentity {
                name: PERSISTED_BINDINGS_SCHEMA_NAME.into(),
                version: CURRENT_PERSISTED_BINDINGS_VERSION,
            },
            revision: 0,
            controller_document: CanonicalDocument {
                schema: SchemaIdentity {
                    name: SCHEMA_NAME.into(),
                    version: CURRENT_SCHEMA_VERSION,
                },
                inputs: Vec::new(),
                controllers: Vec::new(),
            },
        }
    }

    /// Wrap a validated canonical document for its first persistent save.
    pub fn from_canonical_document(document: CanonicalDocument) -> Result<Self, CoreError> {
        document.validate()?;
        Ok(Self {
            controller_document: document,
            ..Self::new()
        })
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema.name != PERSISTED_BINDINGS_SCHEMA_NAME {
            return Err(CoreError::Serialization(format!(
                "unexpected persistent binding schema: {}",
                self.schema.name
            )));
        }
        if self.schema.version != CURRENT_PERSISTED_BINDINGS_VERSION {
            return Err(CoreError::UnsupportedSchemaVersion(self.schema.version));
        }
        self.controller_document.validate()
    }

    pub fn to_json(&self) -> Result<String, CoreError> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map_err(|error| CoreError::Serialization(error.to_string()))
    }

    pub fn from_json(source: &str) -> Result<Self, CoreError> {
        let mut value: serde_json::Value = serde_json::from_str(source)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let canonical_value = value.get("controller_document")
            .ok_or_else(|| CoreError::Serialization("persistent payload is missing controller_document".into()))?;
        let canonical = CanonicalDocument::from_json(&canonical_value.to_string())?;
        value.as_object_mut().expect("JSON object checked by deserialization")
            .insert("controller_document".into(), serde_json::to_value(canonical)
                .map_err(|error| CoreError::Serialization(error.to_string()))?);
        let document: Self = serde_json::from_value(value)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        document.validate()?;
        Ok(document)
    }

    pub fn controller(&self, id: &ControllerId) -> Option<&Controller> {
        self.controller_document.controllers.iter().find(|item| &item.id == id)
    }

    /// Insert or replace one controller and its input definitions atomically.
    ///
    /// Input IDs may be shared only when their definitions are identical. Inputs
    /// no longer referenced after a replacement are pruned. The current value is
    /// unchanged if validation fails.
    pub fn upsert_controller(
        &mut self,
        inputs: Vec<InputDefinition>,
        controller: Controller,
    ) -> Result<(), CoreError> {
        let mut supplied_ids = std::collections::HashSet::new();
        for input in &inputs {
            if !supplied_ids.insert(input.id.clone()) {
                return Err(CoreError::InvalidModel(format!("upsert contains duplicate input ID {}", input.id)));
            }
            if !controller.inputs.contains(&input.id) {
                return Err(CoreError::InvalidModel(
                    "upsert includes an input not referenced by its controller".into(),
                ));
            }
        }
        let mut candidate = self.controller_document.clone();
        candidate.controllers.retain(|item| item.id != controller.id);
        let referenced_by_others: std::collections::HashSet<_> = candidate.controllers
            .iter()
            .flat_map(|item| item.inputs.iter().cloned())
            .collect();
        let supplied_ids: std::collections::HashSet<_> = inputs.iter()
            .map(|input| input.id.clone())
            .collect();
        let retained_for_replacement: std::collections::HashSet<_> = controller.inputs.iter()
            .filter(|id| !supplied_ids.contains(*id))
            .cloned()
            .collect();
        candidate.inputs.retain(|input| {
            referenced_by_others.contains(&input.id) || retained_for_replacement.contains(&input.id)
        });

        for input in inputs {
            if let Some(existing) = candidate.inputs.iter().find(|item| item.id == input.id) {
                if existing != &input {
                    return Err(CoreError::InvalidModel(format!(
                        "input ID {} already exists with a different definition",
                        input.id
                    )));
                }
            } else {
                candidate.inputs.push(input);
            }
        }
        candidate.controllers.push(controller);
        candidate.validate()?;
        if candidate != self.controller_document {
            let next_revision = self.revision.checked_add(1)
                .ok_or_else(|| CoreError::InvalidModel("persistent binding revision overflow".into()))?;
            self.controller_document = candidate;
            self.revision = next_revision;
        }
        Ok(())
    }

    /// Remove a controller atomically. A controller still consumed by another
    /// controller cannot be removed until its dependents are updated or removed.
    pub fn remove_controller(&mut self, id: &ControllerId) -> Result<(), CoreError> {
        if !self.controller_document.controllers.iter().any(|item| &item.id == id) {
            return Err(CoreError::InvalidModel(format!("controller {id} does not exist")));
        }
        let mut candidate = self.controller_document.clone();
        candidate.controllers.retain(|item| &item.id != id);
        let referenced: std::collections::HashSet<_> = candidate.controllers
            .iter()
            .flat_map(|item| item.inputs.iter().cloned())
            .collect();
        candidate.inputs.retain(|input| referenced.contains(&input.id));
        candidate.validate()?;
        let next_revision = self.revision.checked_add(1)
            .ok_or_else(|| CoreError::InvalidModel("persistent binding revision overflow".into()))?;
        self.controller_document = candidate;
        self.revision = next_revision;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        canonical::{BindingMode, ControllerBinding, MappingConfig, TargetMapping},
        ids::{BindingId, InputId, TargetId},
        input::{FallbackPolicy, InputDimension, InputSource},
    };

    fn sample_record() -> (Vec<InputDefinition>, Controller) {
        let input_id = InputId::new("jaw-angle").unwrap();
        let input = InputDefinition {
            id: input_id.clone(),
            source: InputSource::Manual,
            dimension: InputDimension::Scalar,
            fallback: FallbackPolicy::Reject,
        };
        let controller = Controller {
            id: ControllerId::new("jaw-controller").unwrap(),
            enabled: true,
            inputs: vec![input_id],
            dependencies: vec![],
            mapping: MappingConfig::Range1D { minimum: -1.0, neutral: 0.0, maximum: 1.0 },
            bindings: vec![ControllerBinding {
                id: BindingId::new("jaw-width").unwrap(),
                target_id: TargetId::new("property:jaw:width").unwrap(),
                target: TargetMapping::Property {
                    target_id: "jaw".into(),
                    property: "width".into(),
                    scale: 1.0,
                    additive: false,
                },
                mode: BindingMode::AbsolutePose,
                base_value: None,
                composition: None,
            }],
            parent_scoped: false,
        };
        (vec![input], controller)
    }

    #[test]
    fn versioned_payload_round_trips_and_preserves_revision() {
        let (inputs, controller) = sample_record();
        let mut payload = PersistentBindingDocument::new();
        payload.upsert_controller(inputs, controller).unwrap();
        let json = payload.to_json().unwrap();
        let decoded = PersistentBindingDocument::from_json(&json).unwrap();
        assert_eq!(decoded, payload);
        assert_eq!(decoded.revision, 1);
    }

    #[test]
    fn invalid_upsert_is_atomic_and_does_not_advance_revision() {
        let (inputs, mut controller) = sample_record();
        let mut payload = PersistentBindingDocument::new();
        controller.mapping = MappingConfig::Range1D { minimum: 0.0, neutral: 0.0, maximum: 1.0 };
        assert!(payload.upsert_controller(inputs, controller).is_err());
        assert_eq!(payload.revision, 0);
        assert!(payload.controller_document.controllers.is_empty());
    }

    #[test]
    fn replacing_controller_preserves_its_referenced_input_definitions() {
        let (inputs, controller) = sample_record();
        let mut payload = PersistentBindingDocument::new();
        payload.upsert_controller(inputs, controller.clone()).unwrap();
        let mut replacement = controller;
        replacement.enabled = false;
        payload.upsert_controller(Vec::new(), replacement).unwrap();
        assert_eq!(payload.revision, 2);
        assert_eq!(payload.controller_document.inputs.len(), 1);
    }

    #[test]
    fn remove_controller_prunes_inputs_and_increments_revision() {
        let (inputs, controller) = sample_record();
        let controller_id = controller.id.clone();
        let mut payload = PersistentBindingDocument::new();
        payload.upsert_controller(inputs, controller).unwrap();
        payload.remove_controller(&controller_id).unwrap();
        assert_eq!(payload.revision, 2);
        assert!(payload.controller_document.inputs.is_empty());
        assert!(payload.controller_document.controllers.is_empty());
    }

    #[test]
    fn unsupported_storage_schema_versions_fail_closed() {
        let payload = PersistentBindingDocument::new();
        let mut value: serde_json::Value = serde_json::from_str(&payload.to_json().unwrap()).unwrap();
        value["schema"]["version"] = serde_json::json!(2);
        assert!(PersistentBindingDocument::from_json(&value.to_string()).is_err());
    }

    #[test]
    fn unknown_persisted_fields_fail_closed() {
        let payload = PersistentBindingDocument::new();
        let mut value: serde_json::Value = serde_json::from_str(&payload.to_json().unwrap()).unwrap();
        value["unexpected"] = serde_json::json!(true);
        assert!(PersistentBindingDocument::from_json(&value.to_string()).is_err());
    }

    #[test]
    fn missing_controller_cannot_be_removed() {
        let mut payload = PersistentBindingDocument::new();
        let missing = ControllerId::new("missing").unwrap();
        assert!(payload.remove_controller(&missing).is_err());
        assert_eq!(payload.revision, 0);
    }
}

#[cfg(test)]
mod embedded_canonical_migration_tests {
    use super::*;

    #[test]
    fn persistent_v1_envelope_migrates_embedded_canonical_v1_only() {
        let mut payload = serde_json::to_value(PersistentBindingDocument::new()).unwrap();
        payload["controller_document"]["schema"]["version"] = serde_json::json!(1);
        let loaded = PersistentBindingDocument::from_json(&payload.to_string()).unwrap();
        assert_eq!(loaded.schema.version, CURRENT_PERSISTED_BINDINGS_VERSION);
        assert_eq!(loaded.controller_document.schema.version, CURRENT_SCHEMA_VERSION);
    }
}