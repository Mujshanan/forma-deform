//! Paired structured-path and derived-curve output contract for FDM-05.
//!
//! The structured Path2D remains authoritative. Evaluated curve samples are a
//! separate, disposable representation for curve-accurate downstream use.

use crate::binding_evaluation::EvaluatedBindingGeometry;
use crate::curve_sample_evaluation::{EvaluatedCurveSample, EvaluatedCurveSampleSet};
use crate::path2d::{Path2D, PathCommand};
use crate::host_contract::GeometrySnapshotStamp;

/// Immutable pairing of the editable Path2D result and its evaluated curve data.
///
/// Construction checks that both products describe the same source snapshot
/// and current cage pose, and that the sample sequence covers every original
/// quadratic/cubic command in deterministic command/subpath order.
#[derive(Clone, Debug, PartialEq)]
pub struct EvaluatedDeformationOutput {
    structured: EvaluatedBindingGeometry,
    curves: EvaluatedCurveSampleSet,
}

impl EvaluatedDeformationOutput {
    pub fn new(
        structured: EvaluatedBindingGeometry,
        curves: EvaluatedCurveSampleSet,
    ) -> Result<Self, DeformationOutputError> {
        if structured.source_stamp() != curves.source_stamp() {
            return Err(DeformationOutputError::SourceStampMismatch);
        }
        if structured.binding_stamp() != curves.binding_stamp() {
            return Err(DeformationOutputError::BindingStampMismatch);
        }
        if structured.cage_geometry_fingerprint() != curves.current_cage_fingerprint() {
            return Err(DeformationOutputError::CurrentCageMismatch);
        }
        validate_curve_alignment(structured.geometry(), curves.samples())?;
        Ok(Self { structured, curves })
    }

    /// Command-preserving evaluated Path2D. This is not replaced by samples.
    pub fn structured_path(&self) -> &Path2D {
        self.structured.geometry()
    }

    /// Derived evaluated samples, grouped by original command and parameter.
    pub fn curve_samples(&self) -> &EvaluatedCurveSampleSet {
        &self.curves
    }

    pub fn source_stamp(&self) -> &GeometrySnapshotStamp {
        self.structured.source_stamp()
    }

    pub fn rest_cage_fingerprint(&self) -> &[u8; 32] {
        self.curves.rest_cage_fingerprint()
    }

    pub fn current_cage_fingerprint(&self) -> &[u8; 32] {
        self.structured.cage_geometry_fingerprint()
    }

    pub fn structured_path_fingerprint(&self) -> &crate::host_contract::GeometryFingerprint {
        self.structured.fingerprint()
    }

    pub fn mapped_features(&self) -> usize { self.structured.mapped_features() }
    pub fn clamped_features(&self) -> usize { self.structured.clamped_features() }
    pub fn extrapolated_features(&self) -> usize { self.structured.extrapolated_features() }
    pub fn unbound_features(&self) -> usize { self.structured.unbound_features() }
}

