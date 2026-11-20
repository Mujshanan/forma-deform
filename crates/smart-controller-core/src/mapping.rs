use crate::model::EndpointPolicy;

/// Normalize a signed driver angle into [0, 1]. Reversed endpoints are valid.
pub fn normalize_rotation(input: f64, minimum: f64, maximum: f64) -> Result<f64, String> {
    if !input.is_finite() || !minimum.is_finite() || !maximum.is_finite() || minimum == maximum {
        return Err("input and rotation endpoints must be finite and endpoints must differ".into());
    }
    let clamped = input.clamp(minimum.min(maximum), minimum.max(maximum));
    let normalized = (clamped - minimum) / (maximum - minimum);
    if normalized.is_finite() {
        Ok(normalized.clamp(0.0, 1.0))
    } else {
        Err("rotation normalization produced a non-finite result".into())
    }
}

/// Exclusive maps [start, end); inclusive maps [start, end].
/// A degenerate interval always returns start.
pub fn map_to_frame(normalized: f64, start: u32, end: u32, policy: EndpointPolicy) -> u32 {
    debug_assert!(end >= start);
    let span = (end - start) as f64;
    if span == 0.0 {
        return start;
    }
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
    if !value.is_finite() {
        return Err("transform mapping produced a non-finite value".into());
    }
    let _ = additive; // retained in the V2 contract; application semantics belong to its adapter.
    Ok(value)
}
