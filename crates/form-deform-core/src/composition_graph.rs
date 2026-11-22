//! Explicit, typed FDM v2 composition graphs.
//!
//! Independent deformation evaluation remains the default. Composition only
//! occurs through a persisted graph whose order, operators and inputs are explicit.
//! Smart Controller output is deliberately not a node kind in this contract.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::host_contract::{
    CoordinateSpaceDescriptor, GeometrySnapshotStamp, HostTargetRef,
};

use crate::persistence::{DeformationDefinition, GeometryValueKind};

pub const COMPOSITION_GRAPH_SCHEMA_VERSION: u32 = 1;
pub const MAX_COMPOSITION_NODES: usize = 1_024;
pub const MAX_COMPOSITION_REFERENCES: usize = 4_096;
const WEIGHT_SUM_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionGraph {
    graph_id: String,
    revision: u64,
    schema_version: u32,
    target: HostTargetRef,
    reference_geometry: GeometrySnapshotStamp,
    coordinate_space: CoordinateSpaceDescriptor,
    input_kind: GeometryValueKind,
    nodes: Vec<CompositionNode>,
    outputs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompositionNode {
    Deformer {
        node_id: String,
        deformer_id: String,
    },
    Sequential {
        node_id: String,
        /// Ordered from first stage to last stage; order is semantic.
        stages: Vec<String>,
    },
    Parallel {
        node_id: String,
        /// Explicit independent branches evaluated from the same reference input.
        branches: Vec<String>,
    },
    WeightedBlend {
        node_id: String,
        inputs: Vec<WeightedInput>,
    },
}

/// Runtime shape of a node's output. Parallel is a collection of independent
/// branch values, never a single geometry and never an implicit blend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeOutputCardinality {
    Geometry,
    BranchCollection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeSignature {
    input: GeometryValueKind,
    output: GeometryValueKind,
    cardinality: NodeOutputCardinality,
}

impl NodeSignature {
    pub fn input_kind(&self) -> GeometryValueKind { self.input }
    pub fn output_kind(&self) -> GeometryValueKind { self.output }
    pub fn output_cardinality(&self) -> NodeOutputCardinality { self.cardinality }
}

/// One node in deterministic execution order. Dependencies retain their
/// authored semantic order (stage/branch/weight order), even though independent
/// ready nodes are selected lexicographically by stable node ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedNode {
    node_id: String,
    dependencies: Vec<String>,
}

impl PlannedNode {
    pub fn node_id(&self) -> &str { &self.node_id }
    pub fn dependencies(&self) -> &[String] { &self.dependencies }
}

/// Immutable plan produced only after graph and deformation definitions pass
/// validation. This is a plan, not an executor or scheduler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionExecutionPlan {
    graph_id: String,
    graph_revision: u64,
    nodes: Vec<PlannedNode>,
    outputs: Vec<String>,
    output_signature: NodeSignature,
}

impl CompositionExecutionPlan {
    pub fn graph_id(&self) -> &str { &self.graph_id }
    pub fn graph_revision(&self) -> u64 { self.graph_revision }
    pub fn nodes(&self) -> &[PlannedNode] { &self.nodes }
    pub fn outputs(&self) -> &[String] { &self.outputs }
    pub fn output_signature(&self) -> NodeSignature { self.output_signature }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightedInput {
    node_id: String,
    weight: f64,
}

impl WeightedInput {
    pub fn new(node_id: impl Into<String>, weight: f64) -> Result<Self, CompositionGraphError> {
        let value = Self { node_id: node_id.into(), weight };
        require_id(&value.node_id)?;
        if !value.weight.is_finite() || value.weight <= 0.0 {
            return Err(CompositionGraphError::InvalidWeight);
        }
        Ok(value)
    }

    pub fn node_id(&self) -> &str { &self.node_id }
    pub fn weight(&self) -> f64 { self.weight }
}

impl CompositionNode {
    pub fn deformer(node_id: impl Into<String>, deformer_id: impl Into<String>) -> Self {
        Self::Deformer { node_id: node_id.into(), deformer_id: deformer_id.into() }
    }

    pub fn sequential(node_id: impl Into<String>, stages: Vec<String>) -> Self {
        Self::Sequential { node_id: node_id.into(), stages }
    }

    pub fn parallel(node_id: impl Into<String>, branches: Vec<String>) -> Self {
        Self::Parallel { node_id: node_id.into(), branches }
    }

    pub fn weighted_blend(node_id: impl Into<String>, inputs: Vec<WeightedInput>) -> Self {
        Self::WeightedBlend { node_id: node_id.into(), inputs }
    }

