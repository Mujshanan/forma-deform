use crate::{
    mapping::{map_to_frame, normalize_rotation, transform_value},
    model::{EndpointPolicy, Mode, Params},
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreviewOutput {
    pub mode: &'static str,
    pub input: f64,
    pub normalized: f64,
    pub output_frame: Option<u32>,
    pub output_value: Option<f64>,
    pub notes: Vec<String>,
}

pub fn preview_slider(params: &Params, driver_value: f64) -> Result<PreviewOutput, String> {
    if params.mode != Mode::Slider {
        return Err("preview_slider requires slider mode".into());
    }
    let minimum = params
        .min_rotation_deg
        .ok_or("min_rotation_deg is missing")?;
    let maximum = params
        .max_rotation_deg
        .ok_or("max_rotation_deg is missing")?;
    let start = params
        .action_start_frame
        .ok_or("action_start_frame is missing")?;
    let end = params
        .action_end_frame
        .ok_or("action_end_frame is missing")?;
    let policy = params.endpoint_policy();
    let normalized = normalize_rotation(driver_value, minimum, maximum)?;
    let frame = map_to_frame(normalized, start, end, policy);
    let mut notes = vec![format!(
        "endpoint_policy: {}",
        match policy {
            EndpointPolicy::Exclusive => "exclusive",
            EndpointPolicy::Inclusive => "inclusive",
        }
    )];
    if policy == EndpointPolicy::Exclusive {
        notes.push("exclusive endpoint: action_end_frame is not selected".into());
    }
    Ok(PreviewOutput {
        mode: "slider",
        input: driver_value,
        normalized,
        output_frame: Some(frame),
        output_value: None,
        notes,
    })
}

/// Pure V2 transform calculation foundation; the V1 host adapter rejects this mode.
pub fn preview_transform(params: &Params, driver_value: f64) -> Result<PreviewOutput, String> {
    if params.mode != Mode::Transform {
        return Err("preview_transform requires transform mode".into());
    }
    let minimum = params
        .min_rotation_deg
        .ok_or("min_rotation_deg is missing")?;
    let maximum = params
        .max_rotation_deg
        .ok_or("max_rotation_deg is missing")?;
    let scale = params.scale.ok_or("scale is missing")?;
    let normalized = normalize_rotation(driver_value, minimum, maximum)?;
    let value = transform_value(normalized, scale, params.additive)?;
    Ok(PreviewOutput {
        mode: "transform",
        input: driver_value,
        normalized,
        output_frame: None,
        output_value: Some(value),
        notes: vec![
            format!(
                "property: {}",
                params.property.as_deref().unwrap_or_default()
            ),
            format!("additive: {}", params.additive),
            "pure calculation only; host transform application is deferred to V2".into(),
        ],
    })
}
