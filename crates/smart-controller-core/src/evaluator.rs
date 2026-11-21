//! Deterministic, host-independent DAG evaluation orchestration.
//!
//! Mapping is injected through `ControllerMapper`: Phase 3 owns snapshot
//! preflight, dependency ordering, unavailable-output propagation and
//! contribution collection. Mapping algorithms remain independently testable
//! and can be supplied by the Phase 2 mapping layer.

use crate::{
    canonical::{BindingMode, Controller, TargetMapping},
    contribution::{BaseRef, CompositionMode, Contribution},
    diagnostic_codes::{
        Diagnostic, DiagnosticSeverity, CONTROLLER_DISABLED, INPUT_INVALID, INPUT_MISSING,
        INPUT_STALE, INPUT_UNAVAILABLE, MAPPING_TYPE, OUTPUT_SKIPPED,
    },
    error::CoreError,
    graph::ControllerGraph,
    ids::{BindingId, ControllerId, InputId},
    input::{FallbackPolicy, Freshness, InputDefinition, InputSnapshot, InputSource, InputValidity},
    value::TypedValue,
};
use std::collections::{BTreeMap, BTreeSet};

/// A resolved input is either a usable typed value or explicitly unavailable.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedInput {
    pub value: Option<TypedValue>,
    pub source: InputSource,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResolvedInputs {
    values: BTreeMap<InputId, ResolvedInput>,
}

impl ResolvedInputs {
    pub fn get(&self, id: &InputId) -> Option<&ResolvedInput> {
        self.values.get(id)
    }

    pub fn value(&self, id: &InputId) -> Option<TypedValue> {
        self.values.get(id).and_then(|input| input.value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&InputId, &ResolvedInput)> {
        self.values.iter()
    }
}

/// The mapping layer evaluates one controller from its resolved input values.
/// It returns values keyed by the controller's canonical BindingIds.
/// It must not mutate host state or compose target values.
pub trait ControllerMapper {
    fn evaluate(
        &self,
        controller: &Controller,
        inputs: &ResolvedInputs,
    ) -> Result<BTreeMap<BindingId, TypedValue>, CoreError>;
}

impl<F> ControllerMapper for F
where
    F: Fn(&Controller, &ResolvedInputs) -> Result<BTreeMap<BindingId, TypedValue>, CoreError>,
{
    fn evaluate(
        &self,
        controller: &Controller,
        inputs: &ResolvedInputs,
    ) -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
        self(controller, inputs)
    }
}

/// Outputs from Phase 3 are uncomposed contributions. Target composition is
/// deliberately a separate Phase 4 post-pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvaluationResult {
    pub contributions: Vec<Contribution>,
    pub diagnostics: Vec<Diagnostic>,
    /// Binding outputs absent because their controller was disabled, skipped,
    /// failed mapping, or did not emit that binding.
    pub unavailable_outputs: BTreeSet<(ControllerId, BindingId)>,
}

#[derive(Clone, Debug)]
struct PreflightInput {
    definition: InputDefinition,
    resolved: Option<TypedValue>,
}

fn diagnostic(
    code: &str,
    severity: DiagnosticSeverity,
    message: impl Into<String>,
    controller_id: Option<ControllerId>,
    input_id: Option<InputId>,
) -> Result<Diagnostic, CoreError> {
    let mut item = Diagnostic::new(code, severity, message)?;
    item.controller_id = controller_id;
    item.input_id = input_id;
    Ok(item)
}

