//! Host-independent Smart Controller calculation crate.
//!
//! The canonical persisted three-anchor binding API lives in [`binding`].
//! The [`legacy`] module remains as a source-compatible re-export. Plugin
//! command parameters and preview mapping remain separate APIs because they
//! have different data and execution contracts.

pub mod binding;
pub mod legacy;
pub mod mapping;
pub mod model;
pub mod preview;
pub mod validation;

pub use binding::*;
pub use model::{EndpointPolicy, Mode, Params};
pub use preview::{PreviewOutput, preview_slider, preview_transform};
