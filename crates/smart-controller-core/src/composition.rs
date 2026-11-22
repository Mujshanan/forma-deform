//! Deterministic, host-independent target composition.
//!
//! Composition is a pure post-pass over Phase 3 contributions. It never reads
//! or mutates host state and never combines Smart Controller results with
//! Form Deform pin-deformation evaluation.
use crate::{
    contribution::{BaseRef, CompositionMode, Contribution},
    diagnostic_codes::{Diagnostic, DiagnosticSeverity, COMPOSITION_NO_ELIGIBLE},
    error::CoreError,
    evaluator::{evaluate, ControllerMapper},
    graph::ControllerGraph,
    ids::TargetId,
    value::{Angle, TypedValue},
};
pub use crate::evaluator::EvaluationResult;
use std::collections::{BTreeMap, BTreeSet};

/// Explicit values needed to resolve non-authored additive bases.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompositionContext {
    /// Neutral/rest values supplied by the caller, keyed by canonical target.
    pub neutral_values: BTreeMap<TargetId, TypedValue>,
    /// Explicit input bases referenced by BaseRef::Input, keyed by source ID.
    pub input_values: BTreeMap<String, TypedValue>,
}

/// Final target values plus recoverable no-output diagnostics.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompositionResult {
    pub outputs: BTreeMap<TargetId, TypedValue>,
    pub diagnostics: Vec<Diagnostic>,
    /// Targets for which contributions existed but no valid output was eligible.
    pub unavailable_targets: BTreeSet<TargetId>,
}

/// Result of running the DAG evaluator and then its independent composition post-pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComposedEvaluationResult {
    pub evaluation: EvaluationResult,
    pub composition: CompositionResult,
}

/// Run controller evaluation first, then compose its contributions without
/// performing any host reads or writes.
pub fn evaluate_and_compose<M: ControllerMapper>(
    graph: &ControllerGraph,
    definitions: &[InputDefinition],
    snapshot: &InputSnapshot,
    mapper: &M,
    context: &CompositionContext,
) -> Result<ComposedEvaluationResult, CoreError> {
    let evaluation = evaluate(graph, definitions, snapshot, mapper)?;
    let composition = compose(&evaluation.contributions, context)?;
    Ok(ComposedEvaluationResult { evaluation, composition })
}

/// Compose all contributions for each target in canonical target order.
///
/// Contributions are ordered by (BindingId, ControllerId) before any
/// accumulation. ControllerId is a secondary key because BindingIds are only
/// guaranteed unique within an individual controller.
pub fn compose(
    contributions: &[Contribution],
    context: &CompositionContext,
) -> Result<CompositionResult, CoreError> {
    let mut grouped: BTreeMap<TargetId, Vec<&Contribution>> = BTreeMap::new();
    for contribution in contributions {
        contribution.validate()?;
        grouped.entry(contribution.target_id.clone()).or_default().push(contribution);
    }

    let mut result = CompositionResult::default();
    for (target_id, mut items) in grouped {
        items.sort_by(|left, right| {
            (&left.binding_id, &left.controller_id).cmp(&(&right.binding_id, &right.controller_id))
        });

        let mode = items[0].mode;
        if items.iter().any(|item| item.mode != mode) {
            return Err(conflict(&target_id, "contributions for one target declare different composition modes"));
        }

        let value = match mode {
            CompositionMode::Replace => compose_replace(&target_id, &items)?,
            CompositionMode::Additive => compose_additive(&target_id, &items, context)?,
            CompositionMode::WeightedBlend => compose_weighted_blend(&target_id, &items, &mut result)?,
            CompositionMode::PriorityOverride => compose_priority_override(&items),
        };

        if let Some(value) = value {
            result.outputs.insert(target_id, value);
        }
    }
    Ok(result)
}

fn compose_replace(
    target_id: &TargetId,
    items: &[&Contribution],
) -> Result<Option<TypedValue>, CoreError> {
    if items.len() != 1 {
        return Err(conflict(target_id, "multiple Replace contributions are ambiguous"));
    }
    Ok(Some(items[0].value))
}

