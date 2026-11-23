//! Endpoint policy shared by canonical controller mappings and compatibility APIs.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointPolicy {
    /// [start, end): the end frame is reserved as the wrap point.
    Exclusive,
    /// [start, end]: both endpoint frames are reachable.
    Inclusive,
}

impl Default for EndpointPolicy {
    fn default() -> Self {
        Self::Exclusive
    }
}
