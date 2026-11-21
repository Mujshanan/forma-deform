//! Thin compatibility wrappers around the core's tested V1 migrations.
//!
//! This layer performs format decoding and delegates all semantic conversion
//! to smart-controller-core. Migration notices and losses remain available.

use smart_controller_core::{
    CoreError, Params,
    legacy::BindingSet,
    migration::{
        CanonicalDocument, LegacyExportOutput, MigrationOutput, ParamsExportOutput,
        export_binding_set_v1, export_unversioned_params, migrate_binding_set_v1,
        migrate_json, migrate_unversioned_params,
    },
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

/// Detect and migrate either supported legacy JSON format through the core's
/// structural discriminator.
pub fn migrate_legacy_json(source: &str) -> Result<MigrationOutput, CoreError> {
    migrate_json(source)
}

/// Export only when the core proves that the document is losslessly
/// representable as a V1 BindingSet.
pub fn export_binding_set_v1_json(
    document: &CanonicalDocument,
) -> Result<(String, LegacyExportOutput), CoreError> {
    let output = export_binding_set_v1(document)?;
    let json = serde_json::to_string_pretty(&output.binding_set)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    Ok((json, output))
}

/// Export only when the canonical document retains the exact reversible
/// identity and structure required by the unversioned Params format.
pub fn export_params_json(
    document: &CanonicalDocument,
) -> Result<(String, ParamsExportOutput), CoreError> {
    let output = export_unversioned_params(document)?;
    let json = serde_json::to_string_pretty(&output.params)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    Ok((json, output))
}

pub fn decode_canonical_json(source: &str) -> Result<CanonicalDocument, CoreError> {
    CanonicalDocument::from_json(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_legacy_inputs_return_serialization_errors() {
        assert!(migrate_binding_set_json("{").is_err());
        assert!(migrate_params_json("{").is_err());
        assert!(migrate_legacy_json("{").is_err());
    }

    #[test]
    fn canonical_decode_rejects_unknown_fields() {
        let source = r#"{"schema":{"name":"smart-controller","version":1},"inputs":[],"controllers":[],"unexpected":true}"#;
        assert!(decode_canonical_json(source).is_err());
    }

    #[test]
    fn empty_v1_binding_set_migrates_and_exports_losslessly() {
        let legacy = BindingSet::new(vec![]).unwrap();
        let source = serde_json::to_string(&legacy).unwrap();
        let migrated = migrate_legacy_json(&source).unwrap();
        assert!(migrated.report.is_lossless());

        let (exported, report) = export_binding_set_v1_json(&migrated.document).unwrap();
        assert!(report.report.is_lossless());
        let roundtrip = BindingSet::from_json(&exported).unwrap();
        assert!(roundtrip.bindings.is_empty());
    }
}