fn compose_additive(
    target_id: &TargetId,
    items: &[&Contribution],
    context: &CompositionContext,
) -> Result<Option<TypedValue>, CoreError> {
    let base_ref = items[0].base_ref.as_ref().ok_or_else(|| {
        conflict(target_id, "Additive contribution has no base reference")
    })?;
    if items.iter().any(|item| item.base_ref.as_ref() != Some(base_ref)) {
        return Err(conflict(target_id, "Additive contributions use different base references"));
    }

    let base = match base_ref {
        BaseRef::Neutral => context.neutral_values.get(target_id).copied(),
        BaseRef::Authored => {
            let first = items[0].base_value;
            if items.iter().any(|item| item.base_value != first) {
                return Err(conflict(target_id, "Additive contributions disagree on authored base value"));
            }
            first
        }
        BaseRef::Input(input_id) => context.input_values.get(input_id).copied(),
    }.ok_or_else(|| conflict(target_id, "the explicitly referenced Additive base value is unavailable"))?;

    let mut accumulated = base;
    for item in items {
        ensure_compatible(target_id, accumulated, item.value)?;
        accumulated = add_values(accumulated, item.value)
            .map_err(|reason| conflict(target_id, reason))?;
    }
    Ok(Some(accumulated))
}

fn compose_weighted_blend(
    target_id: &TargetId,
    items: &[&Contribution],
    result: &mut CompositionResult,
) -> Result<Option<TypedValue>, CoreError> {
    // Zero weight is valid but contributes nothing. It is excluded before
    // type compatibility and arithmetic checks.
    let eligible: Vec<&Contribution> = items.iter()
        .copied()
        .filter(|item| item.weight.unwrap_or(0.0) > 0.0)
        .collect();
    if eligible.is_empty() {
        result.unavailable_targets.insert(target_id.clone());
        result.diagnostics.push(
            Diagnostic::new(
                COMPOSITION_NO_ELIGIBLE,
                DiagnosticSeverity::Warning,
                "target has no positive-weight contributions",
            )?.with_target(target_id.clone())
        );
        return Ok(None);
    }

    let mut total_weight = 0.0;
    for item in &eligible {
        let weight = item.weight.expect("validated weighted contribution has a weight");
        total_weight += weight;
        if !total_weight.is_finite() {
            return Err(CoreError::CompositionWeightInvalid { binding_id: item.binding_id.to_string() });
        }
    }
    if total_weight <= 0.0 || !total_weight.is_finite() {
        return Err(CoreError::CompositionWeightInvalid {
            binding_id: eligible[0].binding_id.to_string(),
        });
    }

    let first = eligible[0].value;
    for item in eligible.iter().skip(1) {
        ensure_compatible(target_id, first, item.value)?;
    }

    let value = match first {
        TypedValue::Scalar(_) => {
            let mut sum = 0.0;
            for item in &eligible {
                sum += item.value.as_scalar().expect("type checked")
                    * (item.weight.expect("validated") / total_weight);
            }
            TypedValue::scalar(sum)?
        }
        TypedValue::Vector2(_) => {
            let mut x = 0.0;
            let mut y = 0.0;
            for item in &eligible {
                let vector = item.value.as_vector2().expect("type checked");
                let factor = item.weight.expect("validated") / total_weight;
                x += vector.x() * factor;
                y += vector.y() * factor;
            }
            TypedValue::vector2(x, y)?
        }
        TypedValue::Angle(first_angle) => {
            let angles: Vec<(Angle, f64)> = eligible.iter().map(|item| {
                (item.value.as_angle().expect("type checked"), item.weight.expect("validated"))
            }).collect();
            if angles.iter().any(|(angle, _)| angle.is_wrapped() != first_angle.is_wrapped()) {
                return Err(conflict(target_id, "weighted angle contributions mix wrapped and unwrapped semantics"));
            }
            if first_angle.is_wrapped() {
                let mut sin_sum = 0.0;
                let mut cos_sum = 0.0;
                for (angle, weight) in &angles {
                    let normalized_weight = *weight / total_weight;
                    sin_sum += angle.radians().sin() * normalized_weight;
                    cos_sum += angle.radians().cos() * normalized_weight;
                }
                if sin_sum.hypot(cos_sum) <= 1e-12 {
                    return Err(conflict(target_id, "weighted wrapped-angle mean is undefined for opposing directions"));
                }
                TypedValue::angle(normalize_wrapped_angle(sin_sum.atan2(cos_sum)), true)?
            } else {
                let mut sum = 0.0;
                for (angle, weight) in &angles {
                    sum += angle.radians() * (*weight / total_weight);
                }
                TypedValue::angle(sum, false)?
            }
        }
    };
    Ok(Some(value))
}

