use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode { Slider, Transform }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointPolicy {
    /// [start, end): the end frame is reserved as the wrap point.
    Exclusive,
    /// [start, end]: both endpoint frames are reachable.
    Inclusive,
}
impl Default for EndpointPolicy {
    fn default() -> Self { Self::Exclusive }
}

/// Host-independent preview parameters. Host identity is resolved only by the adapter.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Params {
    pub mode: Mode,
    #[serde(default)] pub driver_layer_id: Option<u64>,
    #[serde(default)] pub driver_bone_name: Option<String>,
    #[serde(default)] pub action_layer_id: Option<u64>,
    #[serde(default)] pub min_rotation_deg: Option<f64>,
    #[serde(default)] pub max_rotation_deg: Option<f64>,
    #[serde(default)] pub action_start_frame: Option<u32>,
    #[serde(default)] pub action_end_frame: Option<u32>,
    #[serde(default)] pub endpoint_policy: Option<EndpointPolicy>,
    #[serde(default)] pub target_bone_name: Option<String>,
    #[serde(default)] pub property: Option<String>,
    #[serde(default)] pub scale: Option<f64>,
    #[serde(default)] pub additive: bool,
    /// Explicit deterministic input; when present, no host bone query occurs.
    #[serde(default)] pub manual_value: Option<f64>,
}
impl Params {
    pub fn endpoint_policy(&self) -> EndpointPolicy {
        self.endpoint_policy.unwrap_or_default()
    }
}
