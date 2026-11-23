//! Versioned persistence envelope and schema identity.
use crate::error::CoreError;
use serde::{Deserialize, Serialize};

pub const SCHEMA_NAME: &str = "smart-controller";
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaIdentity {
    pub name: String,
    pub version: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaEnvelope<T> {
    pub schema: SchemaIdentity,
    pub controller: T,
}

impl<T> SchemaEnvelope<T> {
    pub fn new(controller: T) -> Self {
        Self {
            schema: SchemaIdentity {
                name: SCHEMA_NAME.into(),
                version: CURRENT_SCHEMA_VERSION,
            },
            controller,
        }
    }

    pub fn validate_identity(&self) -> Result<(), CoreError> {
        if self.schema.name != SCHEMA_NAME {
            return Err(CoreError::Serialization(format!(
                "unexpected schema name: {}",
                self.schema.name
            )));
        }
        if self.schema.version != CURRENT_SCHEMA_VERSION {
            return Err(CoreError::UnsupportedSchemaVersion(self.schema.version));
        }
        Ok(())
    }
}