/// Execute the four Phase 3 stages:
/// 1. Validate the graph and resolve the complete external input snapshot.
/// 2. Evaluate enabled nodes in deterministic topological order.
/// 3. Collect successful binding contributions.
/// 4. Return those contributions without applying target composition.
///
/// The mapper is called only after the entire snapshot and every declared
/// dependency/input reference have passed preflight.
pub fn evaluate<M: ControllerMapper>(
    graph: &ControllerGraph,
    definitions: &[InputDefinition],
    snapshot: &InputSnapshot,
    mapper: &M,
) -> Result<EvaluationResult, CoreError> {
    // Stage 1a: graph, snapshot, definitions and all references are validated
    // before the first call into the mapping layer.
    graph.validate()?;
    snapshot.validate()?;
    for (_, controller) in graph.iter() {
        controller.validate_local()?;
    }

    let mut definition_map = BTreeMap::new();
    for definition in definitions {
        definition.validate()?;
        if definition_map.insert(definition.id.clone(), definition.clone()).is_some() {
            return Err(CoreError::DuplicateInputId(definition.id.to_string()));
        }
    }

    for (_, controller) in graph.iter() {
        for input_id in &controller.inputs {
            if !definition_map.contains_key(input_id) {
                return Err(CoreError::MissingInput(input_id.to_string()));
            }
        }
    }

    // Every supplied sample must have a declared definition. This prevents
    // unnoticed or ambiguous values from entering an evaluation snapshot.
    for sample in &snapshot.samples {
        let definition = definition_map.get(&sample.id).ok_or_else(|| {
            CoreError::InvalidInputSample {
                input_id: sample.id.to_string(),
                reason: "snapshot contains a sample with no input definition",
            }
        })?;
        if matches!(&definition.source, InputSource::ControllerOutput { .. }) {
            return Err(CoreError::InvalidInputSample {
                input_id: sample.id.to_string(),
                reason: "controller-output inputs must be resolved from the dependency graph, not the external snapshot",
            });
        }
        definition.validate_sample(sample)?;
    }

    let active_input_ids: BTreeSet<InputId> = graph.iter()
        .filter(|(_, controller)| controller.enabled)
        .flat_map(|(_, controller)| controller.inputs.iter().cloned())
        .collect();

    for (_, controller) in graph.iter() {
        for input_id in &controller.inputs {
            let definition = &definition_map[input_id];
            if let InputSource::ControllerOutput { controller_id, binding_id } = &definition.source {
                let producer = graph.get(controller_id).ok_or_else(|| {
                    CoreError::InvalidGraph(format!(
                        "input {} references missing controller {}",
                        input_id, controller_id
                    ))
                })?;
                if !controller.dependencies.contains(controller_id) {
                    return Err(CoreError::InvalidGraph(format!(
                        "controller {} consumes output from {} without declaring that dependency",
                        controller.id, controller_id
                    )));
                }
                if !producer.bindings.iter().any(|binding| &binding.id == binding_id) {
                    return Err(CoreError::InvalidGraph(format!(
                        "input {} references missing output binding {} on controller {}",
                        input_id, binding_id, controller_id
                    )));
                }
            }
        }
    }

    // Resolve every non-controller input now. Controller-output inputs are
    // resolved just-in-time after their producer has run.
    let mut preflight = BTreeMap::new();
    let mut diagnostics = Vec::new();
    for (id, definition) in &definition_map {
        if !active_input_ids.contains(id) {
            continue;
        }
        if matches!(&definition.source, InputSource::ControllerOutput { .. }) {
            preflight.insert(id.clone(), PreflightInput {
                definition: definition.clone(),
                resolved: None,
            });
            continue;
        }
        let sample = snapshot.get(id);
        let (resolved, issue) = resolve_external(definition, sample, snapshot)?;
        if let Some((code, message)) = issue {
            diagnostics.push(diagnostic(
                code,
                DiagnosticSeverity::Warning,
                message,
                None,
                Some(id.clone()),
            )?);
        }
        preflight.insert(id.clone(), PreflightInput {
            definition: definition.clone(),
            resolved,
        });
    }

    // Stage 2: deterministic node evaluation. topological_order is called
    // independently of graph construction as evaluator-side cycle defence.
    let order = graph.topological_order()?;
    let mut outputs: BTreeMap<(ControllerId, BindingId), TypedValue> = BTreeMap::new();
    let mut result = EvaluationResult::default();
    result.diagnostics = diagnostics;

    for controller_id in order {
        let controller = graph.get(&controller_id).ok_or_else(|| {
            CoreError::InvalidGraph(format!("topological order references missing node {controller_id}"))
        })?;

        if !controller.enabled {
            result.diagnostics.push(diagnostic(
                CONTROLLER_DISABLED,
                DiagnosticSeverity::Info,
                format!("controller {controller_id} is disabled"),
                Some(controller_id.clone()),
                None,
            )?);
            for binding in &controller.bindings {
                result.unavailable_outputs.insert((controller_id.clone(), binding.id.clone()));
                result.diagnostics.push(diagnostic(
                    OUTPUT_SKIPPED,
                    DiagnosticSeverity::Info,
                    format!("binding {} was skipped because its controller is disabled", binding.id),
                    Some(controller_id.clone()),
                    None,
                )?);
            }
            continue;
        }

        let mut resolved_inputs = ResolvedInputs::default();
        let mut unavailable_required = None;
        for input_id in &controller.inputs {
            let item = preflight.get(input_id).ok_or_else(|| CoreError::MissingInput(input_id.to_string()))?;
            let value = if let InputSource::ControllerOutput { controller_id: producer, binding_id } = &item.definition.source {
                match outputs.get(&(producer.clone(), binding_id.clone())).copied() {
                    Some(value) => Some(value),
                    None => match &item.definition.fallback {
                        FallbackPolicy::UseValue(value) => {
                            result.diagnostics.push(diagnostic(
                                INPUT_UNAVAILABLE,
                                DiagnosticSeverity::Warning,
                                format!("controller output {}:{} is unavailable; explicit fallback used", producer, binding_id),
                                Some(controller_id.clone()),
                                Some(input_id.clone()),
                            )?);
                            Some(*value)
                        }
                        FallbackPolicy::Unavailable => None,
                        FallbackPolicy::Reject => {
                            return Err(CoreError::MissingInput(format!(
                                "controller output {}:{} for input {}",
                                producer, binding_id, input_id
                            )));
                        }
                    }
                }
            } else {
                item.resolved.or_else(|| apply_fallback(&item.definition.fallback))
            };
            if value.is_none() {
                unavailable_required = Some(input_id.clone());
                break;
            }
            if let Some(value) = value {
                if !item.definition.dimension.accepts(value) {
                    result.diagnostics.push(diagnostic(
                        MAPPING_TYPE,
                        DiagnosticSeverity::Error,
                        format!("input {input_id} resolved to {}, expected {}", value.kind_name(), item.definition.dimension.name()),
                        Some(controller_id.clone()),
                        Some(input_id.clone()),
                    )?);
                    unavailable_required = Some(input_id.clone());
                    break;
                }
                resolved_inputs.values.insert(input_id.clone(), ResolvedInput {
                    value: Some(value),
                    source: item.definition.source.clone(),
                });
            }
        }

        if let Some(input_id) = unavailable_required {
            result.diagnostics.push(diagnostic(
                INPUT_UNAVAILABLE,
                DiagnosticSeverity::Warning,
                format!("controller {controller_id} was skipped because required input {input_id} is unavailable"),
                Some(controller_id.clone()),
                Some(input_id),
            )?);
            mark_controller_outputs_unavailable(
                controller,
                &controller_id,
                &mut result,
                "required input was unavailable",
            )?;
            continue;
        }

        let mapped = match mapper.evaluate(controller, &resolved_inputs) {
            Ok(values) => values,
            Err(error) if is_controller_mapping_failure(&error) => {
                result.diagnostics.push(diagnostic(
                    crate::diagnostic_codes::MAPPING_DOMAIN,
                    DiagnosticSeverity::Error,
                    error.to_string(),
                    Some(controller_id.clone()),
                    None,
                )?);
                mark_controller_outputs_unavailable(
                    controller,
                    &controller_id,
                    &mut result,
                    "mapping failed",
                )?;
                continue;
            }
            Err(error) => return Err(error),
        };

        for binding_id in mapped.keys() {
            if !controller.bindings.iter().any(|binding| &binding.id == binding_id) {
                return Err(CoreError::Evaluation(format!(
                    "mapper emitted unknown binding {binding_id} for controller {controller_id}"
                )));
            }
        }

        // Stage 3: collect successful binding contributions. No target is
        // composed or written here; Phase 4 owns that post-pass.
        for binding in &controller.bindings {
            let Some(value) = mapped.get(&binding.id).copied() else {
                result.unavailable_outputs.insert((controller_id.clone(), binding.id.clone()));
                result.diagnostics.push(diagnostic(
                    OUTPUT_SKIPPED,
                    DiagnosticSeverity::Warning,
                    format!("mapper did not emit binding {}", binding.id),
                    Some(controller_id.clone()),
                    None,
                )?);
                continue;
            };
            if !target_accepts_value(&binding.target, value) {
                result.diagnostics.push(diagnostic(
                    MAPPING_TYPE,
                    DiagnosticSeverity::Error,
                    format!("binding {} produced {} for an incompatible target", binding.id, value.kind_name()),
                    Some(controller_id.clone()),
                    None,
                )?);
                result.unavailable_outputs.insert((controller_id.clone(), binding.id.clone()));
                result.diagnostics.push(diagnostic(
                    OUTPUT_SKIPPED,
                    DiagnosticSeverity::Warning,
                    format!("binding {} was skipped because its output type is incompatible with its target", binding.id),
                    Some(controller_id.clone()),
                    None,
                )?);
                continue;
            }

            let (mode, weight, priority, base_ref) = if let Some(config) = &binding.composition {
                (config.mode, config.weight, config.priority, config.base_ref.clone())
            } else {
                match binding.mode {
                    BindingMode::AbsolutePose => (CompositionMode::Replace, None, None, None),
                    BindingMode::RelativeDelta => (CompositionMode::Additive, None, None, Some(BaseRef::Authored)),
                }
            };
            let contribution = Contribution {
                controller_id: controller_id.clone(),
                binding_id: binding.id.clone(),
                target_id: binding.target_id.clone(),
                value,
                mode,
                weight,
                priority,
                base_ref,
                base_value: if mode == CompositionMode::Additive && matches!(base_ref.as_ref(), Some(BaseRef::Authored)) { binding.base_value } else { None },
            };
            contribution.validate()?;
            outputs.insert((controller_id.clone(), binding.id.clone()), value);
            result.contributions.push(contribution);
        }
    }

    // Preserve deterministic collection order independent of authored map
    // iteration. Stable sorting retains topological order for equal BindingIds.
    result.contributions.sort_by(|left, right| (&left.binding_id, &left.controller_id).cmp(&(&right.binding_id, &right.controller_id)));
    Ok(result)
}

