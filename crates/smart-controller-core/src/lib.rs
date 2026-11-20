//! Host-independent Smart Controller calculation crate.
//! New Slider/Transform mapping is split into focused modules. The legacy
//! three-anchor binding API remains available for the Form Deform contract.
pub mod legacy;
pub mod mapping;
pub mod model;
pub mod preview;
pub mod validation;
pub use legacy::*;
pub use model::{EndpointPolicy, Mode, Params};
pub use preview::{PreviewOutput, preview_slider, preview_transform};
