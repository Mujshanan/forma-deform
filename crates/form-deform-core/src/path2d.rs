//! Canonical, host-neutral Path2D payload used by the M1 snapshot contract.
//!
//! Host adapters map authored host paths into this schema. The schema is not
//! an assertion about AniCraft's in-memory representation.

use crate::host_contract::{
    AxisOrientation, CoordinateOrigin, CoordinateSpaceDescriptor, ContractError,
    GeometryFingerprint,
};
use sha2::{Digest, Sha256};

pub const PATH2D_PAYLOAD_KIND: &str = "Path2D";
pub const PATH2D_SCHEMA_VERSION: u32 = 1;
pub const PATH2D_FINGERPRINT_ALGORITHM: &str = "fdm-path2d-sha256-v1";

/// A finite point in the declared snapshot coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    x: f64,
    y: f64,
}

impl Point2 {
    pub fn new(x: f64, y: f64) -> Result<Self, Path2DError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Path2DError::NonFiniteCoordinate);
        }
        Ok(Self { x, y })
    }

    pub fn x(self) -> f64 { self.x }
    pub fn y(self) -> f64 { self.y }
}

/// Explicit path commands. Tags and field order are part of the v1 digest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo(Point2),
    LineTo(Point2),
    QuadraticTo { control: Point2, to: Point2 },
    CubicTo { control1: Point2, control2: Point2, to: Point2 },
    Close,
}

/// Owned immutable path data. Construction validates command sequencing and
/// finite coordinates; callers can inspect commands but cannot mutate them.
#[derive(Clone, Debug, PartialEq)]
pub struct Path2D {
    commands: Vec<PathCommand>,
}

impl Path2D {
    pub fn new(commands: Vec<PathCommand>) -> Result<Self, Path2DError> {
        if commands.is_empty() {
            return Err(Path2DError::EmptyPath);
        }
        if !matches!(commands.first(), Some(PathCommand::MoveTo(_))) {
            return Err(Path2DError::MustStartWithMoveTo);
        }

        let mut has_current_point = false;
        let mut subpath_open = false;
        for command in &commands {
            match command {
                PathCommand::MoveTo(_) => {
                    has_current_point = true;
                    subpath_open = true;
                }
                PathCommand::LineTo(_)
                | PathCommand::QuadraticTo { .. }
                | PathCommand::CubicTo { .. } => {
                    if !has_current_point {
                        return Err(Path2DError::DrawWithoutCurrentPoint);
                    }
                }
                PathCommand::Close => {
                    if !subpath_open {
                        return Err(Path2DError::CloseWithoutOpenSubpath);
                    }
                    subpath_open = false;
                    has_current_point = false;
                }
            }
        }
        Ok(Self { commands })
    }

    pub fn commands(&self) -> &[PathCommand] { &self.commands }

    /// Hash the exact canonical v1 representation of this path and coordinate
    /// descriptor. The host's native representation is deliberately excluded.
    pub fn fingerprint(
        &self,
        coordinate_space: &CoordinateSpaceDescriptor,
    ) -> Result<GeometryFingerprint, ContractError> {
        coordinate_space.validate()?;
        let mut bytes = Vec::with_capacity(64 + self.commands.len() * 32);
        bytes.extend_from_slice(b"FORMA-DEFORM\0PATH2D\0");
        bytes.extend_from_slice(&PATH2D_SCHEMA_VERSION.to_le_bytes());
        encode_coordinate_space(&mut bytes, coordinate_space)?;
        put_u64(&mut bytes, self.commands.len() as u64);

        for command in &self.commands {
            match command {
                PathCommand::MoveTo(point) => {
                    bytes.push(0x01);
                    put_point(&mut bytes, *point);
                }
                PathCommand::LineTo(point) => {
                    bytes.push(0x02);
                    put_point(&mut bytes, *point);
                }
                PathCommand::QuadraticTo { control, to } => {
                    bytes.push(0x03);
                    put_point(&mut bytes, *control);
                    put_point(&mut bytes, *to);
                }
                PathCommand::CubicTo { control1, control2, to } => {
                    bytes.push(0x04);
                    put_point(&mut bytes, *control1);
                    put_point(&mut bytes, *control2);
                    put_point(&mut bytes, *to);
                }
                PathCommand::Close => bytes.push(0x05),
            }
        }

        let digest = Sha256::digest(&bytes).to_vec();
        Ok(GeometryFingerprint {
            algorithm: PATH2D_FINGERPRINT_ALGORITHM.into(),
            digest,
        })
    }
}

fn encode_coordinate_space(
    out: &mut Vec<u8>,
    space: &CoordinateSpaceDescriptor,
) -> Result<(), ContractError> {
    put_string(out, space.id.as_str())?;
    out.push(match space.kind {
        crate::host_contract::CoordinateSpaceKind::TargetLocalRest => 0x01,
    });
    out.push(space.dimensions);
    put_string(out, &space.linear_unit)?;
    put_string(out, &space.canonical_unit)?;
    put_f64(out, space.unit_scale);
    out.push(match space.axes {
        AxisOrientation::XRightYUp => 0x01,
        AxisOrientation::XRightYDown => 0x02,
    });
    out.push(match space.origin {
        CoordinateOrigin::TopLeft => 0x01,
        CoordinateOrigin::BottomLeft => 0x02,
        CoordinateOrigin::Center => 0x03,
    });
    Ok(())
}

