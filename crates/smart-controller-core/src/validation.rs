use crate::model::{Mode, Params};

/// Validate parameters without consulting AniCraft or host state.
pub fn validate(params: &Params) -> Result<(), String> {
    if let Some(value) = params.manual_value {
        if !value.is_finite() {
            return Err("manual_value must be finite".into());
        }
    } else {
        let layer = params
            .driver_layer_id
            .ok_or("driver_layer_id is required when manual_value is absent")?;
        if layer == 0 {
            return Err("driver_layer_id must be greater than zero".into());
        }
        let bone_id = params
            .driver_bone_id
            .ok_or("driver_bone_id is required when manual_value is absent")?;
        if bone_id == 0 {
            return Err("driver_bone_id must be greater than zero".into());
        }
    }
    match params.mode {
        Mode::Slider => {
            let layer = params
                .action_layer_id
                .ok_or("action_layer_id is required in slider mode")?;
            if layer == 0 {
                return Err("action_layer_id must be greater than zero".into());
            }
            let min = params
                .min_rotation_deg
                .ok_or("min_rotation_deg is required in slider mode")?;
            let max = params
                .max_rotation_deg
                .ok_or("max_rotation_deg is required in slider mode")?;
            if !min.is_finite() || !max.is_finite() || min == max {
                return Err("rotation range must contain two distinct finite endpoints".into());
            }
            let start = params
                .action_start_frame
                .ok_or("action_start_frame is required in slider mode")?;
            let end = params
                .action_end_frame
                .ok_or("action_end_frame is required in slider mode")?;
            if end <= start {
                return Err("action_end_frame must be greater than action_start_frame".into());
            }
        }
        Mode::Transform => {
            let target = params
                .target_bone_name
                .as_deref()
                .ok_or("target_bone_name is required in transform mode")?;
            if target.trim().is_empty() {
                return Err("target_bone_name must not be empty".into());
            }
            let property = params
                .property
                .as_deref()
                .ok_or("property is required in transform mode")?;
            if property.trim().is_empty() {
                return Err("property must not be empty".into());
            }
            let scale = params.scale.ok_or("scale is required in transform mode")?;
            if !scale.is_finite() {
                return Err("scale must be finite".into());
            }
            let min = params
                .min_rotation_deg
                .ok_or("min_rotation_deg is required in transform mode")?;
            let max = params
                .max_rotation_deg
                .ok_or("max_rotation_deg is required in transform mode")?;
            if !min.is_finite() || !max.is_finite() || min == max {
                return Err("rotation range must contain two distinct finite endpoints".into());
            }
        }
    }
    Ok(())
}
