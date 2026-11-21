//! Host-independent Smart Controller calculation crate.
pub mod canonical;
pub mod contribution;
pub mod diagnostic_codes;
pub mod endpoint;
pub mod error;
pub mod graph;
pub mod ids;
pub mod input;
pub mod legacy;
pub mod mapping;
pub mod model;
pub mod migration;
pub mod preview;
pub mod schema;
pub mod validation;
pub mod value;

pub use error::CoreError;
pub use ids::{BindingId, ControllerId, InputId, TargetId};
pub use input::{BoneTransformChannel, FallbackPolicy, Freshness, InputDefinition, InputDimension, InputSample, InputSnapshot, InputSource, InputValidity};
pub use legacy::*;
pub use model::{EndpointPolicy, Mode, Params};
pub use preview::{PreviewOutput, preview_slider, preview_transform};
pub use value::{Angle, Scalar, TypedValue, Vector2};
