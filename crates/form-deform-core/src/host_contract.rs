//! Host-neutral identity and immutable geometry-snapshot contract primitives.
//!
//! These types describe data supplied by a host adapter. They do not allocate
//! host IDs, read documents, persist bindings, or apply deformation results.

/// A non-empty opaque identifier issued by the host.
macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ContractError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ContractError::EmptyIdentifier);
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

opaque_id!(HostNamespace);
opaque_id!(HostDocumentId);
opaque_id!(HostTargetKind);
opaque_id!(HostTargetId);
opaque_id!(CoordinateSpaceId);

/// Stable host identity. The generation is mandatory because host numeric IDs
/// are not assumed to be permanently non-reusable after deletion/import.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostTargetRef {
    pub namespace: HostNamespace,
    pub document_id: HostDocumentId,
    pub target_kind: HostTargetKind,
    pub target_id: HostTargetId,
    pub generation: u64,
}

impl HostTargetRef {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.generation == 0 {
            return Err(ContractError::InvalidGeneration);
        }
        Ok(())
    }
}

/// Host-issued, target-scoped source revisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GeometryRevision(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TopologyRevision(pub u64);

/// A deterministic content digest with an explicit algorithm/canonicalization ID.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GeometryFingerprint {
    pub algorithm: String,
    pub digest: Vec<u8>,
}

