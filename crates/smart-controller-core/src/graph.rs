//! Validated DAG storage with deterministic topological ordering.
use crate::{canonical::Controller, error::CoreError, ids::ControllerId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ControllerGraph { nodes: BTreeMap<ControllerId, Controller> }

impl ControllerGraph {
    pub fn new(controllers: Vec<Controller>) -> Result<Self, CoreError> {
        let mut nodes = BTreeMap::new();
        for controller in controllers {
            controller.validate_local()?;
            let id = controller.id.clone();
            if nodes.insert(id.clone(), controller).is_some() {
                return Err(CoreError::InvalidGraph(format!("duplicate controller ID {id}")));
            }
        }
        let graph = Self { nodes };
        graph.validate()?;
        Ok(graph)
    }

    pub fn get(&self, id: &ControllerId) -> Option<&Controller> { self.nodes.get(id) }
    pub fn iter(&self) -> impl Iterator<Item = (&ControllerId, &Controller)> { self.nodes.iter() }
    pub fn len(&self) -> usize { self.nodes.len() }
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }

    pub fn validate(&self) -> Result<(), CoreError> {
        let mut parents: BTreeMap<&ControllerId, usize> = BTreeMap::new();
        for controller in self.nodes.values() {
            for dependency in &controller.dependencies {
                if !self.nodes.contains_key(dependency) {
                    return Err(CoreError::InvalidGraph(format!("{} references missing dependency {}", controller.id, dependency)));
                }
                *parents.entry(dependency).or_default() += 1;
            }
        }
        for (id, count) in parents {
            if count > 1 && self.nodes[id].parent_scoped {
                return Err(CoreError::InvalidGraph(format!("shared node {id} contains parent-scoped configuration")));
            }
        }
        self.topological_order().map(|_| ())
    }

    /// Dependencies are evaluated before dependents; ties use canonical ID order.
    pub fn topological_order(&self) -> Result<Vec<ControllerId>, CoreError> {
        let mut indegree: BTreeMap<ControllerId, usize> = self.nodes.keys().cloned().map(|id| (id, 0)).collect();
        let mut dependents: BTreeMap<ControllerId, BTreeSet<ControllerId>> = BTreeMap::new();
        for node in self.nodes.values() {
            for dep in &node.dependencies {
                *indegree.get_mut(&node.id).expect("node inserted") += 1;
                dependents.entry(dep.clone()).or_default().insert(node.id.clone());
            }
        }
        let mut ready: BTreeSet<ControllerId> = indegree.iter().filter_map(|(id, degree)| (*degree == 0).then_some(id.clone())).collect();
        let mut ordered = Vec::with_capacity(self.nodes.len());
        while let Some(id) = ready.pop_first() {
            ordered.push(id.clone());
            if let Some(children) = dependents.get(&id) {
                for child in children {
                    let degree = indegree.get_mut(child).expect("dependent inserted");
                    *degree -= 1;
                    if *degree == 0 { ready.insert(child.clone()); }
                }
            }
        }
        if ordered.len() != self.nodes.len() {
            let cycle = indegree.into_iter().filter_map(|(id, degree)| (degree > 0).then_some(id.to_string())).collect();
            return Err(CoreError::CycleDetected(cycle));
        }
        Ok(ordered)
    }
}


#[cfg(test)]
impl ControllerGraph {
    /// Construct an intentionally invalid graph for evaluator cycle-defence tests.
    pub(crate) fn from_unchecked_for_test(controllers: Vec<Controller>) -> Self {
        Self {
            nodes: controllers.into_iter().map(|controller| (controller.id.clone(), controller)).collect(),
        }
    }
}