fn put_string(out: &mut Vec<u8>, value: &str) -> Result<(), ContractError> {
    let length = u32::try_from(value.len()).map_err(|_| ContractError::InvalidFingerprint)?;
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_f64(out: &mut Vec<u8>, value: f64) {
    // Path2D points are finite by construction and unit_scale is validated.
    // Normalize both signs of zero to the same canonical bit pattern.
    let value = if value == 0.0 { 0.0 } else { value };
    out.extend_from_slice(&value.to_bits().to_le_bytes());
}

fn put_point(out: &mut Vec<u8>, point: Point2) {
    put_f64(out, point.x);
    put_f64(out, point.y);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path2DError {
    EmptyPath,
    MustStartWithMoveTo,
    DrawWithoutCurrentPoint,
    CloseWithoutOpenSubpath,
    NonFiniteCoordinate,
}

impl std::fmt::Display for Path2DError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => write!(f, "Path2D must contain at least one command"),
            Self::MustStartWithMoveTo => write!(f, "Path2D must start with MoveTo"),
            Self::DrawWithoutCurrentPoint => write!(f, "drawing command requires an active subpath"),
            Self::CloseWithoutOpenSubpath => write!(f, "Close requires an open subpath"),
            Self::NonFiniteCoordinate => write!(f, "Path2D coordinates must be finite"),
        }
    }
}

impl std::error::Error for Path2DError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_contract::{
        CoordinateSpaceId, CoordinateSpaceKind,
    };

    fn point(x: f64, y: f64) -> Point2 { Point2::new(x, y).unwrap() }

    fn space() -> CoordinateSpaceDescriptor {
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

    fn sample() -> Path2D {
        Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(10.0, 0.0)),
            PathCommand::QuadraticTo { control: point(12.0, 5.0), to: point(10.0, 10.0) },
            PathCommand::CubicTo {
                control1: point(8.0, 12.0),
                control2: point(2.0, 12.0),
                to: point(0.0, 10.0),
            },
            PathCommand::Close,
        ]).unwrap()
    }

    #[test]
    fn equivalent_paths_have_identical_fingerprints() {
        assert_eq!(sample().fingerprint(&space()).unwrap(), sample().fingerprint(&space()).unwrap());
    }

    #[test]
    fn command_order_and_segment_kind_change_fingerprint() {
        let original = sample().fingerprint(&space()).unwrap();
        let reordered = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(10.0, 0.0)),
            PathCommand::LineTo(point(10.0, 10.0)),
            PathCommand::Close,
        ]).unwrap();
        assert_ne!(original, reordered.fingerprint(&space()).unwrap());
    }

    #[test]
    fn coordinate_interpretation_is_part_of_fingerprint() {
        let path = sample();
        let original = path.fingerprint(&space()).unwrap();
        let mut changed = space();
        changed.origin = CoordinateOrigin::Center;
        assert_ne!(original, path.fingerprint(&changed).unwrap());
    }

    #[test]
    fn negative_zero_is_canonicalized() {
        let positive = Path2D::new(vec![PathCommand::MoveTo(point(0.0, 0.0))]).unwrap();
        let negative = Path2D::new(vec![PathCommand::MoveTo(point(-0.0, 0.0))]).unwrap();
        assert_eq!(positive.fingerprint(&space()).unwrap(), negative.fingerprint(&space()).unwrap());
    }

    #[test]
    fn invalid_path_sequences_and_non_finite_points_are_rejected() {
        assert_eq!(Point2::new(f64::NAN, 0.0), Err(Path2DError::NonFiniteCoordinate));
        assert_eq!(Path2D::new(vec![]), Err(Path2DError::EmptyPath));
        assert_eq!(
            Path2D::new(vec![PathCommand::LineTo(point(1.0, 1.0))]),
            Err(Path2DError::MustStartWithMoveTo),
        );
        assert_eq!(
            Path2D::new(vec![PathCommand::MoveTo(point(0.0, 0.0)), PathCommand::Close, PathCommand::Close]),
            Err(Path2DError::CloseWithoutOpenSubpath),
        );
    }

    #[test]
    fn canonical_v1_digest_matches_golden_vector() {
        let digest = sample().fingerprint(&space()).unwrap();
        let hex = digest.digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        assert_eq!(hex, "7a2dc1419d74cba04f979772df1f2da57a6d39585ebe2400fdb1307c05df4011");
    }

    #[test]
    fn digest_is_sha256_length() {
        assert_eq!(sample().fingerprint(&space()).unwrap().digest.len(), 32);
    }
}