impl GeometryFingerprint {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.algorithm.trim().is_empty() || self.digest.is_empty() {
            return Err(ContractError::InvalidFingerprint);
        }
        if self.algorithm == crate::path2d::PATH2D_FINGERPRINT_ALGORITHM && self.digest.len() != 32 {
            return Err(ContractError::InvalidFingerprint);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CoordinateSpaceKind {
    TargetLocalRest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AxisOrientation {
    XRightYUp,
    XRightYDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CoordinateOrigin {
    TopLeft,
    BottomLeft,
    Center,
}

/// Explicit description of the coordinates carried by a geometry payload.
/// Unit names and scale are supplied by the host; FDM does not infer them.
#[derive(Clone, Debug, PartialEq)]
pub struct CoordinateSpaceDescriptor {
    pub id: CoordinateSpaceId,
    pub kind: CoordinateSpaceKind,
    pub dimensions: u8,
    pub linear_unit: String,
    /// Canonical unit named by the host for this payload coordinate space.
    pub canonical_unit: String,
    /// Canonical units per one payload linear unit.
    pub unit_scale: f64,
    pub axes: AxisOrientation,
    pub origin: CoordinateOrigin,
}

impl CoordinateSpaceDescriptor {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.dimensions != 2 {
            return Err(ContractError::UnsupportedDimensions(self.dimensions));
        }
        if self.linear_unit.trim().is_empty() || self.canonical_unit.trim().is_empty() {
            return Err(ContractError::MissingLinearUnit);
        }
        if !self.unit_scale.is_finite() || self.unit_scale <= 0.0 {
            return Err(ContractError::InvalidUnitScale);
        }
        Ok(())
    }
}

/// Immutable point-in-time Path2D input. The payload is an owned value with
/// private storage and read-only accessors; no host-backed mutable storage is
/// accepted by this M1 snapshot type.
#[derive(Clone, Debug, PartialEq)]
pub struct GeometrySnapshot {
    target: HostTargetRef,
    geometry_revision: GeometryRevision,
    topology_revision: TopologyRevision,
    fingerprint: GeometryFingerprint,
    coordinate_space: CoordinateSpaceDescriptor,
    payload_kind: String,
    payload_schema_version: u32,
    payload: crate::path2d::Path2D,
}

impl GeometrySnapshot {
    /// Construct a validated snapshot. Fields remain private so consumers can
    /// only inspect payload through shared references, preserving the
    /// immutable-by-contract evaluation boundary.
    pub fn new(
        target: HostTargetRef,
        geometry_revision: GeometryRevision,
        topology_revision: TopologyRevision,
        fingerprint: GeometryFingerprint,
        coordinate_space: CoordinateSpaceDescriptor,
        payload_kind: impl Into<String>,
        payload_schema_version: u32,
        payload: crate::path2d::Path2D,
    ) -> Result<Self, ContractError> {
        let snapshot = Self {
            target,
            geometry_revision,
            topology_revision,
            fingerprint,
            coordinate_space,
            payload_kind: payload_kind.into(),
            payload_schema_version,
            payload,
        };
        snapshot.validate_envelope()?;
        Ok(snapshot)
    }

    pub fn validate_envelope(&self) -> Result<(), ContractError> {
        self.target.validate()?;
        self.fingerprint.validate()?;
        self.coordinate_space.validate()?;
        if self.geometry_revision.0 == 0 || self.topology_revision.0 == 0 {
            return Err(ContractError::InvalidRevision);
        }
        if self.payload_kind.trim().is_empty() {
            return Err(ContractError::MissingPayloadKind);
        }
        if self.payload_kind != crate::path2d::PATH2D_PAYLOAD_KIND {
            return Err(ContractError::UnsupportedPayloadKind);
        }
        if self.payload_schema_version == 0 {
            return Err(ContractError::InvalidPayloadSchemaVersion);
        }
        if self.payload_schema_version != crate::path2d::PATH2D_SCHEMA_VERSION {
            return Err(ContractError::UnsupportedPayloadSchemaVersion);
        }
        let calculated = self.payload.fingerprint(&self.coordinate_space)?;
        if calculated != self.fingerprint {
            return Err(ContractError::FingerprintDoesNotMatchPayload);
        }
        Ok(())
    }

    pub fn target(&self) -> &HostTargetRef { &self.target }
    pub fn geometry_revision(&self) -> GeometryRevision { self.geometry_revision }
    pub fn topology_revision(&self) -> TopologyRevision { self.topology_revision }
    pub fn fingerprint(&self) -> &GeometryFingerprint { &self.fingerprint }
    pub fn coordinate_space(&self) -> &CoordinateSpaceDescriptor { &self.coordinate_space }
    pub fn payload_kind(&self) -> &str { &self.payload_kind }
    pub fn payload_schema_version(&self) -> u32 { self.payload_schema_version }
    pub fn payload(&self) -> &crate::path2d::Path2D { &self.payload }

    pub fn stamp(&self) -> GeometrySnapshotStamp {
        GeometrySnapshotStamp::new(
            self.target.clone(),
            self.geometry_revision,
            self.topology_revision,
            self.fingerprint.clone(),
        ).expect("validated snapshot always produces a valid stamp")
    }
}

/// The exact source dependency an evaluation result must report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometrySnapshotStamp {
    target: HostTargetRef,
    geometry_revision: GeometryRevision,
    topology_revision: TopologyRevision,
    fingerprint: GeometryFingerprint,
}

impl GeometrySnapshotStamp {
    pub fn new(
        target: HostTargetRef,
        geometry_revision: GeometryRevision,
        topology_revision: TopologyRevision,
        fingerprint: GeometryFingerprint,
    ) -> Result<Self, ContractError> {
        let stamp = Self { target, geometry_revision, topology_revision, fingerprint };
        stamp.validate()?;
        Ok(stamp)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        self.target.validate()?;
        if self.geometry_revision.0 == 0 || self.topology_revision.0 == 0 {
            return Err(ContractError::InvalidRevision);
        }
        self.fingerprint.validate()
    }

    pub fn target(&self) -> &HostTargetRef { &self.target }
    pub fn geometry_revision(&self) -> GeometryRevision { self.geometry_revision }
    pub fn topology_revision(&self) -> TopologyRevision { self.topology_revision }
    pub fn fingerprint(&self) -> &GeometryFingerprint { &self.fingerprint }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotFreshness {
    pub geometry_changed: bool,
    pub topology_changed: bool,
    pub fingerprint_changed: bool,
    pub geometry_revision_regressed: bool,
    pub topology_revision_regressed: bool,
}

impl SnapshotFreshness {
    pub fn is_current(self) -> bool {
        !self.geometry_changed && !self.topology_changed && !self.fingerprint_changed
    }

    /// Equal revisions with different content indicate a host snapshot-integrity
    /// failure rather than an ordinary source edit.
    pub fn is_fingerprint_integrity_error(self) -> bool {
        self.fingerprint_changed && !self.geometry_changed && !self.topology_changed
    }

    /// Compare a captured dependency with the host's current source state.
    /// A different target identity is always stale, even if revisions happen
    /// to have the same numeric values.
    pub fn compare(
        captured: &GeometrySnapshotStamp,
        current: &GeometrySnapshotStamp,
    ) -> Result<Self, ContractError> {
        captured.validate()?;
        current.validate()?;
        if captured.target != current.target {
            return Err(ContractError::TargetIdentityMismatch);
        }
        Ok(Self {
            geometry_changed: captured.geometry_revision != current.geometry_revision,
            topology_changed: captured.topology_revision != current.topology_revision,
            fingerprint_changed: captured.fingerprint != current.fingerprint,
            geometry_revision_regressed: current.geometry_revision.0 < captured.geometry_revision.0,
            topology_revision_regressed: current.topology_revision.0 < captured.topology_revision.0,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContractError {
    EmptyIdentifier,
    InvalidFingerprint,
    UnsupportedDimensions(u8),
    MissingLinearUnit,
    InvalidUnitScale,
    MissingPayloadKind,
    InvalidPayloadSchemaVersion,
    UnsupportedPayloadKind,
    UnsupportedPayloadSchemaVersion,
    FingerprintDoesNotMatchPayload,
    InvalidRevision,
    InvalidGeneration,
    TargetIdentityMismatch,
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIdentifier => write!(f, "host identifiers must not be empty"),
            Self::InvalidFingerprint => write!(f, "fingerprint requires an algorithm and digest"),
            Self::UnsupportedDimensions(n) => write!(f, "unsupported geometry dimensionality: {n}"),
            Self::MissingLinearUnit => write!(f, "coordinate space must declare a linear unit"),
            Self::InvalidUnitScale => write!(f, "coordinate unit scale must be finite and positive"),
            Self::MissingPayloadKind => write!(f, "geometry payload kind must not be empty"),
            Self::InvalidPayloadSchemaVersion => write!(f, "payload schema version must be non-zero"),
            Self::UnsupportedPayloadKind => write!(f, "unsupported geometry payload kind"),
            Self::UnsupportedPayloadSchemaVersion => write!(f, "unsupported Path2D payload schema version"),
            Self::FingerprintDoesNotMatchPayload => write!(f, "geometry fingerprint does not match the canonical payload"),
            Self::InvalidRevision => write!(f, "geometry and topology revisions must start at one"),
            Self::InvalidGeneration => write!(f, "target generation must be non-zero"),
            Self::TargetIdentityMismatch => write!(f, "snapshot target identities do not match"),
        }
    }
}

impl std::error::Error for ContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(generation: u64) -> HostTargetRef {
        HostTargetRef {
            namespace: HostNamespace::new("anicraft").unwrap(),
            document_id: HostDocumentId::new("doc-uuid").unwrap(),
            target_kind: HostTargetKind::new("shape").unwrap(),
            target_id: HostTargetId::new("element:42").unwrap(),
            generation,
        }
    }

    fn path() -> crate::path2d::Path2D {
        crate::path2d::Path2D::new(vec![
            crate::path2d::PathCommand::MoveTo(crate::path2d::Point2::new(0.0, 0.0).unwrap()),
        ]).unwrap()
    }

    fn stamp(generation: u64, geometry: u64, topology: u64, digest: u8) -> GeometrySnapshotStamp {
        GeometrySnapshotStamp::new(
            target(generation),
            GeometryRevision(geometry),
            TopologyRevision(topology),
            GeometryFingerprint {
                algorithm: "sha256-canonical-v1".into(),
                digest: vec![digest; 32],
            },
        ).unwrap()
    }

    #[test]
    fn opaque_ids_reject_empty_values_without_reinterpreting_them() {
        assert_eq!(HostTargetId::new("  "), Err(ContractError::EmptyIdentifier));
        let id = HostTargetId::new("00042").unwrap();
        assert_eq!(id.as_str(), "00042");
    }

    #[test]
    fn generation_is_part_of_persistent_target_identity() {
        assert_ne!(target(1), target(2));
    }

    #[test]
    fn snapshot_freshness_detects_each_source_change_independently() {
        let captured = stamp(1, 4, 7, 9);
        let geometry = SnapshotFreshness::compare(&captured, &stamp(1, 5, 7, 9)).unwrap();
        assert_eq!(geometry, SnapshotFreshness {
            geometry_changed: true,
            topology_changed: false,
            fingerprint_changed: false,
            geometry_revision_regressed: false,
            topology_revision_regressed: false,
        });
        let topology = SnapshotFreshness::compare(&captured, &stamp(1, 4, 8, 9)).unwrap();
        assert!(topology.topology_changed);
        let fingerprint = SnapshotFreshness::compare(&captured, &stamp(1, 4, 7, 8)).unwrap();
        assert!(fingerprint.fingerprint_changed);
        assert!(fingerprint.is_fingerprint_integrity_error());
    }

    #[test]
    fn target_identity_mismatch_is_never_reported_as_current() {
        assert_eq!(
            SnapshotFreshness::compare(&stamp(1, 4, 7, 9), &stamp(2, 4, 7, 9)),
            Err(ContractError::TargetIdentityMismatch)
        );
    }

    #[test]
    fn equal_snapshot_stamps_are_current() {
        let captured = stamp(1, 4, 7, 9);
        let freshness = SnapshotFreshness::compare(&captured, &captured).unwrap();
        assert!(freshness.is_current());
    }

    #[test]
    fn coordinate_space_rejects_ambiguous_or_invalid_units() {
        let descriptor = CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("shape-local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        };
        assert!(descriptor.validate().is_ok());
        let mut invalid = descriptor;
        invalid.unit_scale = f64::NAN;
        assert_eq!(invalid.validate(), Err(ContractError::InvalidUnitScale));
    }

    #[test]
    fn snapshot_constructor_rejects_zero_revisions_and_generation() {
        let coordinate_space = CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("shape-local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        };
        let fingerprint = path().fingerprint(&coordinate_space).unwrap();
        assert_eq!(
            GeometrySnapshot::new(
                target(1), GeometryRevision(0), TopologyRevision(1),
                fingerprint.clone(), coordinate_space.clone(), "Path2D", 1, path(),
            ).unwrap_err(),
            ContractError::InvalidRevision
        );
        assert_eq!(
            GeometrySnapshot::new(
                target(0), GeometryRevision(1), TopologyRevision(1),
                fingerprint, coordinate_space, "Path2D", 1, path(),
            ).unwrap_err(),
            ContractError::InvalidGeneration
        );
    }

    #[test]
    fn snapshot_accepts_matching_path2d_payload_and_fingerprint() {
        let coordinate_space = CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("shape-local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        };
        let payload = path();
        let fingerprint = payload.fingerprint(&coordinate_space).unwrap();
        let snapshot = GeometrySnapshot::new(
            target(1), GeometryRevision(1), TopologyRevision(1),
            fingerprint, coordinate_space, "Path2D", 1, payload,
        ).unwrap();
        assert_eq!(snapshot.payload_kind(), "Path2D");
        assert_eq!(snapshot.payload_schema_version(), 1);
        assert_eq!(snapshot.payload().commands().len(), 1);
        assert!(snapshot.stamp().validate().is_ok());
    }

    #[test]
    fn snapshot_rejects_unknown_payload_kind_and_schema() {
        let coordinate_space = CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("shape-local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        };
        let payload = path();
        let fingerprint = payload.fingerprint(&coordinate_space).unwrap();
        assert_eq!(
            GeometrySnapshot::new(
                target(1), GeometryRevision(1), TopologyRevision(1),
                fingerprint.clone(), coordinate_space.clone(), "Mesh", 1, path(),
            ).unwrap_err(),
            ContractError::UnsupportedPayloadKind
        );
        assert_eq!(
            GeometrySnapshot::new(
                target(1), GeometryRevision(1), TopologyRevision(1),
                fingerprint, coordinate_space, "Path2D", 2, path(),
            ).unwrap_err(),
            ContractError::UnsupportedPayloadSchemaVersion
        );
    }

    #[test]
    fn snapshot_rejects_fingerprint_that_does_not_match_its_payload() {
        let coordinate_space = CoordinateSpaceDescriptor {
            id: CoordinateSpaceId::new("shape-local-rest").unwrap(),
            kind: CoordinateSpaceKind::TargetLocalRest,
            dimensions: 2,
            linear_unit: "px".into(),
            canonical_unit: "px".into(),
            unit_scale: 1.0,
            axes: AxisOrientation::XRightYDown,
            origin: CoordinateOrigin::TopLeft,
        };
        let mut fingerprint = path().fingerprint(&coordinate_space).unwrap();
        fingerprint.digest[0] ^= 0xff;
        assert_eq!(
            GeometrySnapshot::new(
                target(1), GeometryRevision(1), TopologyRevision(1),
                fingerprint, coordinate_space, "Path2D", 1, path(),
            ).unwrap_err(),
            ContractError::FingerprintDoesNotMatchPayload
        );
    }

    #[test]
    fn freshness_classifies_revision_regression() {
        let captured = stamp(1, 8, 6, 9);
        let current = stamp(1, 7, 5, 9);
        let freshness = SnapshotFreshness::compare(&captured, &current).unwrap();
        assert!(!freshness.is_current());
        assert!(freshness.geometry_revision_regressed);
        assert!(freshness.topology_revision_regressed);
    }

    #[test]
    fn fingerprint_requires_declared_algorithm_and_content_digest() {
        assert!(GeometryFingerprint {
            algorithm: "sha256-v1".into(),
            digest: vec![1; 32],
        }.validate().is_ok());
        assert_eq!(GeometryFingerprint {
            algorithm: String::new(),
            digest: vec![1; 32],
        }.validate(), Err(ContractError::InvalidFingerprint));
        assert_eq!(GeometryFingerprint {
            algorithm: crate::path2d::PATH2D_FINGERPRINT_ALGORITHM.into(),
            digest: vec![1; 31],
        }.validate(), Err(ContractError::InvalidFingerprint));
    }
}
