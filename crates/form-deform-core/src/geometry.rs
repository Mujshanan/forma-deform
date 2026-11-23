//! Thin geometry views over the canonical host snapshot contract.
//!
//! This module deliberately does not define a second geometry payload, target
//! identity, revision, fingerprint, or result type. `GeometrySnapshot` and
//! `Path2D` remain the authoritative input contracts; evaluated outputs use
//! `EvaluatedGeometryResult`.

use crate::host_contract::{
    ContractError, CoordinateSpaceDescriptor, GeometrySnapshot, GeometrySnapshotStamp,
    HostTargetRef,
};
use crate::path2d::Path2D;

/// A borrowed, validated view of one immutable source snapshot.
///
/// The wrapper owns no geometry and cannot outlive its source snapshot. It
/// exists to give operators a narrow, read-only geometry-facing API without
/// copying or replacing the canonical host contract.
#[derive(Clone, Copy, Debug)]
pub struct GeometryInput<'a> {
    snapshot: &'a GeometrySnapshot,
}

impl<'a> GeometryInput<'a> {
    /// Validate the snapshot envelope before exposing it to an operator.
    pub fn new(snapshot: &'a GeometrySnapshot) -> Result<Self, GeometryContractError> {
        snapshot
            .validate_envelope()
            .map_err(GeometryContractError::InvalidSnapshot)?;
        Ok(Self { snapshot })
    }

    pub fn path(&self) -> &'a Path2D {
        self.snapshot.payload()
    }

    pub fn target(&self) -> &'a HostTargetRef {
        self.snapshot.target()
    }

    pub fn coordinate_space(&self) -> &'a CoordinateSpaceDescriptor {
        self.snapshot.coordinate_space()
    }

    pub fn stamp(&self) -> GeometrySnapshotStamp {
        self.snapshot.stamp()
    }

    /// Access the original snapshot when a downstream contract needs fields
    /// not surfaced directly by this convenience view.
    pub fn snapshot(&self) -> &'a GeometrySnapshot {
        self.snapshot
    }
}

/// Require exact coordinate-space identity and interpretation.
///
/// FDM-01 does not perform implicit unit conversion, axis flips, or origin
/// translation. A future conversion must be an explicit, separately tested
/// operation. Exact descriptor equality prevents silent unit/axis/origin drift.
pub fn require_compatible_coordinate_spaces(
    expected: &CoordinateSpaceDescriptor,
    actual: &CoordinateSpaceDescriptor,
) -> Result<(), GeometryContractError> {
    expected
        .validate()
        .map_err(GeometryContractError::InvalidCoordinateSpace)?;
    actual
        .validate()
        .map_err(GeometryContractError::InvalidCoordinateSpace)?;

    if expected != actual {
        return Err(GeometryContractError::CoordinateSpaceMismatch);
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeometryContractError {
    InvalidSnapshot(ContractError),
    InvalidCoordinateSpace(ContractError),
    CoordinateSpaceMismatch,
}

impl std::fmt::Display for GeometryContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSnapshot(error) => write!(f, "invalid geometry snapshot: {error}"),
            Self::InvalidCoordinateSpace(error) => {
                write!(f, "invalid coordinate-space descriptor: {error}")
            }
            Self::CoordinateSpaceMismatch => {
                write!(f, "geometry coordinate spaces differ; explicit conversion is required")
            }
        }
    }
}

impl std::error::Error for GeometryContractError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_contract::{
        AxisOrientation, CoordinateOrigin, CoordinateSpaceId, CoordinateSpaceKind,
        GeometryRevision, HostDocumentId, HostNamespace, HostTargetId, HostTargetKind,
        TopologyRevision,
    };
    use crate::path2d::{PathCommand, Point2, PATH2D_PAYLOAD_KIND, PATH2D_SCHEMA_VERSION};

    fn coordinate_space() -> CoordinateSpaceDescriptor {
        CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("target-local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        }
    }

    fn snapshot() -> GeometrySnapshot {
        let coordinate_space = coordinate_space();
        let path = Path2D::new(vec![
            PathCommand::MoveTo(Point2::new(1.0, 2.0).unwrap()),
            PathCommand::CubicTo {
                control1: Point2::new(2.0, 3.0).unwrap(),
                control2: Point2::new(4.0, 3.0).unwrap(),
                to: Point2::new(5.0, 2.0).unwrap(),
            },
            PathCommand::Close,
        ])
        .unwrap();
        let fingerprint = path.fingerprint(&coordinate_space).unwrap();

        GeometrySnapshot::new(
            HostTargetRef {
                namespace: HostNamespace::new("anicraft").unwrap(),
                document_id: HostDocumentId::new("document-1").unwrap(),
                target_kind: HostTargetKind::new("shape").unwrap(),
                target_id: HostTargetId::new("shape-1").unwrap(),
                generation: 1,
            },
            GeometryRevision(1),
            TopologyRevision(1),
            fingerprint,
            coordinate_space,
            PATH2D_PAYLOAD_KIND,
            PATH2D_SCHEMA_VERSION,
            path,
        )
        .unwrap()
    }

    #[test]
    fn geometry_input_borrows_the_canonical_path_without_rebuilding_it() {
        let snapshot = snapshot();
        let input = GeometryInput::new(&snapshot).unwrap();

        assert_eq!(input.path(), snapshot.payload());
        assert_eq!(input.target(), snapshot.target());
        assert_eq!(input.stamp(), snapshot.stamp());
        assert_eq!(input.snapshot(), &snapshot);
        assert!(matches!(
            input.path().commands()[1],
            PathCommand::CubicTo { .. }
        ));
    }

    #[test]
    fn exact_coordinate_space_match_is_accepted() {
        let descriptor = coordinate_space();
        assert_eq!(
            require_compatible_coordinate_spaces(&descriptor, &descriptor),
            Ok(())
        );
    }

    #[test]
    fn differing_units_axes_or_origin_are_rejected_without_conversion() {
        let expected = coordinate_space();

        let mut different_unit = coordinate_space();
        different_unit.linear_unit = "mm".into();
        assert_eq!(
            require_compatible_coordinate_spaces(&expected, &different_unit),
            Err(GeometryContractError::CoordinateSpaceMismatch)
        );

        let mut different_axes = coordinate_space();
        different_axes.axes = AxisOrientation::XRightYUp;
        assert_eq!(
            require_compatible_coordinate_spaces(&expected, &different_axes),
            Err(GeometryContractError::CoordinateSpaceMismatch)
        );

        let mut different_origin = coordinate_space();
        different_origin.origin = CoordinateOrigin::Center;
        assert_eq!(
            require_compatible_coordinate_spaces(&expected, &different_origin),
            Err(GeometryContractError::CoordinateSpaceMismatch)
        );
    }

    #[test]
    fn invalid_coordinate_descriptor_is_rejected_before_comparison() {
        let expected = coordinate_space();
        let mut invalid = coordinate_space();
        invalid.unit_scale = f64::NAN;

        assert!(matches!(
            require_compatible_coordinate_spaces(&expected, &invalid),
            Err(GeometryContractError::InvalidCoordinateSpace(
                ContractError::InvalidUnitScale
            ))
        ));
    }
}
