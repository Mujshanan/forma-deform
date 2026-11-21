use smart_controller_core::diagnostic_codes::ALL_CODES;

#[test]
fn diagnostic_registry_is_unique_and_matches_v1_fixture() {
    let json = include_str!("fixtures/diagnostic_codes_v1.json");
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    let codes: Vec<&str> = serde_json::from_value(value["codes"].clone()).unwrap();
    assert_eq!(codes, ALL_CODES);
    let unique: std::collections::HashSet<_> = ALL_CODES.iter().copied().collect();
    assert_eq!(unique.len(), ALL_CODES.len());
}

#[test]
fn structured_diagnostics_require_registered_codes_and_keep_context() {
    use smart_controller_core::{
        diagnostic_codes::{Diagnostic, DiagnosticSeverity, INPUT_STALE},
        ControllerId, InputId,
    };
    let diagnostic = Diagnostic::new(INPUT_STALE, DiagnosticSeverity::Warning, "sample exceeded its freshness limit")
        .unwrap()
        .with_controller(ControllerId::new("controller-a").unwrap())
        .with_input(InputId::new("input-a").unwrap());
    let json = serde_json::to_string(&diagnostic).unwrap();
    let decoded: Diagnostic = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, diagnostic);
    assert!(Diagnostic::new("SC_NOT_REGISTERED", DiagnosticSeverity::Error, "unknown").is_err());
}