fn validate_curve_alignment(
    path: &Path2D,
    samples: &[EvaluatedCurveSample],
) -> Result<(), DeformationOutputError> {
    let mut cursor = 0usize;
    let mut subpath_index = 0u32;
    let mut saw_move_to = false;

    for (command_index, command) in path.commands().iter().enumerate() {
        match command {
            PathCommand::MoveTo(_) => {
                if saw_move_to {
                    subpath_index = subpath_index.checked_add(1)
                        .ok_or(DeformationOutputError::SubpathIndexOverflow)?;
                }
                saw_move_to = true;
            }
            PathCommand::QuadraticTo { .. } | PathCommand::CubicTo { .. } => {
                let command_index = u32::try_from(command_index)
                    .map_err(|_| DeformationOutputError::CommandIndexOverflow)?;
                let start = cursor;
                while cursor < samples.len()
                    && samples[cursor].command_index == command_index
                {
                    let sample = &samples[cursor];
                    if sample.subpath_index != subpath_index
                        || sample.sample_index != cursor - start
                        || !sample.t.is_finite()
                        || (cursor == start && sample.t != 0.0)
                        || (cursor > start && sample.t <= samples[cursor - 1].t)
                    {
                        return Err(DeformationOutputError::InvalidCurveSampleOrder {
                            command_index,
                        });
                    }
                    cursor += 1;
                }
                if cursor - start < 2 || samples[cursor - 1].t != 1.0 {
                    return Err(DeformationOutputError::IncompleteCurveSamples {
                        command_index,
                    });
                }
            }
            PathCommand::LineTo(_) | PathCommand::Close => {}
        }

        if cursor < samples.len()
            && samples[cursor].command_index < u32::try_from(command_index).unwrap_or(u32::MAX)
        {
            return Err(DeformationOutputError::UnexpectedCurveSampleCommand {
                command_index: samples[cursor].command_index,
            });
        }
    }

    if cursor != samples.len() {
        return Err(DeformationOutputError::UnexpectedCurveSampleCommand {
            command_index: samples[cursor].command_index,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeformationOutputError {
    SourceStampMismatch,
    BindingStampMismatch,
    CurrentCageMismatch,
    CommandIndexOverflow,
    SubpathIndexOverflow,
    InvalidCurveSampleOrder { command_index: u32 },
    IncompleteCurveSamples { command_index: u32 },
    UnexpectedCurveSampleCommand { command_index: u32 },
}

impl std::fmt::Display for DeformationOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceStampMismatch => write!(f, "structured path and curve samples refer to different source snapshots"),
            Self::BindingStampMismatch => write!(f, "structured path and curve samples were evaluated from different validated bindings"),
            Self::CurrentCageMismatch => write!(f, "structured path and curve samples refer to different current cage poses"),
            Self::CommandIndexOverflow => write!(f, "Path2D command index exceeds curve-sample locator capacity"),
            Self::SubpathIndexOverflow => write!(f, "Path2D subpath index exceeds curve-sample locator capacity"),
            Self::InvalidCurveSampleOrder { command_index } => write!(f, "curve samples for command {command_index} are not in canonical parameter order"),
            Self::IncompleteCurveSamples { command_index } => write!(f, "curve command {command_index} does not have a complete endpoint-inclusive sample set"),
            Self::UnexpectedCurveSampleCommand { command_index } => write!(f, "curve samples reference non-curve or missing command {command_index}"),
        }
    }
}
impl std::error::Error for DeformationOutputError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path2d::Point2;

    fn point(x: f64, y: f64) -> Point2 { Point2::new(x, y).unwrap() }

    fn sample(command_index: u32, subpath_index: u32, sample_index: usize, t: f64) -> EvaluatedCurveSample {
        EvaluatedCurveSample {
            command_index,
            subpath_index,
            sample_index,
            t,
            point: point(t, t * t),
        }
    }

    #[test]
    fn validates_mixed_curve_commands_and_multiple_subpaths() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(1.0, 0.0)),
            PathCommand::QuadraticTo { control: point(2.0, 1.0), to: point(3.0, 0.0) },
            PathCommand::Close,
            PathCommand::MoveTo(point(4.0, 0.0)),
            PathCommand::CubicTo {
                control1: point(5.0, 1.0),
                control2: point(6.0, 1.0),
                to: point(7.0, 0.0),
            },
        ]).unwrap();
        let samples = vec![
            sample(2, 0, 0, 0.0),
            sample(2, 0, 1, 0.5),
            sample(2, 0, 2, 1.0),
            sample(5, 1, 0, 0.0),
            sample(5, 1, 1, 0.5),
            sample(5, 1, 2, 1.0),
        ];
        assert_eq!(validate_curve_alignment(&path, &samples), Ok(()));
    }

    #[test]
    fn rejects_missing_curve_endpoints_and_wrong_subpath_identity() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::CubicTo {
                control1: point(1.0, 1.0),
                control2: point(2.0, 1.0),
                to: point(3.0, 0.0),
            },
        ]).unwrap();
        assert_eq!(
            validate_curve_alignment(&path, &[sample(1, 0, 0, 0.0)]),
            Err(DeformationOutputError::IncompleteCurveSamples { command_index: 1 })
        );
        assert_eq!(
            validate_curve_alignment(&path, &[
                sample(1, 2, 0, 0.0),
                sample(1, 2, 1, 1.0),
            ]),
            Err(DeformationOutputError::InvalidCurveSampleOrder { command_index: 1 })
        );
    }

    #[test]
    fn rejects_samples_for_non_curve_commands() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(1.0, 0.0)),
        ]).unwrap();
        assert_eq!(
            validate_curve_alignment(&path, &[sample(1, 0, 0, 0.0)]),
            Err(DeformationOutputError::UnexpectedCurveSampleCommand { command_index: 1 })
        );
    }
}
