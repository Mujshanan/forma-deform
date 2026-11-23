//! Deterministic interpolation helpers shared by advanced 2D mappings.
use crate::{error::CoreError, pose::Pose2D, value::TypedValue};

/// Interpolate compatible typed poses in the supplied, deterministic order.
/// Zero-weight poses are ignored. Positive weights are normalized by repeated
/// pairwise interpolation; wrapped angles therefore use the core's documented
/// shortest-path and antipodal tie semantics.
pub(crate) fn interpolate_weighted_poses(
    samples: &[(&Pose2D, f64)],
) -> Result<Pose2D, CoreError> {
    if samples.is_empty() {
        return Err(CoreError::InvalidMapping("pose interpolation requires at least one sample".into()));
    }
    for (pose, weight) in samples {
        if !weight.is_finite() || *weight < 0.0 {
            return Err(CoreError::InvalidMapping("pose interpolation weights must be finite and non-negative".into()));
        }
        samples[0].0.validate_compatible(pose)?;
    }

    let mut total = 0.0;
    for (_, weight) in samples {
        total += *weight;
        if !total.is_finite() {
            return Err(CoreError::InvalidMapping("pose interpolation total weight is non-finite".into()));
        }
    }
    if total <= 0.0 {
        return Err(CoreError::InvalidMapping("pose interpolation requires positive total weight".into()));
    }

    let mut accumulated_weight = 0.0;
    let mut values: Vec<(crate::ids::BindingId, TypedValue)> = Vec::new();
    for (pose, weight) in samples {
        if *weight == 0.0 { continue; }
        if values.is_empty() {
            values = pose.values().to_vec();
            accumulated_weight = *weight;
            continue;
        }
        let amount = *weight / (accumulated_weight + *weight);
        for (id, value) in &mut values {
            let next = pose.get(id).ok_or_else(|| CoreError::PoseBindingMissing {
                binding_id: id.to_string(),
            })?;
            *value = value.interpolate(*next, amount)?;
        }
        accumulated_weight += *weight;
    }
    Pose2D::new(values)
}
