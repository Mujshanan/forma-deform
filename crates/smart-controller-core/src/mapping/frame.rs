//! Pure frame mapping; endpoint semantics match the characterized V1 policies.
use crate::model::EndpointPolicy;

pub fn map_to_frame(normalized: f64, start: u32, end: u32, policy: EndpointPolicy) -> u32 {
    assert!(end >= start, "frame interval must be ordered");
    let span = (end-start) as f64;
    if span == 0.0 { return start; }
    let t = normalized.clamp(0.0,1.0);
    match policy {
        EndpointPolicy::Exclusive => start + ((t*span).floor().min(span-1.0) as u32),
        EndpointPolicy::Inclusive => start + (t*span).round().min(span) as u32,
    }
}
