//! Compatibility module for the original three-anchor binding API.
//!
//! The implementation now lives in [`crate::binding`]. This module re-exports
//! the exact same public items so existing `smart_controller_core::legacy::*`
//! imports remain source-compatible.

pub use crate::binding::*;