fn resolve_external(
    definition: &InputDefinition,
    sample: Option<&crate::input::InputSample>,
    snapshot: &InputSnapshot,
) -> Result<(Option<TypedValue>, Option<(&'static str, String)>), CoreError> {
    let Some(sample) = sample else {
        return fallback_or_reject(
            definition,
            INPUT_MISSING,
            format!("input {} has no sample", definition.id),
            CoreError::MissingInput(definition.id.to_string()),
        );
    };
    definition.validate_sample(sample)?;

    let validity_issue = match sample.validity {
        InputValidity::Valid => None,
        InputValidity::Missing => Some((INPUT_MISSING, "sample is marked missing")),
        InputValidity::Invalid => Some((INPUT_INVALID, "sample is marked invalid")),
        InputValidity::Unavailable => Some((INPUT_UNAVAILABLE, "sample is marked unavailable")),
    };
    if let Some((code, reason)) = validity_issue {
        return fallback_or_reject(
            definition,
            code,
            format!("input {} {reason}", definition.id),
            CoreError::MissingInput(definition.id.to_string()),
        );
    }

    match snapshot.freshness(sample) {
        Freshness::Fresh => Ok((sample.value, None)),
        Freshness::Stale => fallback_or_reject(
            definition,
            INPUT_STALE,
            format!("input {} is stale", definition.id),
            CoreError::StaleInput {
                input_id: definition.id.to_string(),
                age_ticks: sample.age_ticks.unwrap_or_default(),
                max_age_ticks: snapshot.max_age_ticks(),
            },
        ),
        Freshness::Unknown => fallback_or_reject(
            definition,
            INPUT_UNAVAILABLE,
            format!("input {} freshness is unknown", definition.id),
            CoreError::Evaluation(format!("input {} freshness is unknown", definition.id)),
        ),
    }
}

fn fallback_or_reject(
    definition: &InputDefinition,
    code: &'static str,
    message: String,
    error: CoreError,
) -> Result<(Option<TypedValue>, Option<(&'static str, String)>), CoreError> {
    match &definition.fallback {
        FallbackPolicy::UseValue(value) => Ok((Some(*value), Some((code, format!("{message}; explicit fallback used"))))),
        FallbackPolicy::Unavailable => Ok((None, Some((code, message)))),
        FallbackPolicy::Reject => Err(error),
    }
}

fn is_controller_mapping_failure(error: &CoreError) -> bool {
    matches!(
        error,
        CoreError::InvalidMapping(_) | CoreError::NonFiniteValue { .. }
    )
}

fn apply_fallback(policy: &FallbackPolicy) -> Option<TypedValue> {
    match policy {
        FallbackPolicy::UseValue(value) => Some(*value),
        FallbackPolicy::Unavailable | FallbackPolicy::Reject => None,
    }
}

fn mark_controller_outputs_unavailable(
    controller: &Controller,
    controller_id: &ControllerId,
    result: &mut EvaluationResult,
    reason: &str,
) -> Result<(), CoreError> {
    for binding in &controller.bindings {
        result.unavailable_outputs.insert((controller_id.clone(), binding.id.clone()));
        result.diagnostics.push(diagnostic(
            OUTPUT_SKIPPED,
            DiagnosticSeverity::Warning,
            format!("binding {} was skipped because {reason}", binding.id),
            Some(controller_id.clone()),
            None,
        )?);
    }
    Ok(())
}

fn target_accepts_value(target: &TargetMapping, value: TypedValue) -> bool {
    match target {
        TargetMapping::SymbolFrame { .. } | TargetMapping::FrameRange { .. } | TargetMapping::Property { .. } =>
            matches!(value, TypedValue::Scalar(_)),
        TargetMapping::GeometryDeform { .. } => matches!(value, TypedValue::Vector2(_)),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        canonical::{ControllerBinding, MappingConfig},
        ids::TargetId,
        input::{InputDimension, InputSample},
    };
    use std::{cell::Cell, collections::BTreeMap};

    fn input_id(value: &str) -> InputId { InputId::new(value).unwrap() }
    fn controller_id(value: &str) -> ControllerId { ControllerId::new(value).unwrap() }
    fn binding_id(value: &str) -> BindingId { BindingId::new(value).unwrap() }

    fn binding(id: &str) -> ControllerBinding {
        ControllerBinding {
            id: binding_id(id),
            target_id: TargetId::new(format!("target-{id}")).unwrap(),
            target: TargetMapping::Property {
                target_id: format!("property-{id}"),
                property: "value".into(),
                scale: 1.0,
                additive: false,
            },
            mode: BindingMode::AbsolutePose,
            base_value: None,
        }
    }

    fn node(id: &str, inputs: Vec<InputId>, dependencies: Vec<ControllerId>, enabled: bool, binding_name: &str) -> Controller {
        Controller {
            id: controller_id(id),
            enabled,
            inputs,
            dependencies,
            mapping: MappingConfig::Range1D { minimum: -1.0, neutral: 0.0, maximum: 1.0 },
            bindings: vec![binding(binding_name)],
            parent_scoped: false,
        }
    }

    fn manual_definition(id: &str, fallback: FallbackPolicy) -> InputDefinition {
        InputDefinition {
            id: input_id(id),
            source: InputSource::Manual,
            dimension: InputDimension::Scalar,
            fallback,
        }
    }

    fn snapshot(id: &str, value: f64) -> InputSnapshot {
        InputSnapshot::new(5, vec![InputSample::valid(
            input_id(id),
            TypedValue::scalar(value).unwrap(),
            Some(5),
        )]).unwrap()
    }

    #[test]
    fn evaluates_nodes_in_order_and_returns_uncomposed_contributions() {
        let graph = ControllerGraph::new(vec![node("controller-a", vec![input_id("input-a")], vec![], true, "binding-a")]).unwrap();
        let definitions = vec![manual_definition("input-a", FallbackPolicy::Reject)];
        let mapper = |controller: &Controller, inputs: &ResolvedInputs| {
            Ok(BTreeMap::from([(
                controller.bindings[0].id.clone(),
                inputs.value(&controller.inputs[0]).unwrap(),
            )]))
        };
        let result = evaluate(&graph, &definitions, &snapshot("input-a", 0.75), &mapper).unwrap();
        assert_eq!(result.contributions.len(), 1);
        assert_eq!(result.contributions[0].value.as_scalar(), Some(0.75));
        assert_eq!(result.contributions[0].mode, CompositionMode::Replace);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn complete_input_preflight_happens_before_any_node_evaluation() {
        let graph = ControllerGraph::new(vec![
            node("a", vec![input_id("input-a")], vec![], true, "binding-a"),
            node("b", vec![input_id("input-b")], vec![], true, "binding-b"),
        ]).unwrap();
        let definitions = vec![
            manual_definition("input-a", FallbackPolicy::Reject),
            manual_definition("input-b", FallbackPolicy::Reject),
        ];
        let calls = Cell::new(0);
        let mapper = |_: &Controller, _: &ResolvedInputs| {
            calls.set(calls.get() + 1);
            Ok(BTreeMap::new())
        };
        let result = evaluate(&graph, &definitions, &snapshot("input-a", 0.5), &mapper);
        assert!(result.is_err());
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn controller_output_inputs_follow_declared_dependencies() {
        let upstream = node("upstream", vec![input_id("manual")], vec![], true, "up-binding");
        let downstream = node(
            "downstream",
            vec![input_id("from-upstream")],
            vec![controller_id("upstream")],
            true,
            "down-binding",
        );
        let graph = ControllerGraph::new(vec![downstream, upstream]).unwrap();
        let definitions = vec![
            manual_definition("manual", FallbackPolicy::Reject),
            InputDefinition {
                id: input_id("from-upstream"),
                source: InputSource::ControllerOutput {
                    controller_id: controller_id("upstream"),
                    binding_id: binding_id("up-binding"),
                },
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            },
        ];
        let mapper = |controller: &Controller, inputs: &ResolvedInputs| {
            let input = inputs.value(&controller.inputs[0]).unwrap().as_scalar().unwrap();
            Ok(BTreeMap::from([(
                controller.bindings[0].id.clone(),
                TypedValue::scalar(input + 1.0).unwrap(),
            )]))
        };
        let result = evaluate(&graph, &definitions, &snapshot("manual", 2.0), &mapper).unwrap();
        assert_eq!(result.contributions.len(), 2);
        let downstream_value = result.contributions.iter()
            .find(|item| item.binding_id == binding_id("down-binding"))
            .unwrap().value.as_scalar();
        assert_eq!(downstream_value, Some(4.0));
    }

    #[test]
    fn disabled_controller_output_is_unavailable_not_a_cached_value() {
        let upstream = node("upstream", vec![input_id("manual")], vec![], false, "up-binding");
        let mut downstream = node(
            "downstream",
            vec![input_id("from-upstream")],
            vec![controller_id("upstream")],
            true,
            "down-binding",
        );
        downstream.bindings[0].target = TargetMapping::Property {
            target_id: "downstream-property".into(),
            property: "value".into(),
            scale: 1.0,
            additive: true,
        };
        downstream.bindings[0].mode = BindingMode::RelativeDelta;
        downstream.bindings[0].base_value = Some(TypedValue::scalar(0.0).unwrap());
        let graph = ControllerGraph::new(vec![downstream, upstream]).unwrap();
        let definitions = vec![
            manual_definition("manual", FallbackPolicy::Reject),
            InputDefinition {
                id: input_id("from-upstream"),
                source: InputSource::ControllerOutput {
                    controller_id: controller_id("upstream"),
                    binding_id: binding_id("up-binding"),
                },
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Unavailable,
            },
        ];
        let mapper = |_: &Controller, _: &ResolvedInputs| -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
            panic!("downstream must not evaluate without its required input")
        };
        let result = evaluate(&graph, &definitions, &snapshot("manual", 1.0), &mapper).unwrap();
        assert!(result.contributions.is_empty());
        assert!(result.unavailable_outputs.contains(&(controller_id("upstream"), binding_id("up-binding"))));
        assert!(result.diagnostics.iter().any(|item| item.code == INPUT_UNAVAILABLE));
    }

    #[test]
    fn empty_graph_is_a_clean_no_op() {
        let graph = ControllerGraph::new(vec![]).unwrap();
        let snapshot = InputSnapshot::new(0, vec![]).unwrap();
        let mapper = |_: &Controller, _: &ResolvedInputs| -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
            panic!("empty graph must not invoke mapper")
        };
        let result = evaluate(&graph, &[], &snapshot, &mapper).unwrap();
        assert!(result.contributions.is_empty());
        assert!(result.diagnostics.is_empty());
        assert!(result.unavailable_outputs.is_empty());
    }

    #[test]
    fn stale_input_uses_only_the_explicit_fallback() {
        let graph = ControllerGraph::new(vec![node("a", vec![input_id("input-a")], vec![], true, "binding-a")]).unwrap();
        let definitions = vec![manual_definition(
            "input-a",
            FallbackPolicy::UseValue(TypedValue::scalar(0.25).unwrap()),
        )];
        let stale = InputSnapshot::new(4, vec![InputSample::valid(
            input_id("input-a"),
            TypedValue::scalar(99.0).unwrap(),
            Some(5),
        )]).unwrap();
        let mapper = |controller: &Controller, inputs: &ResolvedInputs| {
            Ok(BTreeMap::from([(
                controller.bindings[0].id.clone(),
                inputs.value(&controller.inputs[0]).unwrap(),
            )]))
        };
        let result = evaluate(&graph, &definitions, &stale, &mapper).unwrap();
        assert_eq!(result.contributions[0].value.as_scalar(), Some(0.25));
        assert!(result.diagnostics.iter().any(|item| item.code == INPUT_STALE));
    }

    #[test]
    fn all_disabled_controllers_produce_only_disabled_and_skipped_diagnostics() {
        let graph = ControllerGraph::new(vec![
            node("disabled", vec![input_id("input-a")], vec![], false, "binding-a"),
        ]).unwrap();
        let definitions = vec![manual_definition("input-a", FallbackPolicy::Reject)];
        let mapper = |_: &Controller, _: &ResolvedInputs| -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
            panic!("disabled controller must not invoke mapper")
        };
        let result = evaluate(
            &graph,
            &definitions,
            &InputSnapshot::new(0, vec![]).unwrap(),
            &mapper,
        ).unwrap();
        assert!(result.contributions.is_empty());
        assert_eq!(result.unavailable_outputs.len(), 1);
        assert!(result.diagnostics.iter().any(|item| item.code == CONTROLLER_DISABLED));
        assert!(result.diagnostics.iter().any(|item| item.code == OUTPUT_SKIPPED));
    }

    #[test]
    fn controller_mapping_failure_is_local_and_downstream_output_becomes_unavailable() {
        let graph = ControllerGraph::new(vec![
            node("upstream", vec![input_id("manual")], vec![], true, "up-binding"),
            node(
                "downstream",
                vec![input_id("from-upstream")],
                vec![controller_id("upstream")],
                true,
                "down-binding",
            ),
        ]).unwrap();
        let definitions = vec![
            manual_definition("manual", FallbackPolicy::Reject),
            InputDefinition {
                id: input_id("from-upstream"),
                source: InputSource::ControllerOutput {
                    controller_id: controller_id("upstream"),
                    binding_id: binding_id("up-binding"),
                },
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Unavailable,
            },
        ];
        let mapper = |controller: &Controller, _: &ResolvedInputs| {
            if controller.id == controller_id("upstream") {
                Err(CoreError::InvalidMapping("mapping domain failure".into()))
            } else {
                panic!("downstream must be skipped after upstream mapping failure")
            }
        };
        let result = evaluate(&graph, &definitions, &snapshot("manual", 0.5), &mapper).unwrap();
        assert!(result.contributions.is_empty());
        assert!(result.unavailable_outputs.contains(&(controller_id("upstream"), binding_id("up-binding"))));
        assert!(result.unavailable_outputs.contains(&(controller_id("downstream"), binding_id("down-binding"))));
        assert!(result.diagnostics.iter().any(|item| item.code == crate::diagnostic_codes::MAPPING_DOMAIN));
        assert!(result.diagnostics.iter().any(|item| item.code == INPUT_UNAVAILABLE));
    }

    #[test]
    fn controller_output_reject_policy_aborts_when_upstream_output_is_unavailable() {
        let graph = ControllerGraph::new(vec![
            node("upstream", vec![input_id("manual")], vec![], true, "up-binding"),
            node(
                "downstream",
                vec![input_id("from-upstream")],
                vec![controller_id("upstream")],
                true,
                "down-binding",
            ),
        ]).unwrap();
        let definitions = vec![
            manual_definition("manual", FallbackPolicy::Reject),
            InputDefinition {
                id: input_id("from-upstream"),
                source: InputSource::ControllerOutput {
                    controller_id: controller_id("upstream"),
                    binding_id: binding_id("up-binding"),
                },
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            },
        ];
        let mapper = |controller: &Controller, _: &ResolvedInputs| {
            if controller.id == controller_id("upstream") {
                Err(CoreError::InvalidMapping("mapping domain failure".into()))
            } else {
                panic!("downstream must not run under Reject fallback policy")
            }
        };
        let result = evaluate(&graph, &definitions, &snapshot("manual", 0.5), &mapper);
        assert!(matches!(result, Err(CoreError::MissingInput(_))));
    }

    #[test]
    fn independent_controller_insertion_and_snapshot_order_do_not_change_results() {
        let alpha = node("alpha", vec![input_id("input-alpha")], vec![], true, "binding-alpha");
        let beta = node("beta", vec![input_id("input-beta")], vec![], true, "binding-beta");
        let graph_ab = ControllerGraph::new(vec![alpha.clone(), beta.clone()]).unwrap();
        let graph_ba = ControllerGraph::new(vec![beta, alpha]).unwrap();
        let definitions = vec![
            manual_definition("input-alpha", FallbackPolicy::Reject),
            manual_definition("input-beta", FallbackPolicy::Reject),
        ];
        let snapshot_ab = InputSnapshot::new(5, vec![
            InputSample::valid(input_id("input-alpha"), TypedValue::scalar(0.25).unwrap(), Some(5)),
            InputSample::valid(input_id("input-beta"), TypedValue::scalar(-0.5).unwrap(), Some(5)),
        ]).unwrap();
        let snapshot_ba = InputSnapshot::new(5, vec![
            InputSample::valid(input_id("input-beta"), TypedValue::scalar(-0.5).unwrap(), Some(5)),
            InputSample::valid(input_id("input-alpha"), TypedValue::scalar(0.25).unwrap(), Some(5)),
        ]).unwrap();
        let mapper = |controller: &Controller, inputs: &ResolvedInputs| {
            Ok(BTreeMap::from([(
                controller.bindings[0].id.clone(),
                inputs.value(&controller.inputs[0]).unwrap(),
            )]))
        };
        let result_ab = evaluate(&graph_ab, &definitions, &snapshot_ab, &mapper).unwrap();
        let result_ba = evaluate(&graph_ba, &definitions, &snapshot_ba, &mapper).unwrap();
        assert_eq!(result_ab, result_ba);
        assert_eq!(
            result_ab.contributions.iter().map(|item| item.binding_id.to_string()).collect::<Vec<_>>(),
            vec!["binding-alpha", "binding-beta"],
        );
    }

    #[test]
    fn mapper_cannot_emit_a_binding_not_owned_by_the_controller() {
        let graph = ControllerGraph::new(vec![
            node("controller-a", vec![input_id("input-a")], vec![], true, "binding-a"),
        ]).unwrap();
        let definitions = vec![manual_definition("input-a", FallbackPolicy::Reject)];
        let mapper = |_: &Controller, _: &ResolvedInputs| {
            Ok(BTreeMap::from([(
                binding_id("foreign-binding"),
                TypedValue::scalar(1.0).unwrap(),
            )]))
        };
        let result = evaluate(&graph, &definitions, &snapshot("input-a", 0.5), &mapper);
        assert!(matches!(result, Err(CoreError::Evaluation(_))));
    }

    #[test]
    fn missing_one_of_multiple_binding_outputs_is_explicitly_unavailable() {
        let mut controller = node(
            "controller-a",
            vec![input_id("input-a")],
            vec![],
            true,
            "binding-a",
        );
        controller.bindings.push(binding("binding-b"));
        let graph = ControllerGraph::new(vec![controller]).unwrap();
        let definitions = vec![manual_definition("input-a", FallbackPolicy::Reject)];
        let mapper = |controller: &Controller, inputs: &ResolvedInputs| {
            Ok(BTreeMap::from([(
                controller.bindings[0].id.clone(),
                inputs.value(&controller.inputs[0]).unwrap(),
            )]))
        };
        let result = evaluate(&graph, &definitions, &snapshot("input-a", 0.75), &mapper).unwrap();
        assert_eq!(result.contributions.len(), 1);
        assert!(result.unavailable_outputs.contains(&(controller_id("controller-a"), binding_id("binding-b"))));
        assert!(result.diagnostics.iter().any(|item| item.code == OUTPUT_SKIPPED));
    }

    #[test]
    fn target_type_mismatch_rejects_only_the_incompatible_binding_output() {
        let mut controller = node(
            "controller-a",
            vec![input_id("input-a")],
            vec![],
            true,
            "binding-a",
        );
        controller.bindings[0].target = TargetMapping::GeometryDeform {
            element_id: "element-a".into(),
            pin_id: "pin-a".into(),
            negative_offset: [-1.0, 0.0],
            neutral_offset: [0.0, 0.0],
            positive_offset: [1.0, 0.0],
        };
        let graph = ControllerGraph::new(vec![controller]).unwrap();
        let definitions = vec![manual_definition("input-a", FallbackPolicy::Reject)];
        let mapper = |controller: &Controller, _: &ResolvedInputs| {
            Ok(BTreeMap::from([(
                controller.bindings[0].id.clone(),
                TypedValue::scalar(0.5).unwrap(),
            )]))
        };
        let result = evaluate(&graph, &definitions, &snapshot("input-a", 0.5), &mapper).unwrap();
        assert!(result.contributions.is_empty());
        assert!(result.unavailable_outputs.contains(&(controller_id("controller-a"), binding_id("binding-a"))));
        assert!(result.diagnostics.iter().any(|item| item.code == MAPPING_TYPE));
        assert!(result.diagnostics.iter().any(|item| item.code == OUTPUT_SKIPPED));
    }

    #[test]
    fn unused_external_snapshot_samples_are_still_type_checked() {
        let graph = ControllerGraph::new(vec![
            node("controller-a", vec![input_id("input-a")], vec![], true, "binding-a"),
        ]).unwrap();
        let definitions = vec![
            manual_definition("input-a", FallbackPolicy::Reject),
            manual_definition("unused-input", FallbackPolicy::Unavailable),
        ];
        let snapshot = InputSnapshot::new(5, vec![
            InputSample::valid(input_id("input-a"), TypedValue::scalar(0.5).unwrap(), Some(5)),
            InputSample::valid(input_id("unused-input"), TypedValue::vector2(1.0, 2.0).unwrap(), Some(5)),
        ]).unwrap();
        let mapper = |_: &Controller, _: &ResolvedInputs| Ok(BTreeMap::new());
        let result = evaluate(&graph, &definitions, &snapshot, &mapper);
        assert!(matches!(result, Err(CoreError::InputValueTypeMismatch { .. })));
    }

    #[test]
    fn controller_output_samples_cannot_be_injected_through_external_snapshot() {
        let graph = ControllerGraph::new(vec![
            node("upstream", vec![input_id("manual")], vec![], true, "up-binding"),
            node(
                "downstream",
                vec![input_id("from-upstream")],
                vec![controller_id("upstream")],
                true,
                "down-binding",
            ),
        ]).unwrap();
        let definitions = vec![
            manual_definition("manual", FallbackPolicy::Reject),
            InputDefinition {
                id: input_id("from-upstream"),
                source: InputSource::ControllerOutput {
                    controller_id: controller_id("upstream"),
                    binding_id: binding_id("up-binding"),
                },
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::Reject,
            },
        ];
        let snapshot = InputSnapshot::new(5, vec![
            InputSample::valid(input_id("manual"), TypedValue::scalar(0.5).unwrap(), Some(5)),
            InputSample::valid(input_id("from-upstream"), TypedValue::scalar(99.0).unwrap(), Some(5)),
        ]).unwrap();
        let mapper = |_: &Controller, _: &ResolvedInputs| Ok(BTreeMap::new());
        let result = evaluate(&graph, &definitions, &snapshot, &mapper);
        assert!(matches!(result, Err(CoreError::InvalidInputSample { .. })));
    }

    #[test]
    fn explicit_fallback_can_resolve_an_unavailable_controller_output() {
        let graph = ControllerGraph::new(vec![
            node("upstream", vec![input_id("manual")], vec![], true, "up-binding"),
            node(
                "downstream",
                vec![input_id("from-upstream")],
                vec![controller_id("upstream")],
                true,
                "down-binding",
            ),
        ]).unwrap();
        let definitions = vec![
            manual_definition("manual", FallbackPolicy::Reject),
            InputDefinition {
                id: input_id("from-upstream"),
                source: InputSource::ControllerOutput {
                    controller_id: controller_id("upstream"),
                    binding_id: binding_id("up-binding"),
                },
                dimension: InputDimension::Scalar,
                fallback: FallbackPolicy::UseValue(TypedValue::scalar(0.125).unwrap()),
            },
        ];
        let mapper = |controller: &Controller, inputs: &ResolvedInputs| {
            if controller.id == controller_id("upstream") {
                Err(CoreError::InvalidMapping("upstream unavailable".into()))
            } else {
                Ok(BTreeMap::from([(
                    controller.bindings[0].id.clone(),
                    inputs.value(&controller.inputs[0]).unwrap(),
                )]))
            }
        };
        let result = evaluate(&graph, &definitions, &snapshot("manual", 0.5), &mapper).unwrap();
        assert_eq!(result.contributions.len(), 1);
        assert_eq!(result.contributions[0].binding_id, binding_id("down-binding"));
        assert_eq!(result.contributions[0].value.as_scalar(), Some(0.125));
        assert!(result.diagnostics.iter().any(|item| item.code == INPUT_UNAVAILABLE));
    }

    #[test]
    fn non_finite_mapping_failure_is_local_to_the_controller() {
        let graph = ControllerGraph::new(vec![
            node("controller-a", vec![input_id("input-a")], vec![], true, "binding-a"),
        ]).unwrap();
        let definitions = vec![manual_definition("input-a", FallbackPolicy::Reject)];
        let mapper = |_: &Controller, _: &ResolvedInputs| -> Result<BTreeMap<BindingId, TypedValue>, CoreError> {
            Err(CoreError::NonFiniteValue { context: "mapped output" })
        };
        let result = evaluate(&graph, &definitions, &snapshot("input-a", 0.5), &mapper).unwrap();
        assert!(result.contributions.is_empty());
        assert!(result.unavailable_outputs.contains(&(controller_id("controller-a"), binding_id("binding-a"))));
        assert!(result.diagnostics.iter().any(|item| item.code == crate::diagnostic_codes::MAPPING_DOMAIN));
    }

    #[test]
    fn evaluator_rechecks_cycles_even_if_graph_was_built_unchecked() {
        let graph = ControllerGraph::from_unchecked_for_test(vec![
            node("a", vec![input_id("input-a")], vec![controller_id("b")], true, "binding-a"),
            node("b", vec![input_id("input-b")], vec![controller_id("a")], true, "binding-b"),
        ]);
        let calls = Cell::new(0);
        let mapper = |_: &Controller, _: &ResolvedInputs| {
            calls.set(calls.get() + 1);
            Ok(BTreeMap::new())
        };
        let result = evaluate(
            &graph,
            &[
                manual_definition("input-a", FallbackPolicy::Unavailable),
                manual_definition("input-b", FallbackPolicy::Unavailable),
            ],
            &InputSnapshot::new(0, vec![]).unwrap(),
            &mapper,
        );
        assert!(matches!(result, Err(CoreError::CycleDetected(_))));
        assert_eq!(calls.get(), 0);
    }
}