    pub fn node_id(&self) -> &str {
        match self {
            Self::Deformer { node_id, .. }
            | Self::Sequential { node_id, .. }
            | Self::Parallel { node_id, .. }
            | Self::WeightedBlend { node_id, .. } => node_id,
        }
    }

    fn references(&self) -> Vec<&str> {
        match self {
            Self::Deformer { .. } => Vec::new(),
            Self::Sequential { stages, .. } => stages.iter().map(String::as_str).collect(),
            Self::Parallel { branches, .. } => branches.iter().map(String::as_str).collect(),
            Self::WeightedBlend { inputs, .. } => inputs.iter().map(|input| input.node_id.as_str()).collect(),
        }
    }
}

impl CompositionGraph {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        graph_id: impl Into<String>,
        revision: u64,
        target: HostTargetRef,
        reference_geometry: GeometrySnapshotStamp,
        coordinate_space: CoordinateSpaceDescriptor,
        input_kind: GeometryValueKind,
        nodes: Vec<CompositionNode>,
        outputs: Vec<String>,
    ) -> Result<Self, CompositionGraphError> {
        let value = Self {
            graph_id: graph_id.into(),
            revision,
            schema_version: COMPOSITION_GRAPH_SCHEMA_VERSION,
            target,
            reference_geometry,
            coordinate_space,
            input_kind,
            nodes,
            outputs,
        };
        value.validate_structure()?;
        Ok(value)
    }

    pub fn validate_structure(&self) -> Result<(), CompositionGraphError> {
        require_id(&self.graph_id)?;
        if self.revision == 0 || self.schema_version != COMPOSITION_GRAPH_SCHEMA_VERSION {
            return Err(CompositionGraphError::UnsupportedGraphSchema);
        }
        self.target.validate().map_err(|_| CompositionGraphError::InvalidTarget)?;
        self.reference_geometry.validate().map_err(|_| CompositionGraphError::InvalidReferenceGeometry)?;
        self.coordinate_space.validate().map_err(|_| CompositionGraphError::InvalidCoordinateSpace)?;
        if self.reference_geometry.target() != &self.target {
            return Err(CompositionGraphError::ReferenceTargetMismatch);
        }
        if self.nodes.is_empty() || self.outputs.is_empty() {
            return Err(CompositionGraphError::EmptyGraph);
        }
        if self.nodes.len() > MAX_COMPOSITION_NODES {
            return Err(CompositionGraphError::ResourceLimitExceeded);
        }

        let mut by_id = BTreeMap::new();
        for node in &self.nodes {
            require_id(node.node_id())?;
            if by_id.insert(node.node_id(), node).is_some() {
                return Err(CompositionGraphError::DuplicateNodeId);
            }
            match node {
                CompositionNode::Deformer { deformer_id, .. } => require_id(deformer_id)?,
                CompositionNode::Sequential { stages, .. } => {
                    validate_reference_list(stages, 2)?;
                }
                CompositionNode::Parallel { branches, .. } => {
                    validate_reference_list(branches, 2)?;
                }
                CompositionNode::WeightedBlend { inputs, .. } => {
                    if inputs.len() < 2 {
                        return Err(CompositionGraphError::InsufficientInputs);
                    }
                    let mut seen = BTreeSet::new();
                    let mut total = 0.0;
                    for input in inputs {
                        require_id(&input.node_id)?;
                        if !seen.insert(input.node_id.as_str()) {
                            return Err(CompositionGraphError::DuplicateInput);
                        }
                        if !input.weight.is_finite() || input.weight <= 0.0 {
                            return Err(CompositionGraphError::InvalidWeight);
                        }
                        total += input.weight;
                    }
                    if !total.is_finite() || (total - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
                        return Err(CompositionGraphError::WeightsMustSumToOne);
                    }
                }
            }
        }

        let mut output_ids = BTreeSet::new();
        for output in &self.outputs {
            require_id(output)?;
            if !output_ids.insert(output.as_str()) {
                return Err(CompositionGraphError::DuplicateOutput);
            }
            if !by_id.contains_key(output.as_str()) {
                return Err(CompositionGraphError::MissingNodeReference);
            }
        }
        let mut reference_count = 0usize;
        for node in &self.nodes {
            let refs = node.references();
            reference_count = reference_count.saturating_add(refs.len());
            if reference_count > MAX_COMPOSITION_REFERENCES {
                return Err(CompositionGraphError::ResourceLimitExceeded);
            }
            let mut seen = BTreeSet::new();
            for reference in refs {
                if !by_id.contains_key(reference) {
                    return Err(CompositionGraphError::MissingNodeReference);
                }
                if !seen.insert(reference) {
                    return Err(CompositionGraphError::DuplicateInput);
                }
                if reference == node.node_id() {
                    return Err(CompositionGraphError::CycleDetected);
                }
            }
        }

        self.validate_acyclic(&by_id)?;
        self.validate_all_nodes_reach_output(&by_id)?;
        Ok(())
    }

