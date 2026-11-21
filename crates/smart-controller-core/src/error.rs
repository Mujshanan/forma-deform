//! Typed errors shared by the host-independent Smart Controller core.
//!
//! Keep these errors independent of UI strings and host SDK types. Display text is
//! intentionally concise; callers should match on variants for stable behaviour.

use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreError {
    InvalidIdentifier { kind: &'static str },
    NonFiniteValue { context: &'static str },
    DuplicateInputId(String),
    InvalidInputSample { input_id: String, reason: &'static str },
    InputValueTypeMismatch {
        input_id: String,
        expected: &'static str,
        actual: &'static str,
    },
    MissingInput(String),
    StaleInput {
        input_id: String,
        age_ticks: u64,
        max_age_ticks: u64,
    },
    InvalidModel(String),
    InvalidMapping(String),
    InvalidGraph(String),
    CycleDetected(Vec<String>),
    CompositionConflict { target_id: String, reason: String },
    UnsupportedCompositionMode(String),
    UnsupportedSchemaVersion(u32),
    Serialization(String),
    Evaluation(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { kind } => write!(f, "{kind} identifier must not be empty"),
            Self::NonFiniteValue { context } => write!(f, "{context} must be finite"),
            Self::DuplicateInputId(id) => write!(f, "duplicate input ID: {id}"),
            Self::InvalidInputSample { input_id, reason } => {
                write!(f, "invalid sample for input {input_id}: {reason}")
            }
            Self::InputValueTypeMismatch { input_id, expected, actual } => {
                write!(f, "input {input_id} expects {expected}, received {actual}")
            }
            Self::MissingInput(id) => write!(f, "required input is missing: {id}"),
            Self::StaleInput { input_id, age_ticks, max_age_ticks } => write!(
                f,
                "input {input_id} is stale (age {age_ticks} ticks, maximum {max_age_ticks})"
            ),
            Self::InvalidModel(message) => write!(f, "invalid controller model: {message}"),
            Self::InvalidMapping(message) => write!(f, "invalid mapping: {message}"),
            Self::InvalidGraph(message) => write!(f, "invalid controller graph: {message}"),
            Self::CycleDetected(ids) => write!(f, "controller dependency cycle detected: {}", ids.join(" -> ")),
            Self::CompositionConflict { target_id, reason } => {
                write!(f, "composition conflict for target {target_id}: {reason}")
            }
            Self::UnsupportedCompositionMode(mode) => {
                write!(f, "unsupported composition mode: {mode}")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported schema version: {version}")
            }
            Self::Serialization(message) => write!(f, "serialization failed: {message}"),
            Self::Evaluation(message) => write!(f, "evaluation failed: {message}"),
        }
    }
}

impl Error for CoreError {}
