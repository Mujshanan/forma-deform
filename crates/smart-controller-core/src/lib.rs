//! Host-independent Smart Controller calculation crate.
//! New Slider/Transform mapping is split into focused modules. The legacy
//! three-anchor binding API remains available for the Form Deform contract.

pub mod error;
pub mod ids;
pub mod input;
pub mod legacy;
pub mod mapping;
pub mod model;
pub mod preview;
pub mod validation;
pub mod value;

pub use error::CoreError;
pub use ids::{BindingId, ControllerId, InputId, TargetId};
pub use input::{
    BoneTransformChannel, FallbackPolicy, Freshness, InputDefinition, InputDimension,
    InputSample, InputSnapshot, InputSource, InputValidity,
};
pub use legacy::*;
pub use model::{EndpointPolicy, Mode, Params};
pub use preview::{PreviewOutput, preview_slider, preview_transform};
pub use value::{Angle, Scalar, TypedValue, Vector2};
