//! Stable machine-readable diagnostic identifiers. Display messages are not API.
pub const INPUT_MISSING: &str = "SC_INPUT_MISSING";
pub const INPUT_STALE: &str = "SC_INPUT_STALE";
pub const INPUT_INVALID: &str = "SC_INPUT_INVALID";
pub const INPUT_UNAVAILABLE: &str = "SC_INPUT_UNAVAILABLE";
pub const GRAPH_INVALID: &str = "SC_GRAPH_INVALID";
pub const GRAPH_CYCLE: &str = "SC_GRAPH_CYCLE";
pub const MAPPING_DOMAIN: &str = "SC_MAPPING_DOMAIN";
pub const MAPPING_TYPE: &str = "SC_MAPPING_TYPE";
pub const COMPOSITION_CONFLICT: &str = "SC_COMPOSITION_CONFLICT";
pub const COMPOSITION_WEIGHT_INVALID: &str = "SC_COMPOSITION_WEIGHT_INVALID";
pub const COMPOSITION_NO_ELIGIBLE: &str = "SC_COMPOSITION_NO_ELIGIBLE";
pub const CONTROLLER_DISABLED: &str = "SC_CONTROLLER_DISABLED";
pub const OUTPUT_SKIPPED: &str = "SC_OUTPUT_SKIPPED";

/// Registry order is stable and is mirrored by diagnostic_codes_v1.json.
pub const ALL_CODES: &[&str] = &[
    INPUT_MISSING,
    INPUT_STALE,
    INPUT_INVALID,
    INPUT_UNAVAILABLE,
    GRAPH_INVALID,
    GRAPH_CYCLE,
    MAPPING_DOMAIN,
    MAPPING_TYPE,
    COMPOSITION_CONFLICT,
    COMPOSITION_WEIGHT_INVALID,
    COMPOSITION_NO_ELIGIBLE,
    CONTROLLER_DISABLED,
    OUTPUT_SKIPPED,
];

use crate::{
    error::CoreError,
    ids::{ControllerId, InputId, TargetId},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Public structured diagnostic. The code is stable; the message is explanatory
/// and must never be used as a machine-readable discriminator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_id: Option<ControllerId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_id: Option<InputId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<TargetId>,
}

impl Diagnostic {
    pub fn new(
        code: impl Into<String>,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
    ) -> Result<Self, CoreError> {
        let code = code.into();
        if !ALL_CODES.contains(&code.as_str()) {
            return Err(CoreError::InvalidModel(format!(
                "unregistered diagnostic code: {code}"
            )));
        }
        Ok(Self {
            code,
            severity,
            message: message.into(),
            controller_id: None,
            input_id: None,
            target_id: None,
        })
    }

    pub fn with_controller(mut self, id: ControllerId) -> Self {
        self.controller_id = Some(id);
        self
    }
    pub fn with_input(mut self, id: InputId) -> Self {
        self.input_id = Some(id);
        self
    }
    pub fn with_target(mut self, id: TargetId) -> Self {
        self.target_id = Some(id);
        self
    }
}