    pub fn validate_against(
        &self,
        definitions: &[DeformationDefinition],
    ) -> Result<(), CompositionGraphError> {
        self.validate_structure()?;
        let definitions_by_id: BTreeMap<&str, &DeformationDefinition> =
            definitions.iter().map(|definition| (definition.id(), definition)).collect();

        for node in &self.nodes {
            if let CompositionNode::Deformer { deformer_id, .. } = node {
                let definition = definitions_by_id.get(deformer_id.as_str())
                    .ok_or(CompositionGraphError::MissingDeformer)?;
                definition.validate().map_err(|_| CompositionGraphError::InvalidDeformer)?;
                if definition.target() != &self.target {
                    return Err(CompositionGraphError::TargetMismatch);
                }
                if definition.reference_geometry() != &self.reference_geometry {
                    return Err(CompositionGraphError::ReferenceGeometryMismatch);
                }
                if definition.coordinate_space() != &self.coordinate_space {
                    return Err(CompositionGraphError::CoordinateSpaceMismatch);
                }
            }
        }

        let node_map: BTreeMap<&str, &CompositionNode> =
            self.nodes.iter().map(|node| (node.node_id(), node)).collect();
        let mut signatures = BTreeMap::new();
        for node in &self.nodes {
            let signature = self.resolve_signature(node, &node_map, &definitions_by_id, &mut signatures)?;
            signatures.insert(node.node_id(), signature);
        }
        let mut graph_output_signature: Option<NodeSignature> = None;
        for output in &self.outputs {
            let signature = signatures.get(output.as_str()).ok_or(CompositionGraphError::MissingNodeReference)?;
            if signature.input != self.input_kind {
                return Err(CompositionGraphError::GraphInputTypeMismatch);
            }
            if let Some(expected) = graph_output_signature {
                if signature.output != expected.output {
                    return Err(CompositionGraphError::IncompatibleValueTypes);
                }
                if signature.cardinality != expected.cardinality {
                    return Err(CompositionGraphError::GraphOutputCardinalityMismatch);
                }
            } else {
                graph_output_signature = Some(*signature);
            }
        }
        Ok(())
    }

