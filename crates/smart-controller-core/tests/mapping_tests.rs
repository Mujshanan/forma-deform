use smart_controller_core::{
    mapping::{map_to_frame, normalize_rotation},
    EndpointPolicy,
};

#[test]
fn exclusive_endpoint_never_selects_end() {
    assert_eq!(map_to_frame(1.0, 10, 20, EndpointPolicy::Exclusive), 19);
}
#[test]
fn inclusive_endpoint_selects_end() {
    assert_eq!(map_to_frame(1.0, 10, 20, EndpointPolicy::Inclusive), 20);
}
#[test]
fn degenerate_frame_range_returns_start() {
    assert_eq!(map_to_frame(1.0, 7, 7, EndpointPolicy::Exclusive), 7);
    assert_eq!(map_to_frame(0.5, 7, 7, EndpointPolicy::Inclusive), 7);
}
#[test]
fn inverted_rotation_range_normalizes_in_reverse_direction() {
    assert_eq!(normalize_rotation(45.0, 45.0, -45.0).unwrap(), 0.0);
    assert_eq!(normalize_rotation(0.0, 45.0, -45.0).unwrap(), 0.5);
    assert_eq!(normalize_rotation(-45.0, 45.0, -45.0).unwrap(), 1.0);
}
