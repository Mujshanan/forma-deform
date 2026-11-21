use smart_controller_core::{
    binding::{
        BindingSet as CanonicalBindingSet,
        ControllerBinding as CanonicalBinding,
        ControllerMode as CanonicalMode,
        ControllerTarget as CanonicalTarget,
        EaseCurve as CanonicalEase,
        InputRange as CanonicalRange,
        PreviewResult as CanonicalPreview,
        Vec2 as CanonicalVec2,
    },
    legacy::{
        BindingSet as LegacyBindingSet,
        ControllerBinding as LegacyBinding,
        Vec2 as LegacyVec2,
    },
    BindingSet as RootBindingSet,
    ControllerBinding as RootBinding,
    Vec2 as RootVec2,
};

fn sample_binding() -> CanonicalBinding {
    CanonicalBinding {
        id: "binding-test".into(),
        controller_id: "bone-test".into(),
        input_range: CanonicalRange { minimum: -30.0, neutral: 0.0, maximum: 60.0 },
        easing: CanonicalEase::Smoothstep,
        mode: CanonicalMode::AbsolutePose,
        target: CanonicalTarget::GeometryDeform {
            element_id: "element-test".into(),
            pin_id: "pin-test".into(),
            negative_offset: CanonicalVec2::new(-3.0, 1.0),
            neutral_offset: CanonicalVec2::new(0.0, 0.0),
            positive_offset: CanonicalVec2::new(6.0, -2.0),
        },
        enabled: true,
    }
}

#[test]
fn canonical_legacy_and_root_paths_are_the_same_public_types() {
    let canonical: CanonicalBinding = sample_binding();
    let legacy: LegacyBinding = canonical.clone();
    let root: RootBinding = legacy.clone();

    let _: CanonicalBinding = legacy;
    let _: LegacyBinding = root;

    let canonical_vec = CanonicalVec2::new(2.0, 3.0);
    let legacy_vec: LegacyVec2 = canonical_vec;
    let root_vec: RootVec2 = legacy_vec;
    assert_eq!(root_vec, CanonicalVec2::new(2.0, 3.0));

    let canonical_set = CanonicalBindingSet::new(vec![canonical]).unwrap();
    let legacy_set: LegacyBindingSet = canonical_set.clone();
    let root_set: RootBindingSet = legacy_set.clone();
    assert_eq!(root_set, canonical_set);
}

#[test]
fn canonical_binding_path_preserves_version_one_json_and_evaluation() {
    let set = CanonicalBindingSet::new(vec![sample_binding()]).unwrap();
    let json = set.to_json().unwrap();
    assert!(json.contains("\"schema_version\": 1"));
    assert!(json.contains("\"element_id\": \"element-test\""));

    let decoded = CanonicalBindingSet::from_json(&json).unwrap();
    assert_eq!(decoded, set);

    let result = decoded.bindings[0].evaluate(30.0).unwrap();
    assert_eq!(
        result,
        Some(CanonicalPreview::GeometryDeform {
            element_id: "element-test".into(),
            pin_id: "pin-test".into(),
            offset: CanonicalVec2::new(3.0, -1.0),
        })
    );
}
