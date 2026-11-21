//! Pure frame mapping; endpoint semantics match the characterized V1 policies.
use crate::{endpoint::EndpointPolicy, error::CoreError};

pub fn map_to_frame(normalized: f64, start: u32, end: u32, policy: EndpointPolicy) -> Result<u32, CoreError> {
    if end < start {
        return Err(CoreError::InvalidMapping("frame interval end must not precede start".into()));
    }
    if !normalized.is_finite() {
        return Err(CoreError::NonFiniteValue { context: "normalized frame input" });
    }
    let span = (end-start) as f64;
    if span == 0.0 { return Ok(start); }
    let t = normalized.clamp(0.0,1.0);
    Ok(match policy {
        EndpointPolicy::Exclusive => start + ((t*span).floor().min(span-1.0) as u32),
        EndpointPolicy::Inclusive => start + (t*span).round().min(span) as u32,
    })
}
