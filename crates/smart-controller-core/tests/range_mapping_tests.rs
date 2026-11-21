use smart_controller_core::{mapping::{easing::Easing, frame, range::{normalize_directed, ThreePointRange}}, model::EndpointPolicy};

#[test]
fn three_point_range_preserves_negative_neutral_positive() {
    let range = ThreePointRange::new(-10.0, 0.0, 30.0).unwrap();
    assert_eq!(range.normalize_signed(-10.0).unwrap(), -1.0);
    assert_eq!(range.normalize_signed(0.0).unwrap(), 0.0);
    assert_eq!(range.normalize_signed(30.0).unwrap(), 1.0);
    assert_eq!(range.normalize_signed(100.0).unwrap(), 1.0);
}

#[test]
fn directed_range_supports_reversed_endpoints() {
    assert_eq!(normalize_directed(5.0, 10.0, 0.0).unwrap(), 0.5);
}

#[test]
fn frame_mapping_preserves_endpoint_policies() {
    assert_eq!(frame::map_to_frame(1.0, 1, 9, EndpointPolicy::Exclusive).unwrap(), 8);
    assert_eq!(frame::map_to_frame(1.0, 1, 9, EndpointPolicy::Inclusive).unwrap(), 9);
}

#[test]
fn easing_preserves_linear_and_smoothstep_endpoints() {
    assert_eq!(Easing::Linear.apply(0.5).unwrap(), 0.5);
    assert_eq!(Easing::Smoothstep.apply(0.0).unwrap(), 0.0);
    assert_eq!(Easing::Smoothstep.apply(1.0).unwrap(), 1.0);
}
