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
        if !definition_map.contains_key(&sample.id) {
            return Err(CoreError::InvalidInputSample {
                input_id: sample.id.to_string(),
                reason: "snapshot contains a sample with no input definition",
            });
        }
    }

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
        if matches!(definition.source, InputSource::ControllerOutput { .. }) {
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
                outputs.get(&(producer.clone(), binding_id.clone())).copied()
                    .or_else(|| apply_fallback(&item.definition.fallback))
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
            mark_controller_outputs_unavailable(controller, &controller_id, &mut result);
            continue;
        }

        let mapped = match mapper.evaluate(controller, &resolved_inputs) {
            Ok(values) => values,
            Err(CoreError::InvalidMapping(message)) => {
                result.diagnostics.push(diagnostic(
                    crate::diagnostic_codes::MAPPING_DOMAIN,
                    DiagnosticSeverity::Error,
                    message,
                    Some(controller_id.clone()),
                    None,
                )?);
                mark_controller_outputs_unavailable(controller, &controller_id, &mut result);
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
                continue;
            }

            let (mode, base_ref) = match binding.mode {
                BindingMode::AbsolutePose => (CompositionMode::Replace, None),
                BindingMode::RelativeDelta => (CompositionMode::Additive, Some(BaseRef::Authored)),
            };
            let contribution = Contribution {
                binding_id: binding.id.clone(),
                target_id: binding.target_id.clone(),
                value,
                mode,
                weight: None,
                priority: None,
                base_ref,
            };
            contribution.validate()?;
            outputs.insert((controller_id.clone(), binding.id.clone()), value);
            result.contributions.push(contribution);
        }
    }

    // Preserve deterministic collection order independent of authored map
    // iteration. Stable sorting retains topological order for equal BindingIds.
    result.contributions.sort_by(|left, right| left.binding_id.cmp(&right.binding_id));
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
) {
    for binding in &controller.bindings {
        result.unavailable_outputs.insert((controller_id.clone(), binding.id.clone()));
    }
}

fn target_accepts_value(target: &TargetMapping, value: TypedValue) -> bool {
    match target {
        TargetMapping::SymbolFrame { .. } | TargetMapping::FrameRange { .. } | TargetMapping::Property { .. } =>
            matches!(value, TypedValue::Scalar(_)),
        TargetMapping::GeometryDeform { .. } => matches!(value, TypedValue::Vector2(_)),
    }
}