fn compose_priority_override(items: &[&Contribution]) -> Option<TypedValue> {
    // Highest numeric priority wins. Equal priorities choose the
    // lexicographically smallest (BindingId, ControllerId) key.
    items.iter().copied().max_by(|left, right| {
        left.priority.cmp(&right.priority)
            .then_with(|| (&right.binding_id, &right.controller_id).cmp(&(&left.binding_id, &left.controller_id)))
    }).map(|item| item.value)
}

fn ensure_compatible(
    target_id: &TargetId,
    left: TypedValue,
    right: TypedValue,
) -> Result<(), CoreError> {
    if left.kind_name() != right.kind_name() {
        return Err(conflict(
            target_id,
            format!("incompatible value types: {} and {}", left.kind_name(), right.kind_name()),
        ));
    }
    if let (TypedValue::Angle(left), TypedValue::Angle(right)) = (left, right) {
        if left.is_wrapped() != right.is_wrapped() {
            return Err(conflict(target_id, "angle contributions mix wrapped and unwrapped semantics"));
        }
    }
    Ok(())
}

fn add_values(left: TypedValue, right: TypedValue) -> Result<TypedValue, &'static str> {
    match (left, right) {
        (TypedValue::Scalar(a), TypedValue::Scalar(b)) => TypedValue::scalar(a.get() + b.get())
            .map_err(|_| "Additive scalar result is non-finite"),
        (TypedValue::Vector2(a), TypedValue::Vector2(b)) => TypedValue::vector2(a.x() + b.x(), a.y() + b.y())
            .map_err(|_| "Additive vector result is non-finite"),
        (TypedValue::Angle(a), TypedValue::Angle(b)) if a.is_wrapped() == b.is_wrapped() => {
            let radians = a.radians() + b.radians();
            let radians = if a.is_wrapped() {
                use std::f64::consts::{PI, TAU};
                (radians + PI).rem_euclid(TAU) - PI
            } else {
                radians
            };
            TypedValue::angle(radians, a.is_wrapped()).map_err(|_| "Additive angle result is non-finite")
        }
        _ => Err("Additive values have incompatible types or angle semantics"),
    }
}

fn normalize_wrapped_angle(radians: f64) -> f64 {
    use std::f64::consts::{PI, TAU};
    (radians + PI).rem_euclid(TAU) - PI
}