    fn resolve_signature<'a>(
        &self,
        node: &'a CompositionNode,
        nodes: &BTreeMap<&str, &'a CompositionNode>,
        definitions: &BTreeMap<&str, &DeformationDefinition>,
        cache: &mut BTreeMap<&'a str, NodeSignature>,
    ) -> Result<NodeSignature, CompositionGraphError> {
        if let Some(signature) = cache.get(node.node_id()) {
            return Ok(*signature);
        }
        let signature = match node {
            CompositionNode::Deformer { deformer_id, .. } => {
                let definition = definitions.get(deformer_id.as_str())
                    .ok_or(CompositionGraphError::MissingDeformer)?;
                NodeSignature {
                    input: definition.input_kind(),
                    output: definition.output_kind(),
                    cardinality: NodeOutputCardinality::Geometry,
                }
            }
            CompositionNode::Sequential { stages, .. } => {
                let mut resolved = stages.iter().map(|id| {
                    let child = nodes.get(id.as_str()).ok_or(CompositionGraphError::MissingNodeReference)?;
                    self.resolve_signature(child, nodes, definitions, cache)
                });
                let first = resolved.next().ok_or(CompositionGraphError::InsufficientInputs)??;
                if first.cardinality != NodeOutputCardinality::Geometry {
                    return Err(CompositionGraphError::SequentialRequiresSingleGeometry);
                }
                let mut previous_output = first.output;
                for item in resolved {
                    let current = item?;
                    if current.cardinality != NodeOutputCardinality::Geometry {
                        return Err(CompositionGraphError::SequentialRequiresSingleGeometry);
                    }
                    if current.input != previous_output {
                        return Err(CompositionGraphError::IncompatibleValueTypes);
                    }
                    previous_output = current.output;
                }
                NodeSignature {
                    input: first.input,
                    output: previous_output,
                    cardinality: NodeOutputCardinality::Geometry,
                }
            }
            CompositionNode::Parallel { branches, .. } => {
                let mut resolved = branches.iter().map(|id| {
                    let child = nodes.get(id.as_str()).ok_or(CompositionGraphError::MissingNodeReference)?;
                    self.resolve_signature(child, nodes, definitions, cache)
                });
                let first = resolved.next().ok_or(CompositionGraphError::InsufficientInputs)??;
                if first.cardinality != NodeOutputCardinality::Geometry {
                    return Err(CompositionGraphError::ParallelBranchRequiresSingleGeometry);
                }
                for item in resolved {
                    let current = item?;
                    if current.cardinality != NodeOutputCardinality::Geometry {
                        return Err(CompositionGraphError::ParallelBranchRequiresSingleGeometry);
                    }
                    if current.input != first.input || current.output != first.output {
                        return Err(CompositionGraphError::IncompatibleValueTypes);
                    }
                }
                NodeSignature {
                    input: first.input,
                    output: first.output,
                    cardinality: NodeOutputCardinality::BranchCollection,
                }
            }
            CompositionNode::WeightedBlend { inputs, .. } => {
                let mut resolved = inputs.iter().map(|input| {
                    let child = nodes.get(input.node_id.as_str()).ok_or(CompositionGraphError::MissingNodeReference)?;
                    self.resolve_signature(child, nodes, definitions, cache)
                });
                let first = resolved.next().ok_or(CompositionGraphError::InsufficientInputs)??;
                if first.cardinality != NodeOutputCardinality::Geometry {
                    return Err(CompositionGraphError::WeightedBlendRequiresSingleGeometry);
                }
                for item in resolved {
                    let current = item?;
                    if current.cardinality != NodeOutputCardinality::Geometry {
                        return Err(CompositionGraphError::WeightedBlendRequiresSingleGeometry);
                    }
                    if current.input != first.input || current.output != first.output {
                        return Err(CompositionGraphError::IncompatibleValueTypes);
                    }
                }
                NodeSignature {
                    input: first.input,
                    output: first.output,
                    cardinality: NodeOutputCardinality::Geometry,
                }
            }
        };
        cache.insert(node.node_id(), signature);
        Ok(signature)
    }

    /// Validate the graph against its authored definitions, then produce a
    /// deterministic topological plan. Independent ready nodes are ordered by
    /// node ID; authored stage/branch/input order is preserved within each node.
    pub fn execution_plan(
        &self,
        definitions: &[DeformationDefinition],
    ) -> Result<CompositionExecutionPlan, CompositionGraphError> {
        self.validate_against(definitions)?;

        let by_id: BTreeMap<&str, &CompositionNode> =
            self.nodes.iter().map(|node| (node.node_id(), node)).collect();
        let mut indegree: BTreeMap<String, usize> = BTreeMap::new();
        let mut dependents: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for node in &self.nodes {
            let dependencies = node.references();
            indegree.insert(node.node_id().to_owned(), dependencies.len());
            for dependency in dependencies {
                dependents.entry(dependency.to_owned()).or_default()
                    .push(node.node_id().to_owned());
            }
        }
        for values in dependents.values_mut() {
            values.sort();
        }

        let mut ready: BTreeSet<String> = indegree.iter()
            .filter_map(|(id, count)| (*count == 0).then_some(id.clone()))
            .collect();
        let mut planned = Vec::with_capacity(self.nodes.len());
        while let Some(id) = ready.iter().next().cloned() {
            ready.remove(&id);
            let node = by_id.get(id.as_str()).ok_or(CompositionGraphError::MissingNodeReference)?;
            planned.push(PlannedNode {
                node_id: id.clone(),
                dependencies: node.references().into_iter().map(str::to_owned).collect(),
            });
            if let Some(children) = dependents.get(&id) {
                for child in children {
                    let count = indegree.get_mut(child).ok_or(CompositionGraphError::MissingNodeReference)?;
                    *count = count.checked_sub(1).ok_or(CompositionGraphError::CycleDetected)?;
                    if *count == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }
        if planned.len() != self.nodes.len() {
            return Err(CompositionGraphError::CycleDetected);
        }

        let node_map: BTreeMap<&str, &CompositionNode> =
            self.nodes.iter().map(|node| (node.node_id(), node)).collect();
        let definitions_by_id: BTreeMap<&str, &DeformationDefinition> =
            definitions.iter().map(|definition| (definition.id(), definition)).collect();
        let mut signatures = BTreeMap::new();
        for node in &self.nodes {
            let signature = self.resolve_signature(node, &node_map, &definitions_by_id, &mut signatures)?;
            signatures.insert(node.node_id(), signature);
        }
        let output_signature = *signatures.get(self.outputs[0].as_str())
            .ok_or(CompositionGraphError::MissingNodeReference)?;

        Ok(CompositionExecutionPlan {
            graph_id: self.graph_id.clone(),
            graph_revision: self.revision,
            nodes: planned,
            outputs: self.outputs.clone(),
            output_signature,
        })
    }

    fn validate_acyclic(&self, nodes: &BTreeMap<&str, &CompositionNode>) -> Result<(), CompositionGraphError> {
        fn visit<'a>(
            id: &'a str,
            nodes: &BTreeMap<&str, &'a CompositionNode>,
            visiting: &mut BTreeSet<&'a str>,
            visited: &mut BTreeSet<&'a str>,
        ) -> Result<(), CompositionGraphError> {
            if visited.contains(id) { return Ok(()); }
            if !visiting.insert(id) { return Err(CompositionGraphError::CycleDetected); }
            let node = nodes.get(id).ok_or(CompositionGraphError::MissingNodeReference)?;
            for dependency in node.references() {
                visit(dependency, nodes, visiting, visited)?;
            }
            visiting.remove(id);
            visited.insert(id);
            Ok(())
        }
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for id in nodes.keys() {
            visit(id, nodes, &mut visiting, &mut visited)?;
        }
        Ok(())
    }

    fn validate_all_nodes_reach_output(&self, nodes: &BTreeMap<&str, &CompositionNode>) -> Result<(), CompositionGraphError> {
        fn collect<'a>(
            id: &'a str,
            nodes: &BTreeMap<&str, &'a CompositionNode>,
            reached: &mut BTreeSet<&'a str>,
        ) -> Result<(), CompositionGraphError> {
            if !reached.insert(id) { return Ok(()); }
            let node = nodes.get(id).ok_or(CompositionGraphError::MissingNodeReference)?;
            for dependency in node.references() {
                collect(dependency, nodes, reached)?;
            }
            Ok(())
        }
        let mut reached = BTreeSet::new();
        for output in &self.outputs {
            collect(output, nodes, &mut reached)?;
        }
        if reached.len() != nodes.len() { return Err(CompositionGraphError::OrphanNode); }
        Ok(())
    }

    pub fn graph_id(&self) -> &str { &self.graph_id }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn schema_version(&self) -> u32 { self.schema_version }
    pub fn target(&self) -> &HostTargetRef { &self.target }
    pub fn reference_geometry(&self) -> &GeometrySnapshotStamp { &self.reference_geometry }
    pub fn coordinate_space(&self) -> &CoordinateSpaceDescriptor { &self.coordinate_space }
    pub fn input_kind(&self) -> GeometryValueKind { self.input_kind }
    pub fn nodes(&self) -> &[CompositionNode] { &self.nodes }
    pub fn outputs(&self) -> &[String] { &self.outputs }
}

