//! Host-neutral interfaces for deterministic FDM composition evaluation.
//!
//! This module defines evaluator and value contracts only. Planning and execution
//! are implemented in later M1.3 slices. It does not schedule host work or mutate
//! AniCraft geometry.

use crate::evaluation_contract::{DependencyStamp, EvaluationRequest};
use crate::path2d::Path2D;
use crate::persistence::{DeformationDefinition, DeformationKind, GeometryValueKind};

/// A single typed geometry value carried between graph nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct GeometryValue {
    kind: GeometryValueKind,
    geometry: Path2D,
}

impl GeometryValue {
    pub fn new(kind: GeometryValueKind, geometry: Path2D) -> Self {
        Self { kind, geometry }
    }

    pub fn kind(&self) -> GeometryValueKind { self.kind }
    pub fn geometry(&self) -> &Path2D { &self.geometry }
    pub fn into_geometry(self) -> Path2D { self.geometry }
}

/// One explicitly identified result from a parallel branch.
#[derive(Clone, Debug, PartialEq)]
pub struct BranchValue {
    branch_node_id: String,
    value: GeometryValue,
}

impl BranchValue {
    pub fn new(branch_node_id: impl Into<String>, value: GeometryValue) -> Result<Self, EvaluationInterfaceError> {
        let branch_node_id = branch_node_id.into();
        require_id(&branch_node_id)?;
        Ok(Self { branch_node_id, value })
    }

    pub fn branch_node_id(&self) -> &str { &self.branch_node_id }
    pub fn value(&self) -> &GeometryValue { &self.value }
}

/// Runtime cardinality is explicit: a normal node yields one geometry, while a
/// Parallel node yields an ordered set of independently evaluated branches.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeValue {
    Geometry(GeometryValue),
    Branches(Vec<BranchValue>),
}

/// Read-only context supplied to every operator invocation. The request carries
/// the immutable source snapshots, time sample, model revision and dependencies.
pub struct OperatorContext<'a> {
    request: &'a EvaluationRequest,
}

impl<'a> OperatorContext<'a> {
    pub fn new(request: &'a EvaluationRequest) -> Result<Self, EvaluationInterfaceError> {
        request.validate().map_err(|_| EvaluationInterfaceError::InvalidEvaluationRequest)?;
        Ok(Self { request })
    }

    pub fn request(&self) -> &'a EvaluationRequest { self.request }
    pub fn dependencies(&self) -> &'a [DependencyStamp] { self.request.dependencies() }
}

/// Plug-in implementation for one authored deformation kind.
///
/// Implementations must be deterministic for identical definition, input,
/// request and algorithm version. They must be pure with respect to host state:
/// no document reads, writes, global playhead access or hidden mutable cache.
pub trait DeformerOperator: Send + Sync {
    fn kind(&self) -> DeformationKind;
    fn algorithm_id(&self) -> &'static str;
    fn algorithm_version(&self) -> u32;

    fn evaluate(
        &self,
        definition: &DeformationDefinition,
        input: &GeometryValue,
        context: &OperatorContext<'_>,
    ) -> Result<GeometryValue, OperatorFailure>;
}

/// Host-neutral lookup boundary. The registry owns operator instances; the
/// composition engine only requests an implementation by its authored kind.
pub trait OperatorRegistry {
    fn resolve(&self, kind: DeformationKind) -> Option<&dyn DeformerOperator>;
}

/// Stable failure categories suitable for diagnostics and tests. Detailed
/// human-readable context may be attached by the caller, not used as identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorFailure {
    UnsupportedDefinition,
    InputKindMismatch,
    CoordinateSpaceMismatch,
    InvalidParameters,
    TopologyMismatch,
    NonFiniteResult,
    ResourceLimitExceeded,
    ArithmeticFailure,
}

/// Contract-level failures before or around operator execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationInterfaceError {
    EmptyIdentifier,
    InvalidEvaluationRequest,
    MissingOperator,
    InvalidOperatorVersion,
    InvalidNodeOutputCardinality,
    IncompatibleValueKinds,
}

impl std::fmt::Display for EvaluationInterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FDM evaluation interface error: {self:?}")
    }
}
impl std::error::Error for EvaluationInterfaceError {}

fn require_id(value: &str) -> Result<(), EvaluationInterfaceError> {
    if value.trim().is_empty() {
        Err(EvaluationInterfaceError::EmptyIdentifier)
    } else {
        Ok(())
    }
}

/// Validate the metadata exposed by an operator before it is admitted to a run.
pub fn validate_operator(operator: &dyn DeformerOperator) -> Result<(), EvaluationInterfaceError> {
    if operator.algorithm_id().trim().is_empty() || operator.algorithm_version() == 0 {
        return Err(EvaluationInterfaceError::InvalidOperatorVersion);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path2d::{PathCommand, Point2};

    fn geometry() -> Path2D {
        Path2D::new(vec![PathCommand::MoveTo(Point2::new(0.0, 0.0).unwrap())]).unwrap()
    }

    #[test]
    fn geometry_values_retain_their_explicit_semantic_kind() {
        let absolute = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());
        let displacement = GeometryValue::new(GeometryValueKind::Displacement, geometry());
        assert_eq!(absolute.kind(), GeometryValueKind::AbsoluteGeometry);
        assert_eq!(displacement.kind(), GeometryValueKind::Displacement);
    }

    #[test]
    fn branch_values_require_stable_node_identity() {
        let value = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());
        assert_eq!(
            BranchValue::new("  ", value.clone()),
            Err(EvaluationInterfaceError::EmptyIdentifier)
        );
        assert_eq!(BranchValue::new("branch-a", value).unwrap().branch_node_id(), "branch-a");
    }

    #[test]
    fn node_value_distinguishes_single_geometry_from_parallel_branches() {
        let value = GeometryValue::new(GeometryValueKind::AbsoluteGeometry, geometry());
        let single = NodeValue::Geometry(value.clone());
        let branches = NodeValue::Branches(vec![
            BranchValue::new("branch-a", value.clone()).unwrap(),
            BranchValue::new("branch-b", value).unwrap(),
        ]);
        assert!(matches!(single, NodeValue::Geometry(_)));
        assert!(matches!(branches, NodeValue::Branches(values) if values.len() == 2));
    }

    struct InvalidOperator;
    impl DeformerOperator for InvalidOperator {
        fn kind(&self) -> DeformationKind { DeformationKind::PinDeformation }
        fn algorithm_id(&self) -> &'static str { " " }
        fn algorithm_version(&self) -> u32 { 0 }
        fn evaluate(
            &self,
            _definition: &DeformationDefinition,
            input: &GeometryValue,
            _context: &OperatorContext<'_>,
        ) -> Result<GeometryValue, OperatorFailure> {
            Ok(input.clone())
        }
    }

    #[test]
    fn operator_metadata_requires_named_nonzero_algorithm_version() {
        assert_eq!(
            validate_operator(&InvalidOperator),
            Err(EvaluationInterfaceError::InvalidOperatorVersion)
        );
    }
}
