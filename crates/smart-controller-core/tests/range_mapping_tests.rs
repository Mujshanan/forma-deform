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

#[test]
fn numeric_mapping_boundaries_reject_non_finite_and_overflowing_values() {
    assert!(ThreePointRange::new(-f64::MAX, f64::MAX / 2.0, f64::MAX).is_err());
    assert!(normalize_directed(0.0, -f64::MAX, f64::MAX).is_err());
    assert!(frame::map_to_frame(f64::NAN, 0, 10, EndpointPolicy::Inclusive).is_err());
    assert!(Easing::Linear.apply(f64::INFINITY).is_err());
}

#[test]
fn new_frame_mapping_matches_the_v1_frame_mapper() {
    for policy in [EndpointPolicy::Exclusive, EndpointPolicy::Inclusive] {
        for step in 0..=100 {
            let t = step as f64 / 100.0;
            assert_eq!(
                frame::map_to_frame(t, 3, 27, policy).unwrap(),
                smart_controller_core::mapping::map_to_frame(t, 3, 27, policy),
            );
        }
    }
}

#[test]
fn directed_range_mapping_matches_the_v1_rotation_mapper() {
    for (start, end) in [(-45.0, 45.0), (30.0, -20.0), (0.0, 180.0)] {
        for step in 0..=100 {
            let input = start + (end - start) * (step as f64 / 100.0);
            let actual = normalize_directed(input, start, end).unwrap();
            let expected = smart_controller_core::mapping::normalize_rotation(input, start, end).unwrap();
            assert!((actual - expected).abs() <= 1e-12);
        }
    }
}

#[test]
fn three_anchor_range_matches_the_v1_input_range_normalizer() {
    let old = smart_controller_core::legacy::InputRange { minimum: -45.0, neutral: 5.0, maximum: 80.0 };
    let new = ThreePointRange::new(old.minimum, old.neutral, old.maximum).unwrap();
    for step in 0..=200 {
        let input = -60.0 + step as f64 * 0.75;
        let actual = new.normalize_signed(input).unwrap();
        let expected = old.normalize(input).unwrap();
        assert!((actual - expected).abs() <= 1e-12);
    }
}
