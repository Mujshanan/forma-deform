//! Rectilinear grid-based 2D pose mapping with bilinear interpolation.
use crate::{
    error::CoreError,
    pose::Pose2D,
    value::Vector2,
};
use super::interpolation::interpolate_weighted_poses;

/// Behaviour for points outside the authored grid rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridBoundaryPolicy {
    Reject,
    Clamp,
}

/// A validated rectilinear pose grid. Poses are stored row-major:
/// index = y_index * x_coordinates.len() + x_index.
#[derive(Clone, Debug, PartialEq)]
pub struct GridMapping {
    x_coordinates: Vec<f64>,
    y_coordinates: Vec<f64>,
    poses: Vec<Pose2D>,
    boundary: GridBoundaryPolicy,
}

impl GridMapping {
    pub fn new(
        x_coordinates: Vec<f64>,
        y_coordinates: Vec<f64>,
        poses: Vec<Pose2D>,
        boundary: GridBoundaryPolicy,
    ) -> Result<Self, CoreError> {
        validate_axis(&x_coordinates, "x")?;
        validate_axis(&y_coordinates, "y")?;
        let expected = x_coordinates.len().checked_mul(y_coordinates.len())
            .ok_or_else(|| CoreError::InvalidMapping("grid dimensions overflow".into()))?;
        if poses.len() != expected {
            return Err(CoreError::InvalidMapping(format!(
                "grid requires {expected} poses in row-major order, received {}",
                poses.len()
            )));
        }
        let first = poses.first().ok_or_else(|| CoreError::InvalidMapping("grid requires at least one pose".into()))?;
        for pose in poses.iter().skip(1) {
            first.validate_compatible(pose)?;
        }
        Ok(Self { x_coordinates, y_coordinates, poses, boundary })
    }

    pub fn x_coordinates(&self) -> &[f64] { &self.x_coordinates }
    pub fn y_coordinates(&self) -> &[f64] { &self.y_coordinates }
    pub fn poses(&self) -> &[Pose2D] { &self.poses }
    pub fn boundary_policy(&self) -> GridBoundaryPolicy { self.boundary }

    pub fn evaluate(&self, point: Vector2) -> Result<Pose2D, CoreError> {
        let x = apply_boundary(point.x(), &self.x_coordinates, self.boundary)?;
        let y = apply_boundary(point.y(), &self.y_coordinates, self.boundary)?;
        let (x0, tx) = locate_interval(x, &self.x_coordinates);
        let (y0, ty) = locate_interval(y, &self.y_coordinates);
        let x1 = x0 + 1;
        let y1 = y0 + 1;
        let nx = self.x_coordinates.len();
        let p00 = &self.poses[y0 * nx + x0];
        let p10 = &self.poses[y0 * nx + x1];
        let p01 = &self.poses[y1 * nx + x0];
        let p11 = &self.poses[y1 * nx + x1];

        // Bilinear interpolation uses a fixed x-then-y operation order.
        let lower = interpolate_weighted_poses(&[(p00, 1.0 - tx), (p10, tx)])?;
        let upper = interpolate_weighted_poses(&[(p01, 1.0 - tx), (p11, tx)])?;
        interpolate_weighted_poses(&[(&lower, 1.0 - ty), (&upper, ty)])
    }
}

fn validate_axis(values: &[f64], name: &str) -> Result<(), CoreError> {
    if values.len() < 2 {
        return Err(CoreError::InvalidMapping(format!("grid {name} axis requires at least two coordinates")));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(CoreError::NonFiniteValue { context: "grid coordinate" });
    }
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(CoreError::InvalidMapping(format!("grid {name} coordinates must be strictly increasing")));
    }
    if values.windows(2).any(|pair| !(pair[1] - pair[0]).is_finite()) {
        return Err(CoreError::InvalidMapping(format!("grid {name} coordinate intervals must be finite")));
    }
    Ok(())
}

fn apply_boundary(value: f64, axis: &[f64], policy: GridBoundaryPolicy) -> Result<f64, CoreError> {
    let minimum = axis[0];
    let maximum = *axis.last().expect("validated axis is non-empty");
    if value < minimum || value > maximum {
        match policy {
            GridBoundaryPolicy::Reject =>
                return Err(CoreError::InvalidMapping("query point lies outside the authored pose grid".into())),
            GridBoundaryPolicy::Clamp => return Ok(value.clamp(minimum, maximum)),
        }
    }
    Ok(value)
}

fn locate_interval(value: f64, axis: &[f64]) -> (usize, f64) {
    if value >= axis[axis.len() - 1] {
        return (axis.len() - 2, 1.0);
    }
    let upper = axis.partition_point(|coordinate| *coordinate <= value);
    let lower = upper.saturating_sub(1).min(axis.len() - 2);
    let amount = (value - axis[lower]) / (axis[lower + 1] - axis[lower]);
    (lower, amount.clamp(0.0, 1.0))
}
