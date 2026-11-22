//! Pure Smart Controller binding model and evaluator.
//!
//! This crate has no AniCraft, WASM, browser, or UI dependencies.
//! Host identity resolution, persistence, and display belong in an adapter crate.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputRange {
    pub minimum: f64,
    pub neutral: f64,
    pub maximum: f64,
}

impl InputRange {
    pub fn validate(self) -> Result<(), ControllerError> {
        crate::mapping::range::ThreePointRange::new(
            self.minimum,
            self.neutral,
            self.maximum,
        )
        .map(|_| ())
        .map_err(|_| ControllerError::InvalidInputRange)
    }

    /// Clamp and normalize an input to [-1, 1], preserving neutral at exactly 0.
    /// The checked core mapping primitive owns the arithmetic; this legacy
    /// wrapper preserves the V1 error contract.
    pub fn normalize(self, input: f64) -> Result<f64, ControllerError> {
        if !input.is_finite() {
            return Err(ControllerError::NonFiniteInput);
        }
        self.validate()?;
        crate::mapping::range::ThreePointRange::new(
            self.minimum,
            self.neutral,
            self.maximum,
        )
        .and_then(|range| range.normalize_signed(input))
        .map_err(|_| ControllerError::ArithmeticOverflow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EaseCurve {
    Linear,
    Smoothstep,
}

impl EaseCurve {
    fn apply(self, value: f64) -> f64 {
        let easing = match self {
            Self::Linear => crate::mapping::easing::Easing::Linear,
            Self::Smoothstep => crate::mapping::easing::Easing::Smoothstep,
        };
        // ControllerBinding::evaluate supplies a finite normalized value.
        easing.apply(value).expect("validated legacy input must be finite")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControllerMode {
    /// Return the absolute value/offset declared by the target anchors.
    AbsolutePose,
    /// Return a geometry offset relative to the target's neutral anchor.
    RelativeDelta,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControllerTarget {
    SymbolFrame {
        symbol_instance_id: String,
        negative_frame: u32,
        neutral_frame: u32,
        positive_frame: u32,
    },
    GeometryDeform {
        element_id: String,
        pin_id: String,
        negative_offset: Vec2,
        neutral_offset: Vec2,
        positive_offset: Vec2,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerBinding {
    /// Unique identity of this binding (not the controller/bone identity).
    pub id: String,
    /// Stable host identity of the input controller/bone.
    pub controller_id: String,
    pub input_range: InputRange,
    pub easing: EaseCurve,
    pub mode: ControllerMode,
    pub target: ControllerTarget,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PreviewResult {
    SymbolFrame {
        symbol_instance_id: String,
        frame: u32,
    },
    GeometryDeform {
        element_id: String,
        pin_id: String,
        offset: Vec2,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControllerError {
    EmptyBindingId,
    EmptyControllerId,
    EmptyTargetId,
    InvalidInputRange,
    NonFiniteInput,
    NonFiniteTargetValue,
    RelativeDeltaUnsupportedForSymbolFrame,
    DuplicateBindingId(String),
    BindingNotFound(String),
    UnsupportedSchemaVersion(u32),
    Serialization(String),
    ArithmeticOverflow,
}

impl fmt::Display for ControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBindingId => write!(f, "binding ID must not be empty"),
            Self::EmptyControllerId => write!(f, "controller ID must not be empty"),
            Self::EmptyTargetId => write!(f, "target ID must not be empty"),
            Self::InvalidInputRange => write!(f, "input range must be finite and strictly ordered"),
            Self::NonFiniteInput => write!(f, "controller input must be finite"),
            Self::NonFiniteTargetValue => write!(f, "target anchors must contain only finite values"),
            Self::RelativeDeltaUnsupportedForSymbolFrame => {
                write!(f, "RelativeDelta is not supported for SymbolFrame targets")
            }
            Self::DuplicateBindingId(id) => write!(f, "duplicate binding ID: {id}"),
            Self::BindingNotFound(id) => write!(f, "binding not found: {id}"),
            Self::UnsupportedSchemaVersion(version) => write!(f, "unsupported binding schema version: {version}"),
            Self::Serialization(message) => write!(f, "binding serialization failed: {message}"),
            Self::ArithmeticOverflow => write!(f, "controller evaluation produced a non-finite result"),
        }
    }
}

impl Error for ControllerError {}

impl ControllerBinding {
    pub fn validate(&self) -> Result<(), ControllerError> {
        if self.id.trim().is_empty() {
            return Err(ControllerError::EmptyBindingId);
        }
        if self.controller_id.trim().is_empty() {
            return Err(ControllerError::EmptyControllerId);
        }
        self.input_range.validate()?;

        match &self.target {
            ControllerTarget::SymbolFrame { symbol_instance_id, .. } => {
                if symbol_instance_id.trim().is_empty() {
                    return Err(ControllerError::EmptyTargetId);
                }
                if self.mode == ControllerMode::RelativeDelta {
                    return Err(ControllerError::RelativeDeltaUnsupportedForSymbolFrame);
                }
            }
            ControllerTarget::GeometryDeform {
                element_id,
                pin_id,
                negative_offset,
                neutral_offset,
                positive_offset,
            } => {
                if element_id.trim().is_empty() || pin_id.trim().is_empty() {
                    return Err(ControllerError::EmptyTargetId);
                }
                if !negative_offset.is_finite()
                    || !neutral_offset.is_finite()
                    || !positive_offset.is_finite()
                {
                    return Err(ControllerError::NonFiniteTargetValue);
                }
            }
        }
        Ok(())
    }

    /// Evaluate one explicit controller input. This function never mutates the binding.
    /// A disabled binding intentionally produces an empty preview result.
    pub fn evaluate(&self, input: f64) -> Result<Option<PreviewResult>, ControllerError> {
        self.validate()?;
        if !self.enabled {
            return Ok(None);
        }

        let normalized = self.input_range.normalize(input)?;
        let eased = if normalized < 0.0 {
            -self.easing.apply(-normalized)
        } else {
            self.easing.apply(normalized)
        };

        let result = match &self.target {
            ControllerTarget::SymbolFrame {
                symbol_instance_id,
                negative_frame,
                neutral_frame,
                positive_frame,
            } => {
                let frame = if eased < 0.0 {
                    interpolate(
                        *negative_frame as f64,
                        *neutral_frame as f64,
                        eased + 1.0,
                    )?
                } else {
                    interpolate(*neutral_frame as f64, *positive_frame as f64, eased)?
                };
                PreviewResult::SymbolFrame {
                    symbol_instance_id: symbol_instance_id.clone(),
                    frame: frame.round() as u32,
                }
            }
            ControllerTarget::GeometryDeform {
                element_id,
                pin_id,
                negative_offset,
                neutral_offset,
                positive_offset,
            } => {
                let absolute = if eased < 0.0 {
                    interpolate_vec2(*negative_offset, *neutral_offset, eased + 1.0)?
                } else {
                    interpolate_vec2(*neutral_offset, *positive_offset, eased)?
                };
                let offset = match self.mode {
                    ControllerMode::AbsolutePose => absolute,
                    ControllerMode::RelativeDelta => Vec2 {
                        x: absolute.x - neutral_offset.x,
                        y: absolute.y - neutral_offset.y,
                    },
                };
                if !offset.is_finite() {
                    return Err(ControllerError::ArithmeticOverflow);
                }
                PreviewResult::GeometryDeform {
                    element_id: element_id.clone(),
                    pin_id: pin_id.clone(),
                    offset,
                }
            }
        };

        Ok(Some(result))
    }
}

/// Current version of the persisted binding-set schema.
pub const BINDING_SCHEMA_VERSION: u32 = 1;

/// Versioned, serializable collection of controller bindings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingSet {
    pub schema_version: u32,
    pub bindings: Vec<ControllerBinding>,
}

impl BindingSet {
    pub fn new(bindings: Vec<ControllerBinding>) -> Result<Self, ControllerError> {
        let set = Self {
            schema_version: BINDING_SCHEMA_VERSION,
            bindings,
        };
        set.validate()?;
        Ok(set)
    }

    pub fn validate(&self) -> Result<(), ControllerError> {
        if self.schema_version != BINDING_SCHEMA_VERSION {
            return Err(ControllerError::UnsupportedSchemaVersion(self.schema_version));
        }
        validate_bindings(&self.bindings)
    }

    /// Find a binding by its stable binding ID.
    pub fn get(&self, id: &str) -> Option<&ControllerBinding> {
        self.bindings.iter().find(|binding| binding.id == id)
    }

    /// Add a new binding. Existing IDs are never silently overwritten.
    pub fn insert(&mut self, binding: ControllerBinding) -> Result<(), ControllerError> {
        binding.validate()?;
        if self.get(&binding.id).is_some() {
            return Err(ControllerError::DuplicateBindingId(binding.id));
        }
        self.bindings.push(binding);
        Ok(())
    }

    /// Replace an existing binding identified by the replacement's stable ID.
    pub fn update(&mut self, binding: ControllerBinding) -> Result<(), ControllerError> {
        binding.validate()?;
        let id = binding.id.clone();
        let Some(existing) = self.bindings.iter_mut().find(|item| item.id == id) else {
            return Err(ControllerError::BindingNotFound(id));
        };
        *existing = binding;
        Ok(())
    }

    /// Remove a binding by ID, returning the removed value when it existed.
    pub fn remove(&mut self, id: &str) -> Option<ControllerBinding> {
        let index = self.bindings.iter().position(|binding| binding.id == id)?;
        Some(self.bindings.remove(index))
    }

    pub fn to_json(&self) -> Result<String, ControllerError> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map_err(|error| ControllerError::Serialization(error.to_string()))
    }

    pub fn from_json(source: &str) -> Result<Self, ControllerError> {
        let set: Self = serde_json::from_str(source)
            .map_err(|error| ControllerError::Serialization(error.to_string()))?;
        set.validate()?;
        Ok(set)
    }
}

/// Validate a collection and reject duplicate binding identities.
pub fn validate_bindings(bindings: &[ControllerBinding]) -> Result<(), ControllerError> {
    let mut ids = HashSet::with_capacity(bindings.len());
    for binding in bindings {
        binding.validate()?;
        if !ids.insert(binding.id.as_str()) {
            return Err(ControllerError::DuplicateBindingId(binding.id.clone()));
        }
    }
    Ok(())
}

fn interpolate(from: f64, to: f64, amount: f64) -> Result<f64, ControllerError> {
    // Weighted interpolation avoids overflowing the endpoint difference.
    let value = from * (1.0 - amount) + to * amount;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ControllerError::ArithmeticOverflow)
    }
}

fn interpolate_vec2(from: Vec2, to: Vec2, amount: f64) -> Result<Vec2, ControllerError> {
    let value = Vec2 {
        x: interpolate(from.x, to.x, amount)?,
        y: interpolate(from.y, to.y, amount)?,
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ControllerError::ArithmeticOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol_binding() -> ControllerBinding {
        ControllerBinding {
            id: "binding-head-turn".into(),
            controller_id: "bone-head-turn".into(),
            input_range: InputRange {
                minimum: -45.0,
                neutral: 0.0,
                maximum: 45.0,
            },
            easing: EaseCurve::Linear,
            mode: ControllerMode::AbsolutePose,
            target: ControllerTarget::SymbolFrame {
                symbol_instance_id: "instance-head".into(),
                negative_frame: 1,
                neutral_frame: 5,
                positive_frame: 9,
            },
            enabled: true,
        }
    }

    fn geometry_binding(mode: ControllerMode) -> ControllerBinding {
        ControllerBinding {
            id: "binding-arm-corrective".into(),
            controller_id: "bone-arm".into(),
            input_range: InputRange {
                minimum: -45.0,
                neutral: 0.0,
                maximum: 45.0,
            },
            easing: EaseCurve::Linear,
            mode,
            target: ControllerTarget::GeometryDeform {
                element_id: "shape-arm".into(),
                pin_id: "pin-arm-tip".into(),
                negative_offset: Vec2 { x: -4.0, y: 2.0 },
                neutral_offset: Vec2 { x: 1.0, y: 1.0 },
                positive_offset: Vec2 { x: 9.0, y: 5.0 },
            },
            enabled: true,
        }
    }

    #[test]
    fn symbol_frame_maps_negative_neutral_and_positive_inputs() {
        let binding = symbol_binding();
        assert_eq!(
            binding.evaluate(-45.0).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 1 })
        );
        assert_eq!(
            binding.evaluate(0.0).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 5 })
        );
        assert_eq!(
            binding.evaluate(45.0).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 9 })
        );
    }

    #[test]
    fn symbol_frame_interpolates_and_rounds() {
        let binding = symbol_binding();
        assert_eq!(
            binding.evaluate(-22.5).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 3 })
        );
    }

    #[test]
    fn input_is_clamped_to_the_configured_range() {
        let binding = symbol_binding();
        assert_eq!(binding.evaluate(-90.0).unwrap(), binding.evaluate(-45.0).unwrap());
        assert_eq!(binding.evaluate(90.0).unwrap(), binding.evaluate(45.0).unwrap());
    }

    #[test]
    fn smoothstep_preserves_neutral_and_eases_each_side() {
        let mut binding = symbol_binding();
        binding.easing = EaseCurve::Smoothstep;
        assert_eq!(
            binding.evaluate(0.0).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 5 })
        );
        assert_eq!(
            binding.evaluate(22.5).unwrap(),
            Some(PreviewResult::SymbolFrame { symbol_instance_id: "instance-head".into(), frame: 7 })
        );
    }

    #[test]
    fn relative_geometry_delta_is_zero_at_neutral() {
        let binding = geometry_binding(ControllerMode::RelativeDelta);
        assert_eq!(
            binding.evaluate(0.0).unwrap(),
            Some(PreviewResult::GeometryDeform {
                element_id: "shape-arm".into(),
                pin_id: "pin-arm-tip".into(),
                offset: Vec2 { x: 0.0, y: 0.0 }
            })
        );
        assert_eq!(
            binding.evaluate(-45.0).unwrap(),
            Some(PreviewResult::GeometryDeform {
                element_id: "shape-arm".into(),
                pin_id: "pin-arm-tip".into(),
                offset: Vec2 { x: -5.0, y: 1.0 }
            })
        );
    }

    #[test]
    fn absolute_geometry_returns_the_anchored_offset() {
        let binding = geometry_binding(ControllerMode::AbsolutePose);
        assert_eq!(
            binding.evaluate(45.0).unwrap(),
            Some(PreviewResult::GeometryDeform {
                element_id: "shape-arm".into(),
                pin_id: "pin-arm-tip".into(),
                offset: Vec2 { x: 9.0, y: 5.0 }
            })
        );
    }

    #[test]
    fn relative_delta_is_rejected_for_symbol_frames() {
        let mut binding = symbol_binding();
        binding.mode = ControllerMode::RelativeDelta;
        assert_eq!(
            binding.validate(),
            Err(ControllerError::RelativeDeltaUnsupportedForSymbolFrame)
        );
    }

    #[test]
    fn invalid_and_overflowing_input_ranges_are_rejected() {
        let invalid = InputRange { minimum: 0.0, neutral: 0.0, maximum: 1.0 };
        assert_eq!(invalid.validate(), Err(ControllerError::InvalidInputRange));

        let overflowing = InputRange {
            minimum: -f64::MAX,
            neutral: f64::MAX / 2.0,
            maximum: f64::MAX,
        };
        assert_eq!(overflowing.validate(), Err(ControllerError::InvalidInputRange));
    }

    #[test]
    fn non_finite_input_is_rejected() {
        let binding = symbol_binding();
        assert_eq!(binding.evaluate(f64::NAN), Err(ControllerError::NonFiniteInput));
    }

    #[test]
    fn non_finite_geometry_anchor_is_rejected() {
        let mut binding = geometry_binding(ControllerMode::AbsolutePose);
        if let ControllerTarget::GeometryDeform { positive_offset, .. } = &mut binding.target {
            positive_offset.x = f64::INFINITY;
        }
        assert_eq!(binding.validate(), Err(ControllerError::NonFiniteTargetValue));
    }

    #[test]
    fn relative_delta_detects_subtraction_overflow() {
        let mut binding = geometry_binding(ControllerMode::RelativeDelta);
        if let ControllerTarget::GeometryDeform {
            negative_offset,
            neutral_offset,
            ..
        } = &mut binding.target
        {
            negative_offset.x = -f64::MAX;
            neutral_offset.x = f64::MAX;
        }
        assert_eq!(binding.evaluate(-45.0), Err(ControllerError::ArithmeticOverflow));
    }

    #[test]
    fn disabled_binding_returns_no_preview() {
        let mut binding = symbol_binding();
        binding.enabled = false;
        assert_eq!(binding.evaluate(0.0).unwrap(), None);
    }

    #[test]
    fn binding_collection_rejects_duplicate_ids() {
        let first = symbol_binding();
        let mut second = geometry_binding(ControllerMode::AbsolutePose);
        second.id = first.id.clone();
        assert_eq!(
            validate_bindings(&[first, second]),
            Err(ControllerError::DuplicateBindingId("binding-head-turn".into()))
        );
    }

    #[test]
    fn binding_set_insert_get_update_and_remove_work_by_stable_id() {
        let first = symbol_binding();
        let mut set = BindingSet::new(vec![first.clone()]).unwrap();

        assert_eq!(set.get(&first.id), Some(&first));

        let mut added = geometry_binding(ControllerMode::AbsolutePose);
        added.id = "binding-leg".into();
        set.insert(added.clone()).unwrap();
        assert_eq!(set.get("binding-leg"), Some(&added));

        added.enabled = false;
        set.update(added.clone()).unwrap();
        assert_eq!(set.get("binding-leg"), Some(&added));

        let removed = set.remove("binding-leg").unwrap();
        assert_eq!(removed, added);
        assert_eq!(set.get("binding-leg"), None);
        assert_eq!(set.remove("missing"), None);
        assert_eq!(set.bindings.len(), 1);
    }

    #[test]
    fn binding_set_insert_rejects_duplicate_and_invalid_bindings() {
        let first = symbol_binding();
        let mut set = BindingSet::new(vec![first.clone()]).unwrap();
        assert_eq!(
            set.insert(first.clone()),
            Err(ControllerError::DuplicateBindingId(first.id))
        );

        let mut invalid = geometry_binding(ControllerMode::AbsolutePose);
        invalid.id = "   ".into();
        assert_eq!(set.insert(invalid), Err(ControllerError::EmptyBindingId));
        assert_eq!(set.bindings.len(), 1);
    }

    #[test]
    fn binding_set_update_requires_an_existing_id_and_valid_value() {
        let mut set = BindingSet::new(vec![symbol_binding()]).unwrap();
        let mut missing = geometry_binding(ControllerMode::AbsolutePose);
        missing.id = "missing".into();
        assert_eq!(
            set.update(missing),
            Err(ControllerError::BindingNotFound("missing".into()))
        );

        let mut invalid = symbol_binding();
        invalid.mode = ControllerMode::RelativeDelta;
        assert_eq!(
            set.update(invalid),
            Err(ControllerError::RelativeDeltaUnsupportedForSymbolFrame)
        );
        assert_eq!(set.bindings, vec![symbol_binding()]);
    }

    #[test]
    fn binding_set_json_round_trips() {
        let set = BindingSet::new(vec![
            symbol_binding(),
            geometry_binding(ControllerMode::RelativeDelta),
        ]).unwrap();
        let json = set.to_json().unwrap();
        let decoded = BindingSet::from_json(&json).unwrap();
        assert_eq!(decoded, set);
    }

    #[test]
    fn binding_set_rejects_unknown_schema_versions() {
        let set = BindingSet::new(vec![symbol_binding()]).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&set.to_json().unwrap()).unwrap();
        value["schema_version"] = serde_json::json!(2);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            BindingSet::from_json(&json),
            Err(ControllerError::UnsupportedSchemaVersion(2))
        );
    }

    #[test]
    fn binding_set_rejects_duplicate_binding_ids() {
        let first = symbol_binding();
        let mut second = geometry_binding(ControllerMode::AbsolutePose);
        second.id = first.id.clone();
        assert_eq!(
            BindingSet::new(vec![first, second]),
            Err(ControllerError::DuplicateBindingId("binding-head-turn".into()))
        );
    }

    #[test]
    fn repeated_evaluation_is_deterministic_and_does_not_mutate_binding() {
        let binding = geometry_binding(ControllerMode::RelativeDelta);
        let original = binding.clone();
        let first = binding.evaluate(18.0).unwrap();
        let second = binding.evaluate(18.0).unwrap();
        assert_eq!(first, second);
        assert_eq!(binding, original);
    }
}