fn conflict(target_id: &TargetId, reason: impl Into<String>) -> CoreError {
    CoreError::CompositionConflict {
        target_id: target_id.to_string(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        canonical::{BindingMode, CompositionConfig, Controller, ControllerBinding, MappingConfig, TargetMapping},
        ids::{BindingId, ControllerId, InputId},
        input::{FallbackPolicy, InputDefinition, InputDimension, InputSample, InputSnapshot, InputSource},
    };

    fn id(value: &str) -> TargetId { TargetId::new(value).unwrap() }
    fn controller(value: &str) -> ControllerId { ControllerId::new(value).unwrap() }
    fn binding(value: &str) -> BindingId { BindingId::new(value).unwrap() }
    fn scalar(value: f64) -> TypedValue { TypedValue::scalar(value).unwrap() }

    fn contribution(
        controller_id: &str,
        binding_id: &str,
        value: TypedValue,
        mode: CompositionMode,
    ) -> Contribution {
        Contribution {
            controller_id: controller(controller_id),
            binding_id: binding(binding_id),
            target_id: id("target-a"),
            value,
            mode,
            weight: None,
            priority: None,
            base_ref: None,
            base_value: None,
        }
    }

    fn additive(controller_id: &str, binding_id: &str, value: f64, base: f64) -> Contribution {
        let mut item = contribution(controller_id, binding_id, scalar(value), CompositionMode::Additive);
        item.base_ref = Some(BaseRef::Authored);
        item.base_value = Some(scalar(base));
        item
    }

    fn weighted(controller_id: &str, binding_id: &str, value: f64, weight: f64) -> Contribution {
        let mut item = contribution(controller_id, binding_id, scalar(value), CompositionMode::WeightedBlend);
        item.weight = Some(weight);
        item
    }

    fn priority(controller_id: &str, binding_id: &str, value: f64, priority: i64) -> Contribution {
        let mut item = contribution(controller_id, binding_id, scalar(value), CompositionMode::PriorityOverride);
        item.priority = Some(priority);
        item
    }

    #[test]
    fn evaluate_and_compose_uses_explicit_binding_composition_configuration() {
        let make_controller = |controller_name: &str, input_name: &str, binding_name: &str, weight: f64| {
            Controller {
                id: ControllerId::new(controller_name).unwrap(),
                enabled: true,
                inputs: vec![InputId::new(input_name).unwrap()],
                dependencies: vec![],
                mapping: MappingConfig::Range1D { minimum: -1.0, neutral: 0.0, maximum: 1.0 },
                bindings: vec![ControllerBinding {
                    id: BindingId::new(binding_name).unwrap(),
                    target_id: id("shared-target"),
                    target: TargetMapping::Property {
                        target_id: "arm".into(),
                        property: "rotation".into(),
                        scale: 1.0,
                        additive: false,
                    },
                    mode: BindingMode::AbsolutePose,
                    base_value: None,
                    composition: Some(CompositionConfig {
                        mode: CompositionMode::WeightedBlend,
                        weight: Some(weight),
                        priority: None,
                        base_ref: None,
                    }),
                }],
                parent_scoped: false,
            }
        };
        let graph = ControllerGraph::new(vec![
            make_controller("controller-a", "input-a", "binding-a", 1.0),
            make_controller("controller-b", "input-b", "binding-b", 3.0),
        ]).unwrap();
        let definitions = vec![
            InputDefinition {
                id: InputId::new("input-a").unwrap(),
                source: InputSource::Manual,
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            },
            InputDefinition {
                id: InputId::new("input-b").unwrap(),
                source: InputSource::Manual,
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            },
        ];
        let snapshot = InputSnapshot::new(5, vec![
            InputSample::valid(InputId::new("input-a").unwrap(), scalar(10.0), Some(5)),
            InputSample::valid(InputId::new("input-b").unwrap(), scalar(20.0), Some(5)),
        ]).unwrap();
        let mapper = |controller: &Controller, inputs: &crate::evaluator::ResolvedInputs| {
            Ok(BTreeMap::from([(
                controller.bindings[0].id.clone(),
                inputs.value(&controller.inputs[0]).unwrap(),
            )]))
        };
        let result = evaluate_and_compose(
            &graph,
            &definitions,
            &snapshot,
            &mapper,
            &CompositionContext::default(),
        ).unwrap();
        assert_eq!(result.evaluation.contributions.len(), 2);
        assert_eq!(
            result.composition.outputs.get(&id("shared-target")),
            Some(&scalar(17.5)),
        );
    }

    #[test]
    fn evaluate_and_compose_preserves_angle_typed_property_values() {
        let mut controller = Controller {
            id: ControllerId::new("angle-controller").unwrap(),
            enabled: true,
            inputs: vec![InputId::new("angle-input").unwrap()],
            dependencies: vec![],
            mapping: MappingConfig::Range1D { minimum: -1.0, neutral: 0.0, maximum: 1.0 },
            bindings: vec![ControllerBinding {
                id: BindingId::new("angle-binding").unwrap(),
                target_id: id("angle-target"),
                target: TargetMapping::Property {
                    target_id: "arm".into(),
                    property: "rotation".into(),
                    scale: 1.0,
                    additive: false,
                },
                mode: BindingMode::AbsolutePose,
                base_value: None,
                composition: None,
            }],
            parent_scoped: false,
        };
        controller.bindings[0].composition = Some(CompositionConfig {
            mode: CompositionMode::WeightedBlend,
            weight: Some(1.0),
            priority: None,
            base_ref: None,
        });
        let graph = ControllerGraph::new(vec![controller]).unwrap();
        let definitions = vec![InputDefinition {
            id: InputId::new("angle-input").unwrap(),
            source: InputSource::Manual,
            dimension: InputDimension::Scalar,
            fallback: FallbackPolicy::Reject,
        }];
        let snapshot = InputSnapshot::new(1, vec![
            InputSample::valid(InputId::new("angle-input").unwrap(), scalar(0.0), Some(1)),
        ]).unwrap();
        let mapper = |controller: &Controller, _: &crate::evaluator::ResolvedInputs| {
            Ok(BTreeMap::from([(
                controller.bindings[0].id.clone(),
                TypedValue::angle(std::f64::consts::PI, true)?,
            )]))
        };
        let result = evaluate_and_compose(
            &graph,
            &definitions,
            &snapshot,
            &mapper,
            &CompositionContext::default(),
        ).unwrap();
        assert_eq!(
            result.composition.outputs.get(&id("angle-target")),
            Some(&TypedValue::angle(-std::f64::consts::PI, true).unwrap()),
        );
    }

    #[test]
    fn replace_emits_the_single_contribution() {
        let item = contribution("controller-a", "binding-a", scalar(4.0), CompositionMode::Replace);
        let result = compose(&[item], &CompositionContext::default()).unwrap();
        assert_eq!(result.outputs.get(&id("target-a")), Some(&scalar(4.0)));
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn multiple_replace_contributions_conflict_instead_of_overwriting() {
        let items = vec![
            contribution("controller-a", "binding-a", scalar(1.0), CompositionMode::Replace),
            contribution("controller-b", "binding-b", scalar(2.0), CompositionMode::Replace),
        ];
        assert!(matches!(compose(&items, &CompositionContext::default()), Err(CoreError::CompositionConflict { .. })));
    }

    #[test]
    fn mixed_modes_for_one_target_are_a_typed_conflict() {
        let items = vec![
            contribution("controller-a", "binding-a", scalar(1.0), CompositionMode::Replace),
            additive("controller-b", "binding-b", 2.0, 0.0),
        ];
        assert!(matches!(compose(&items, &CompositionContext::default()), Err(CoreError::CompositionConflict { .. })));
    }

    #[test]
    fn additive_accumulates_in_canonical_binding_order_from_authored_base() {
        let items = vec![
            additive("controller-b", "binding-b", 0.2, 10.0),
            additive("controller-a", "binding-a", 0.3, 10.0),
        ];
        let result = compose(&items, &CompositionContext::default()).unwrap();
        assert_eq!(result.outputs.get(&id("target-a")), Some(&scalar(10.5)));
    }

    #[test]
    fn additive_supports_neutral_and_explicit_input_bases() {
        let mut neutral = contribution("controller-a", "binding-a", scalar(2.0), CompositionMode::Additive);
        neutral.base_ref = Some(BaseRef::Neutral);
        let mut input = contribution("controller-b", "binding-b", scalar(3.0), CompositionMode::Additive);
        input.base_ref = Some(BaseRef::Input("input-base".into()));
        let neutral_result = compose(&[neutral], &CompositionContext {
            neutral_values: BTreeMap::from([(id("target-a"), scalar(10.0))]),
            input_values: BTreeMap::new(),
        }).unwrap();
        assert_eq!(neutral_result.outputs.get(&id("target-a")), Some(&scalar(12.0)));
        let input_result = compose(&[input], &CompositionContext {
            neutral_values: BTreeMap::new(),
            input_values: BTreeMap::from([("input-base".into(), scalar(20.0))]),
        }).unwrap();
        assert_eq!(input_result.outputs.get(&id("target-a")), Some(&scalar(23.0)));
    }

    #[test]
    fn additive_requires_a_single_available_compatible_base() {
        let items = vec![
            additive("controller-a", "binding-a", 1.0, 10.0),
            additive("controller-b", "binding-b", 2.0, 11.0),
        ];
        assert!(matches!(compose(&items, &CompositionContext::default()), Err(CoreError::CompositionConflict { .. })));
        let mut missing = contribution("controller-a", "binding-a", scalar(1.0), CompositionMode::Additive);
        missing.base_ref = Some(BaseRef::Neutral);
        assert!(matches!(compose(&[missing], &CompositionContext::default()), Err(CoreError::CompositionConflict { .. })));
    }

    #[test]
    fn weighted_blend_normalizes_positive_weights() {
        let items = vec![
            weighted("controller-a", "binding-a", 10.0, 1.0),
            weighted("controller-b", "binding-b", 20.0, 3.0),
        ];
        let result = compose(&items, &CompositionContext::default()).unwrap();
        assert_eq!(result.outputs.get(&id("target-a")), Some(&scalar(17.5)));
    }

    #[test]
    fn zero_weights_are_excluded_and_all_zero_produces_diagnostic_without_output() {
        let items = vec![
            weighted("controller-a", "binding-a", 10.0, 0.0),
            weighted("controller-b", "binding-b", 20.0, 0.0),
        ];
        let result = compose(&items, &CompositionContext::default()).unwrap();
        assert!(!result.outputs.contains_key(&id("target-a")));
        assert!(result.unavailable_targets.contains(&id("target-a")));
        assert!(result.diagnostics.iter().any(|item| item.code == COMPOSITION_NO_ELIGIBLE));
    }

    #[test]
    fn invalid_negative_or_non_finite_weights_are_typed_errors() {
        for weight in [-1.0, f64::NAN, f64::INFINITY] {
            let item = weighted("controller-a", "binding-a", 10.0, weight);
            assert!(matches!(compose(&[item], &CompositionContext::default()), Err(CoreError::CompositionWeightInvalid { .. })));
        }
    }

    #[test]
    fn weighted_blend_rejects_incompatible_types_and_undefined_wrapped_angle_mean() {
        let mut vector = contribution("controller-b", "binding-b", TypedValue::vector2(1.0, 2.0).unwrap(), CompositionMode::WeightedBlend);
        vector.weight = Some(1.0);
        let scalar = weighted("controller-a", "binding-a", 1.0, 1.0);
        assert!(matches!(compose(&[scalar, vector], &CompositionContext::default()), Err(CoreError::CompositionConflict { .. })));

        let mut positive = contribution("controller-a", "binding-a", TypedValue::angle(0.0, true).unwrap(), CompositionMode::WeightedBlend);
        positive.weight = Some(1.0);
        let mut negative = contribution("controller-b", "binding-b", TypedValue::angle(std::f64::consts::PI, true).unwrap(), CompositionMode::WeightedBlend);
        negative.weight = Some(1.0);
        assert!(matches!(compose(&[positive, negative], &CompositionContext::default()), Err(CoreError::CompositionConflict { .. })));
    }

    #[test]
    fn priority_override_uses_highest_priority_then_canonical_binding_id() {
        let items = vec![
            priority("controller-a", "binding-z", 1.0, 5),
            priority("controller-b", "binding-a", 2.0, 5),
            priority("controller-c", "binding-c", 3.0, 4),
        ];
        let result = compose(&items, &CompositionContext::default()).unwrap();
        assert_eq!(result.outputs.get(&id("target-a")), Some(&scalar(2.0)));
    }

    #[test]
    fn duplicate_binding_ids_are_still_deterministic_across_controllers() {
        let items = vec![
            priority("controller-b", "same-binding", 1.0, 7),
            priority("controller-a", "same-binding", 2.0, 7),
        ];
        let result = compose(&items, &CompositionContext::default()).unwrap();
        assert_eq!(result.outputs.get(&id("target-a")), Some(&scalar(2.0)));
    }

    #[test]
    fn additive_vector_and_wrapped_angle_values_are_supported() {
        let mut vector = contribution("controller-a", "binding-a", TypedValue::vector2(1.0, -2.0).unwrap(), CompositionMode::Additive);
        vector.base_ref = Some(BaseRef::Authored);
        vector.base_value = Some(TypedValue::vector2(10.0, 20.0).unwrap());
        let vector_result = compose(&[vector], &CompositionContext::default()).unwrap();
        assert_eq!(vector_result.outputs.get(&id("target-a")), Some(&TypedValue::vector2(11.0, 18.0).unwrap()));

        let mut angle = contribution("controller-a", "binding-a", TypedValue::angle(0.5, true).unwrap(), CompositionMode::Additive);
        angle.base_ref = Some(BaseRef::Authored);
        angle.base_value = Some(TypedValue::angle(3.0, true).unwrap());
        let angle_result = compose(&[angle], &CompositionContext::default()).unwrap();
        assert_eq!(angle_result.outputs.get(&id("target-a")), Some(&TypedValue::angle(3.5 - std::f64::consts::TAU, true).unwrap()));
    }

    #[test]
    fn output_is_independent_of_input_contribution_order() {
        let first = vec![
            weighted("controller-b", "binding-b", 10.0, 1.0),
            weighted("controller-a", "binding-a", 20.0, 2.0),
        ];
        let second = vec![first[1].clone(), first[0].clone()];
        assert_eq!(
            compose(&first, &CompositionContext::default()).unwrap(),
            compose(&second, &CompositionContext::default()).unwrap(),
        );
    }
}
