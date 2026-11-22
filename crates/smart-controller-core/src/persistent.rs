//! Versioned, host-independent payload for bindings persisted by AniCraft.
//!
//! AniCraft owns storage and document transactions. This module owns the payload
//! schema, validation, and atomic collection edits; it never performs I/O.
use crate::{
    canonical::{CanonicalDocument, Controller},
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
        let document: Self = serde_json::from_str(source)
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
        if inputs.iter().any(|input| !controller.inputs.contains(&input.id)) {
            return Err(CoreError::InvalidModel(
                "upsert includes an input not referenced by its controller".into(),
            ));
        }
        let mut candidate = self.controller_document.clone();
        candidate.controllers.retain(|item| item.id != controller.id);
        let referenced: std::collections::HashSet<_> = candidate.controllers
            .iter()
            .flat_map(|item| item.inputs.iter().cloned())
            .collect();
        candidate.inputs.retain(|input| referenced.contains(&input.id));

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