fn validate_reference_list(values: &[String], minimum: usize) -> Result<(), CompositionGraphError> {
    if values.len() < minimum { return Err(CompositionGraphError::InsufficientInputs); }
    let mut seen = BTreeSet::new();
    for value in values {
        require_id(value)?;
        if !seen.insert(value.as_str()) { return Err(CompositionGraphError::DuplicateInput); }
    }
    Ok(())
}

fn require_id(value: &str) -> Result<(), CompositionGraphError> {
    if value.trim().is_empty() { Err(CompositionGraphError::EmptyIdentifier) } else { Ok(()) }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionGraphError {
    EmptyIdentifier,
    UnsupportedGraphSchema,
    InvalidTarget,
    InvalidReferenceGeometry,
    InvalidCoordinateSpace,
    ReferenceTargetMismatch,
    EmptyGraph,
    DuplicateNodeId,
    DuplicateOutput,
    MissingNodeReference,
    MissingDeformer,
    InvalidDeformer,
    InsufficientInputs,
    DuplicateInput,
    InvalidWeight,
    WeightsMustSumToOne,
    CycleDetected,
    OrphanNode,
    TargetMismatch,
    ReferenceGeometryMismatch,
    CoordinateSpaceMismatch,
    IncompatibleValueTypes,
    GraphInputTypeMismatch,
    SequentialRequiresSingleGeometry,
    ParallelBranchRequiresSingleGeometry,
    WeightedBlendRequiresSingleGeometry,
    GraphOutputCardinalityMismatch,
    ResourceLimitExceeded,
}

impl std::fmt::Display for CompositionGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "composition graph validation failed: {self:?}")
    }
}
impl std::error::Error for CompositionGraphError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceId, CoordinateSpaceKind,
        GeometryFingerprint, GeometryRevision, HostDocumentId, HostNamespace,
        HostTargetId, HostTargetKind, TopologyRevision,
    };
    use crate::persistence::{DeformationDefinition, DeformationKind, GeometryValueKind, ParameterValue};
    use std::collections::BTreeMap;

    fn target() -> HostTargetRef {
        HostTargetRef {
            namespace: HostNamespace::new("anicraft").unwrap(),
            document_id: HostDocumentId::new("doc-1").unwrap(),
            target_kind: HostTargetKind::new("shape").unwrap(),
            target_id: HostTargetId::new("shape-1").unwrap(),
            generation: 1,
        }
    }

    fn space() -> CoordinateSpaceDescriptor {
        CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        }
    }

    fn reference() -> GeometrySnapshotStamp {
        GeometrySnapshotStamp::new(
            target(), GeometryRevision(1), TopologyRevision(1),
            GeometryFingerprint { algorithm: "sha256-test".into(), digest: vec![1; 32] },
        ).unwrap()
    }

    fn definition(id: &str, input: GeometryValueKind, output: GeometryValueKind) -> DeformationDefinition {
        DeformationDefinition::new(
            id, 1, id, DeformationKind::PinDeformation, target(), reference(),
            space(), input, output, BTreeMap::<String, ParameterValue>::new(),
        ).unwrap()
    }

    #[test]
    fn graph_defaults_to_explicit_nodes_and_ordered_sequential_composition() {
        let defs = vec![definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement), definition("b", GeometryValueKind::Displacement, GeometryValueKind::Displacement)];
        let graph = CompositionGraph::new(
            "graph", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("n-a", "a"),
                CompositionNode::deformer("n-b", "b"),
                CompositionNode::sequential("seq", vec!["n-a".into(), "n-b".into()]),
            ],
            vec!["seq".into()],
        ).unwrap();
        assert!(graph.validate_against(&defs).is_ok());
    }

    #[test]
    fn graph_rejects_cycles() {
        let graph = CompositionGraph::new(
            "cycle", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::sequential("a", vec!["b".into(), "c".into()]),
                CompositionNode::sequential("b", vec!["a".into(), "c".into()]),
                CompositionNode::deformer("c", "unused"),
            ],
            vec!["a".into()],
        );
        assert_eq!(graph, Err(CompositionGraphError::CycleDetected));
    }

    #[test]
    fn weighted_blend_requires_positive_normalized_weights() {
        assert_eq!(WeightedInput::new("a", 0.0), Err(CompositionGraphError::InvalidWeight));
        let defs = vec![definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement), definition("b", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement)];
        let graph = CompositionGraph::new(
            "blend", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a", "a"),
                CompositionNode::deformer("b", "b"),
                CompositionNode::weighted_blend("blend", vec![
                    WeightedInput::new("a", 0.25).unwrap(),
                    WeightedInput::new("b", 0.75).unwrap(),
                ]),
            ],
            vec!["blend".into()],
        ).unwrap();
        assert!(graph.validate_against(&defs).is_ok());
    }

    #[test]
    fn graph_rejects_mismatched_coordinate_spaces() {
        let mut other = space();
        other.axes = AxisOrientation::XRightYUp;
        let definition = DeformationDefinition::new(
            "a", 1, "a", DeformationKind::PinDeformation, target(), reference(),
            other, GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement, BTreeMap::new(),
        ).unwrap();
        let graph = CompositionGraph::new(
            "graph", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![CompositionNode::deformer("a-node", "a")],
            vec!["a-node".into()],
        ).unwrap();
        assert_eq!(graph.validate_against(&[definition]), Err(CompositionGraphError::CoordinateSpaceMismatch));
    }

    #[test]
    fn graph_rejects_orphan_nodes_and_missing_inputs() {
        let orphan = CompositionGraph::new(
            "orphan", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![CompositionNode::deformer("a", "a"), CompositionNode::deformer("b", "b")],
            vec!["a".into()],
        );
        assert_eq!(orphan, Err(CompositionGraphError::OrphanNode));
        let missing = CompositionGraph::new(
            "missing", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![CompositionNode::sequential("seq", vec!["x".into(), "y".into()])],
            vec!["seq".into()],
        );
        assert_eq!(missing, Err(CompositionGraphError::MissingNodeReference));
    }

    #[test]
    fn graph_rejects_unknown_node_fields_and_resource_overflow() {
        let graph = CompositionGraph::new(
            "bounded", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![CompositionNode::deformer("one", "missing")],
            vec!["one".into()],
        ).unwrap();
        let json = serde_json::to_string(&graph).unwrap();
        let changed = json.replacen("{", "{\"unexpected\":true,", 1);
        assert!(serde_json::from_str::<CompositionGraph>(&changed).is_err());

        let too_many = (0..=MAX_COMPOSITION_NODES)
            .map(|i| CompositionNode::deformer(format!("n-{i}"), "d"))
            .collect::<Vec<_>>();
        assert_eq!(
            CompositionGraph::new("large", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry, too_many, vec!["n-0".into()]),
            Err(CompositionGraphError::ResourceLimitExceeded)
        );
    }

    #[test]
    fn sequential_stages_must_chain_input_to_previous_output() {
        let defs = vec![
            definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("b", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
        ];
        let graph = CompositionGraph::new(
            "bad-sequence", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a", "a"),
                CompositionNode::deformer("b", "b"),
                CompositionNode::sequential("seq", vec!["a".into(), "b".into()]),
            ],
            vec!["seq".into()],
        ).unwrap();
        assert_eq!(graph.validate_against(&defs), Err(CompositionGraphError::IncompatibleValueTypes));
    }

    #[test]
    fn parallel_node_has_branch_collection_cardinality() {
        let defs = vec![
            definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("b", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
        ];
        let graph = CompositionGraph::new(
            "parallel", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a-node", "a"),
                CompositionNode::deformer("b-node", "b"),
                CompositionNode::parallel("branches", vec!["a-node".into(), "b-node".into()]),
            ],
            vec!["branches".into()],
        ).unwrap();
        let plan = graph.execution_plan(&defs).unwrap();
        assert_eq!(plan.output_signature().output_cardinality(), NodeOutputCardinality::BranchCollection);
    }

    #[test]
    fn sequential_rejects_parallel_branch_collection_input() {
        let defs = vec![
            definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("b", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("c", GeometryValueKind::Displacement, GeometryValueKind::Displacement),
        ];
        let graph = CompositionGraph::new(
            "bad-sequential-cardinality", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a-node", "a"),
                CompositionNode::deformer("b-node", "b"),
                CompositionNode::parallel("parallel", vec!["a-node".into(), "b-node".into()]),
                CompositionNode::deformer("c-node", "c"),
                CompositionNode::sequential("seq", vec!["parallel".into(), "c-node".into()]),
            ],
            vec!["seq".into()],
        ).unwrap();
        assert_eq!(graph.validate_against(&defs), Err(CompositionGraphError::SequentialRequiresSingleGeometry));
    }

    #[test]
    fn weighted_blend_rejects_parallel_branch_collection_input() {
        let defs = vec![
            definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("b", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
        ];
        let graph = CompositionGraph::new(
            "bad-blend-cardinality", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a-node", "a"),
                CompositionNode::deformer("b-node", "b"),
                CompositionNode::parallel("parallel", vec!["a-node".into(), "b-node".into()]),
                CompositionNode::weighted_blend("blend", vec![
                    WeightedInput::new("parallel", 0.5).unwrap(),
                    WeightedInput::new("a-node", 0.5).unwrap(),
                ]),
            ],
            vec!["blend".into()],
        ).unwrap();
        assert_eq!(graph.validate_against(&defs), Err(CompositionGraphError::WeightedBlendRequiresSingleGeometry));
    }

    #[test]
    fn parallel_rejects_nested_branch_collection() {
        let defs = vec![
            definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("b", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("c", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
        ];
        let graph = CompositionGraph::new(
            "bad-parallel-cardinality", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a-node", "a"),
                CompositionNode::deformer("b-node", "b"),
                CompositionNode::parallel("inner", vec!["a-node".into(), "b-node".into()]),
                CompositionNode::deformer("c-node", "c"),
                CompositionNode::parallel("outer", vec!["inner".into(), "c-node".into()]),
            ],
            vec!["outer".into()],
        ).unwrap();
        assert_eq!(graph.validate_against(&defs), Err(CompositionGraphError::ParallelBranchRequiresSingleGeometry));
    }

    #[test]
    fn graph_rejects_mixed_output_cardinality() {
        let defs = vec![
            definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("b", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
        ];
        let graph = CompositionGraph::new(
            "mixed-outputs", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a-node", "a"),
                CompositionNode::deformer("b-node", "b"),
                CompositionNode::parallel("parallel", vec!["a-node".into(), "b-node".into()]),
            ],
            vec!["a-node".into(), "parallel".into()],
        ).unwrap();
        assert_eq!(graph.validate_against(&defs), Err(CompositionGraphError::GraphOutputCardinalityMismatch));
    }

    #[test]
    fn execution_plan_uses_lexical_tie_breaking_not_authored_node_storage_order() {
        let defs = vec![
            definition("z", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("c", GeometryValueKind::Displacement, GeometryValueKind::Displacement),
        ];
        let graph = CompositionGraph::new(
            "stable-plan", 7, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("z-node", "z"),
                CompositionNode::deformer("a-node", "a"),
                CompositionNode::deformer("c-node", "c"),
                CompositionNode::sequential("sequence", vec!["a-node".into(), "c-node".into()]),
                CompositionNode::weighted_blend("output", vec![
                    WeightedInput::new("z-node", 0.5).unwrap(),
                    WeightedInput::new("sequence", 0.5).unwrap(),
                ]),
            ],
            vec!["output".into()],
        ).unwrap();
        let first_plan = graph.execution_plan(&defs).unwrap();
        let reordered = CompositionGraph::new(
            "stable-plan", 7, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::weighted_blend("output", vec![
                    WeightedInput::new("z-node", 0.5).unwrap(),
                    WeightedInput::new("sequence", 0.5).unwrap(),
                ]),
                CompositionNode::sequential("sequence", vec!["a-node".into(), "c-node".into()]),
                CompositionNode::deformer("c-node", "c"),
                CompositionNode::deformer("a-node", "a"),
                CompositionNode::deformer("z-node", "z"),
            ],
            vec!["output".into()],
        ).unwrap();
        let second_plan = reordered.execution_plan(&defs).unwrap();
        let ids = |plan: &CompositionExecutionPlan| plan.nodes().iter().map(|node| node.node_id().to_owned()).collect::<Vec<_>>();
        assert_eq!(ids(&first_plan), ids(&second_plan));
        assert_eq!(ids(&first_plan), vec!["a-node", "c-node", "sequence", "z-node", "output"]);
        let sequence = first_plan.nodes().iter().find(|node| node.node_id() == "sequence").unwrap();
        assert_eq!(sequence.dependencies(), &["a-node".to_string(), "c-node".to_string()]);
    }

    #[test]
    fn execution_plan_preserves_weighted_input_order() {
        let defs = vec![
            definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("b", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
        ];
        let graph = CompositionGraph::new(
            "weighted-order", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a-node", "a"),
                CompositionNode::deformer("b-node", "b"),
                CompositionNode::weighted_blend("blend", vec![
                    WeightedInput::new("b-node", 0.75).unwrap(),
                    WeightedInput::new("a-node", 0.25).unwrap(),
                ]),
            ],
            vec!["blend".into()],
        ).unwrap();
        let plan = graph.execution_plan(&defs).unwrap();
        let blend = plan.nodes().iter().find(|node| node.node_id() == "blend").unwrap();
        assert_eq!(blend.dependencies(), &["b-node".to_string(), "a-node".to_string()]);
    }

    #[test]
    fn execution_planning_rejects_cycles_and_invalid_connections() {
        let defs = vec![
            definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement),
            definition("b", GeometryValueKind::Displacement, GeometryValueKind::Displacement),
        ];
        let cycle = CompositionGraph::new(
            "cycle-plan", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::sequential("left", vec!["right".into(), "a-node".into()]),
                CompositionNode::sequential("right", vec!["left".into(), "a-node".into()]),
                CompositionNode::deformer("a-node", "a"),
            ],
            vec!["left".into()],
        );
        assert_eq!(cycle, Err(CompositionGraphError::CycleDetected));

        let bad_type = CompositionGraph::new(
            "bad-connection", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a-node", "a"),
                CompositionNode::deformer("b-node", "b"),
                CompositionNode::sequential("sequence", vec!["b-node".into(), "a-node".into()]),
            ],
            vec!["sequence".into()],
        ).unwrap();
        assert_eq!(bad_type.execution_plan(&defs), Err(CompositionGraphError::IncompatibleValueTypes));
    }

    #[test]
    fn graph_rejects_incompatible_value_types() {
        let defs = vec![definition("a", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::Displacement), definition("b", GeometryValueKind::AbsoluteGeometry, GeometryValueKind::AbsoluteGeometry)];
        let graph = CompositionGraph::new(
            "bad-types", 1, target(), reference(), space(), GeometryValueKind::AbsoluteGeometry,
            vec![
                CompositionNode::deformer("a", "a"),
                CompositionNode::deformer("b", "b"),
                CompositionNode::weighted_blend("blend", vec![
                    WeightedInput::new("a", 0.5).unwrap(),
                    WeightedInput::new("b", 0.5).unwrap(),
                ]),
            ],
            vec!["blend".into()],
        ).unwrap();
        assert_eq!(graph.validate_against(&defs), Err(CompositionGraphError::IncompatibleValueTypes));
    }
}
