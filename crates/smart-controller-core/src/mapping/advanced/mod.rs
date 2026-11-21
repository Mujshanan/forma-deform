//! Host-independent advanced two-dimensional pose mappings.
pub mod grid;
pub mod interpolation;
pub mod triangulated;

use crate::{error::CoreError, pose::Pose2D, value::Vector2};

/// Common contract for mapping a 2D driver position to a complete typed pose.
pub trait PoseMapping2D {
    fn map_pose(&self, point: Vector2) -> Result<Pose2D, CoreError>;
}

impl PoseMapping2D for grid::GridMapping {
    fn map_pose(&self, point: Vector2) -> Result<Pose2D, CoreError> {
        self.evaluate(point)
    }
}

impl PoseMapping2D for triangulated::TriangulatedMapping {
    fn map_pose(&self, point: Vector2) -> Result<Pose2D, CoreError> {
        self.evaluate(point)
    }
}
