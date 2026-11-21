use smart_controller_core::{
    legacy::{BindingSet, ControllerBinding as LegacyBinding, ControllerMode, ControllerTarget, EaseCurve, InputRange, Vec2},
    migration::{migrate_binding_set_v1, migrate_unversioned_params},
    model::{EndpointPolicy, Mode, Params},
    canonical::{BindingMode, EasingConfig, MappingConfig, TargetMapping},
};

fn legacy_geometry() -> LegacyBinding {
    LegacyBinding {
        id: "bind-arm".into(),
        controller_id: "bone-arm".into(),
        input_range: InputRange { minimum: -45.0, neutral: 0.0, maximum: 45.0 },
        easing: EaseCurve::Smoothstep,
        mode: ControllerMode::RelativeDelta,
        target: ControllerTarget::GeometryDeform {
            element_id: "arm-shape".into(), pin_id: "wrist".into(),
            negative_offset: Vec2 { x: -4.0, y: 2.0 },
            neutral_offset: Vec2 { x: 1.0, y: 1.0 },
            positive_offset: Vec2 { x: 8.0, y: 5.0 },
        },
        enabled: true,
    }
}

#[test]
fn binding_set_migration_preserves_geometry_anchors_range_easing_and_mode() {
    let set = BindingSet::new(vec![legacy_geometry()]).unwrap();
    let migrated = migrate_binding_set_v1(&set).unwrap();
    assert!(migrated.report.is_lossless());
    assert_eq!(migrated.report.source_format, "binding_set_v1");
    assert_eq!(migrated.document.controllers.len(), 1);
    let controller = &migrated.document.controllers[0];
    assert_eq!(controller.enabled, true);
    assert_eq!(controller.mapping, MappingConfig::ThreeAnchorRange { minimum: -45.0, neutral: 0.0, maximum: 45.0, easing: EasingConfig::Smoothstep });
    let binding = &controller.bindings[0];
    assert_eq!(binding.mode, BindingMode::RelativeDelta);
    assert_eq!(binding.base_value.as_ref().unwrap().as_vector2().unwrap().x(), 1.0);
    assert_eq!(binding.target, TargetMapping::GeometryDeform {
        element_id: "arm-shape".into(), pin_id: "wrist".into(),
        negative_offset: [-4.0, 2.0], neutral_offset: [1.0, 1.0], positive_offset: [8.0, 5.0],
    });
    migrated.document.validate().unwrap();
}

#[test]
fn binding_set_migration_preserves_symbol_frames_and_disabled_state() {
    let mut binding = legacy_geometry();
    binding.id = "bind-symbol".into();
    binding.controller_id = "bone-head".into();
    binding.mode = ControllerMode::AbsolutePose;
    binding.enabled = false;
    binding.target = ControllerTarget::SymbolFrame {
        symbol_instance_id: "head-instance".into(),
        negative_frame: 2, neutral_frame: 7, positive_frame: 12,
    };
    let set = BindingSet::new(vec![binding]).unwrap();
    let migrated = migrate_binding_set_v1(&set).unwrap();
    let controller = &migrated.document.controllers[0];
    assert!(!controller.enabled);
    assert_eq!(controller.bindings[0].mode, BindingMode::AbsolutePose);
    assert_eq!(controller.bindings[0].base_value, None);
    assert_eq!(controller.bindings[0].target, TargetMapping::SymbolFrame {
        symbol_instance_id: "head-instance".into(), negative_frame: 2, neutral_frame: 7, positive_frame: 12,
    });
}

#[test]
fn binding_set_migration_deduplicates_shared_input_definitions() {
    let first = legacy_geometry();
    let mut second = legacy_geometry();
    second.id = "bind-arm-secondary".into();
    let set = BindingSet::new(vec![first, second]).unwrap();
    let migrated = migrate_binding_set_v1(&set).unwrap();
    assert_eq!(migrated.document.controllers.len(), 2);
    assert_eq!(migrated.document.inputs.len(), 1);
}

fn slider_params() -> Params {
    Params {
        mode: Mode::Slider,
        driver_layer_id: None, driver_bone_name: None,
        action_layer_id: Some(5),
        min_rotation_deg: Some(-30.0), max_rotation_deg: Some(30.0),
        action_start_frame: Some(1), action_end_frame: Some(12),
        endpoint_policy: Some(EndpointPolicy::Inclusive),
        target_bone_name: None, property: None, scale: None, additive: false,
        manual_value: Some(0.0),
    }
}

#[test]
fn unversioned_slider_params_migration_preserves_frame_mapping() {
    let migrated = migrate_unversioned_params(&slider_params()).unwrap();
    assert_eq!(migrated.report.source_format, "unversioned_params");
    assert!(migrated.report.is_lossless());
    assert_eq!(migrated.document.controllers[0].mapping, MappingConfig::DirectedRange1D {
        start: -30.0, end: 30.0,
    });
    assert_eq!(migrated.document.controllers[0].bindings[0].target, TargetMapping::FrameRange {
        action_layer_id: 5, start_frame: 1, end_frame: 12, endpoint_policy: EndpointPolicy::Inclusive,
    });
    assert!(matches!(&migrated.document.inputs[0].source, smart_controller_core::input::InputSource::Manual));
    migrated.document.validate().unwrap();
}

#[test]
fn unversioned_transform_params_migration_preserves_property_mapping() {
    let mut params = slider_params();
    params.mode = Mode::Transform;
    params.target_bone_name = Some("forearm".into());
    params.property = Some("rotation".into());
    params.scale = Some(1.5);
    params.additive = true;
    let migrated = migrate_unversioned_params(&params).unwrap();
    let binding = &migrated.document.controllers[0].bindings[0];
    assert_eq!(binding.mode, BindingMode::RelativeDelta);
    assert_eq!(binding.base_value.unwrap().as_scalar(), Some(0.0));
    assert_eq!(binding.target, TargetMapping::Property {
        target_id: "forearm".into(), property: "rotation".into(), scale: 1.5, additive: true,
    });
}

#[test]
fn params_migration_reports_fields_that_cannot_apply_to_the_selected_mode() {
    let mut params = slider_params();
    params.mode = Mode::Transform;
    params.target_bone_name = Some("forearm".into());
    params.property = Some("rotation".into());
    params.scale = Some(1.0);
    let migrated = migrate_unversioned_params(&params).unwrap();
    assert!(!migrated.report.is_lossless());
    assert!(migrated.report.losses.iter().any(|loss| loss.contains("Slider-only")));
}
