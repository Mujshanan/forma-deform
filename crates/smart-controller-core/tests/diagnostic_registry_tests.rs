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
