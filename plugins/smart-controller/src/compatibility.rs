//! Thin compatibility wrappers around the core's tested V1 migrations.
//!
//! This layer performs format decoding and delegates all semantic conversion
//! to smart-controller-core. It does not discard migration notices or losses.

use smart_controller_core::{
    CoreError,
    migration::{CanonicalDocument, MigrationOutput, migrate_binding_set_v1, migrate_unversioned_params},
    legacy::BindingSet,
    Params,
};

pub fn migrate_binding_set_json(source: &str) -> Result<MigrationOutput, CoreError> {
    let binding_set: BindingSet = serde_json::from_str(source)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    migrate_binding_set_v1(&binding_set)
}

pub fn migrate_params_json(source: &str) -> Result<MigrationOutput, CoreError> {
    let params: Params = serde_json::from_str(source)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    migrate_unversioned_params(&params)
}

pub fn decode_canonical_json(source: &str) -> Result<CanonicalDocument, CoreError> {
    CanonicalDocument::from_json(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_legacy_inputs_return_serialization_errors() {
        assert!(migrate_binding_set_json("{") .is_err());
        assert!(migrate_params_json("{") .is_err());
    }

    #[test]
    fn canonical_decode_rejects_unknown_fields() {
        let source = r#"{"schema":{"name":"smart-controller","version":1},"inputs":[],"controllers":[],"unexpected":true}"#;
        assert!(decode_canonical_json(source).is_err());
    }

}
