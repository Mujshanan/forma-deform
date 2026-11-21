use smart_controller_core::{canonical::{Controller, MappingConfig}, graph::ControllerGraph, ControllerId, InputId};

fn node(id: &str, deps: &[&str]) -> Controller {
    Controller {
        id: ControllerId::new(id).unwrap(), enabled: true,
        inputs: vec![InputId::new(format!("input-{id}")).unwrap()],
        dependencies: deps.iter().map(|d| ControllerId::new(*d).unwrap()).collect(),
        mapping: MappingConfig::Range1D { minimum: -1.0, neutral: 0.0, maximum: 1.0 },
        bindings: vec![], parent_scoped: false,
    }
}

#[test]
fn graph_orders_dependencies_before_dependents_and_ties_by_id() {
    let graph = ControllerGraph::new(vec![node("z", &["b"]), node("b", &[]), node("a", &[])]).unwrap();
    let order: Vec<_> = graph.topological_order().unwrap().into_iter().map(|id| id.to_string()).collect();
    assert_eq!(order, vec!["a", "b", "z"]);
}

#[test]
fn graph_rejects_missing_dependency_and_cycles() {
    assert!(ControllerGraph::new(vec![node("a", &["missing"])]).is_err());
    assert!(ControllerGraph::new(vec![node("a", &["b"]), node("b", &["a"])]).is_err());
}

#[test]
fn empty_graph_is_valid() {
    assert!(ControllerGraph::new(vec![]).unwrap().is_empty());
}
