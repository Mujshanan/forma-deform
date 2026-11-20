//! Host-independent Smart Controller calculation crate.
//! Slider mapping and the three-anchor binding model remain pure here;
//! Asset Warp pin deformation math is owned by the separate form-deform-core crate.
pub mod legacy;
pub mod mapping;
pub mod model;
pub mod preview;
pub mod validation;
pub use legacy::*;
pub use model::{EndpointPolicy, Mode, Params};
pub use preview::{preview_slider, preview_transform, PreviewOutput};
