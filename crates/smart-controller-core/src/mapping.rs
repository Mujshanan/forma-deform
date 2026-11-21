use crate::model::EndpointPolicy;

/// Normalize a signed driver angle into [0, 1]. Reversed endpoints are valid.
pub fn normalize_rotation(input: f64, minimum: f64, maximum: f64) -> Result<f64, String> {
    if !input.is_finite() || !minimum.is_finite() || !maximum.is_finite() || minimum == maximum {
        return Err("input and rotation endpoints must be finite and endpoints must differ".into());
    }
    let clamped = input.clamp(minimum.min(maximum), minimum.max(maximum));
    let normalized = (clamped - minimum) / (maximum - minimum);
    if normalized.is_finite() { Ok(normalized.clamp(0.0, 1.0)) }
    else { Err("rotation normalization produced a non-finite result".into()) }
}

/// Exclusive maps [start, end); inclusive maps [start, end].
/// A degenerate interval always returns start.
pub fn map_to_frame(normalized: f64, start: u32, end: u32, policy: EndpointPolicy) -> u32 {
    debug_assert!(end >= start);
    let span = (end - start) as f64;
    if span == 0.0 { return start; }
    let t = normalized.clamp(0.0, 1.0);
    match policy {
        EndpointPolicy::Exclusive => start + ((t * span).floor().min(span - 1.0) as u32),
        EndpointPolicy::Inclusive => start + (t * span).round().min(span) as u32,
    }
}

/// Scale a normalized controller value into a target property value.
pub fn transform_value(normalized: f64, scale: f64, additive: bool) -> Result<f64, String> {
    if !normalized.is_finite() || !scale.is_finite() {
        return Err("transform input and scale must be finite".into());
    }
    let value = normalized.clamp(0.0, 1.0) * scale;
    if !value.is_finite() { return Err("transform mapping produced a non-finite value".into()); }
    let _ = additive; // retained in the V2 contract; application semantics belong to its adapter.
    Ok(value)
}


#[cfg(test)]
mod compatibility_tests {
    use super::*;

    #[test]
    fn rotation_normalization_clamps_and_preserves_reversed_ranges() {
        assert_eq!(normalize_rotation(-90.0, -45.0, 45.0).unwrap(), 0.0);
        assert_eq!(normalize_rotation(90.0, -45.0, 45.0).unwrap(), 1.0);
        assert_eq!(normalize_rotation(0.0, 45.0, -45.0).unwrap(), 0.5);
        assert_eq!(normalize_rotation(45.0, 45.0, -45.0).unwrap(), 0.0);
        assert_eq!(normalize_rotation(-45.0, 45.0, -45.0).unwrap(), 1.0);
    }

    #[test]
    fn rotation_normalization_rejects_non_finite_and_degenerate_ranges() {
        assert!(normalize_rotation(f64::NAN, -1.0, 1.0).is_err());
        assert!(normalize_rotation(0.0, f64::INFINITY, 1.0).is_err());
        assert!(normalize_rotation(0.0, 1.0, 1.0).is_err());
    }

    #[test]
    fn frame_mapping_preserves_exclusive_and_inclusive_endpoints() {
        assert_eq!(map_to_frame(0.0, 10, 20, EndpointPolicy::Exclusive), 10);
        assert_eq!(map_to_frame(1.0, 10, 20, EndpointPolicy::Exclusive), 19);
        assert_eq!(map_to_frame(0.0, 10, 20, EndpointPolicy::Inclusive), 10);
        assert_eq!(map_to_frame(1.0, 10, 20, EndpointPolicy::Inclusive), 20);
        assert_eq!(map_to_frame(0.5, 10, 20, EndpointPolicy::Inclusive), 15);
    }

    #[test]
    fn frame_mapping_clamps_and_handles_degenerate_intervals() {
        assert_eq!(map_to_frame(-1.0, 10, 20, EndpointPolicy::Inclusive), 10);
        assert_eq!(map_to_frame(2.0, 10, 20, EndpointPolicy::Inclusive), 20);
        assert_eq!(map_to_frame(0.75, 12, 12, EndpointPolicy::Exclusive), 12);
    }

    #[test]
    fn transform_additive_flag_retains_current_non_composing_behavior() {
        assert_eq!(transform_value(0.25, 8.0, false).unwrap(), 2.0);
        assert_eq!(transform_value(0.25, 8.0, true).unwrap(), 2.0);
    }

    #[test]
    fn transform_rejects_non_finite_inputs_and_overflow() {
        assert!(transform_value(f64::NAN, 1.0, false).is_err());
        assert!(transform_value(0.5, f64::INFINITY, false).is_err());
        assert!(transform_value(1.0, f64::MAX, false).is_ok());
    }
}
