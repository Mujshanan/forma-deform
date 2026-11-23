//! Deterministic adaptive sampling for authored Path2D Bézier segments.
//!
//! Samples are temporary derived geometry. They do not replace Path2D commands
//! and their indices are not persistent source-feature identities.

use crate::path2d::{Path2D, PathCommand, Point2};

pub const CURVE_SAMPLING_POLICY_VERSION: u32 = 1;
const MAX_SUBDIVISION_DEPTH: u8 = 32;
const MAX_TOTAL_SAMPLES: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveSamplingPolicy {
    /// Maximum source-space distance from a Bézier segment to its emitted chord.
    pub max_deviation: f64,
    /// Maximum number of recursive midpoint subdivisions along any branch.
    pub max_subdivision_depth: u8,
    /// Maximum emitted samples for one authored curve command, including endpoints.
    pub max_samples_per_curve: usize,
    /// Maximum emitted samples across the complete Path2D snapshot.
    pub max_total_samples: usize,
}

impl CurveSamplingPolicy {
    pub fn validate(self) -> Result<(), CurveSamplingError> {
        if !self.max_deviation.is_finite()
            || self.max_deviation <= 0.0
            || self.max_subdivision_depth > MAX_SUBDIVISION_DEPTH
            || self.max_samples_per_curve < 2
            || self.max_total_samples < 2
            || self.max_total_samples > MAX_TOTAL_SAMPLES
            || self.max_samples_per_curve > self.max_total_samples
        {
            return Err(CurveSamplingError::InvalidPolicy);
        }
        Ok(())
    }
}

impl Default for CurveSamplingPolicy {
    fn default() -> Self {
        Self {
            max_deviation: 0.25,
            max_subdivision_depth: 16,
            max_samples_per_curve: 65_537,
            max_total_samples: MAX_TOTAL_SAMPLES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveSample {
    /// Parameter on the original authored command, in the closed interval [0, 1].
    pub t: f64,
    pub point: Point2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurveSegmentSamples {
    /// Zero-based Path2D command index; valid only for the stamped source snapshot.
    pub command_index: u32,
    /// Zero-based subpath ordinal, matching source-feature enumeration.
    pub subpath_index: u32,
    /// Ordered samples, always including the authored segment endpoints.
    pub samples: Vec<CurveSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurveSampleSet {
    /// Explicit algorithm identity for cache and persistence contracts.
    pub policy_version: u32,
    /// Exact policy values used to produce this set.
    pub policy: CurveSamplingPolicy,
    /// Curved commands in original Path2D command order. Lines are not subdivided.
    pub segments: Vec<CurveSegmentSamples>,
    pub total_samples: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CurveSamplingError {
    InvalidPolicy,
    TooManyCommands,
    InvalidPathSequence { command_index: usize },
    SubdivisionDepthExceeded { command_index: usize },
    PerCurveSampleBudgetExceeded { command_index: usize },
    TotalSampleBudgetExceeded,
    ArithmeticOverflow,
}

impl std::fmt::Display for CurveSamplingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Path2D curve sampling failed: {self:?}")
    }
}
impl std::error::Error for CurveSamplingError {}

/// Adaptively sample every quadratic and cubic Path2D command.
///
/// The flatness test uses the maximum distance of the interior control points
/// to the endpoint segment. By the Bézier convex-hull property, this bounds
/// the curve-to-chord deviation for each emitted segment. Subdivision is
/// deterministic, midpoint-based, and left-to-right. Any exhausted budget
/// returns an error; a partial sample set is never returned.
pub fn sample_path_curves(
    path: &Path2D,
    policy: CurveSamplingPolicy,
) -> Result<CurveSampleSet, CurveSamplingError> {
    policy.validate()?;
    if path.commands().len() > u32::MAX as usize {
        return Err(CurveSamplingError::TooManyCommands);
    }

    let mut segments = Vec::new();
    let mut current = None;
    let mut subpath_start = None;
    let mut subpath_index = 0u32;
    let mut has_seen_move = false;
    let mut total_samples = 0usize;

    for (command_index, command) in path.commands().iter().enumerate() {
        match command {
            PathCommand::MoveTo(point) => {
                if has_seen_move {
                    subpath_index = subpath_index.checked_add(1)
                        .ok_or(CurveSamplingError::ArithmeticOverflow)?;
                }
                has_seen_move = true;
                current = Some(*point);
                subpath_start = Some(*point);
            }
            PathCommand::LineTo(point) => {
                if current.is_none() {
                    return Err(CurveSamplingError::InvalidPathSequence { command_index });
                }
                current = Some(*point);
            }
            PathCommand::QuadraticTo { control, to } => {
                let from = current.ok_or(CurveSamplingError::InvalidPathSequence { command_index })?;
                let samples = sample_quadratic(
                    command_index, from, *control, *to, policy, &mut total_samples,
                )?;
                segments.push(CurveSegmentSamples {
                    command_index: command_index as u32,
                    subpath_index,
                    samples,
                });
                current = Some(*to);
            }
            PathCommand::CubicTo { control1, control2, to } => {
                let from = current.ok_or(CurveSamplingError::InvalidPathSequence { command_index })?;
                let samples = sample_cubic(
                    command_index, from, *control1, *control2, *to, policy, &mut total_samples,
                )?;
                segments.push(CurveSegmentSamples {
                    command_index: command_index as u32,
                    subpath_index,
                    samples,
                });
                current = Some(*to);
            }
            PathCommand::Close => {
                if current.is_none() || subpath_start.is_none() {
                    return Err(CurveSamplingError::InvalidPathSequence { command_index });
                }
                current = None;
                subpath_start = None;
            }
        }
    }

    Ok(CurveSampleSet {
        policy_version: CURVE_SAMPLING_POLICY_VERSION,
        policy,
        segments,
        total_samples,
    })
}

fn sample_quadratic(
    command_index: usize,
    p0: Point2,
    p1: Point2,
    p2: Point2,
    policy: CurveSamplingPolicy,
    total_samples: &mut usize,
) -> Result<Vec<CurveSample>, CurveSamplingError> {
    let mut output = Vec::new();
    push_sample(command_index, &mut output, total_samples, policy, 0.0, p0)?;
    subdivide_quadratic(
        command_index, [p0, p1, p2], 0.0, 1.0, 0, policy, &mut output, total_samples,
    )?;
    Ok(output)
}

fn sample_cubic(
    command_index: usize,
    p0: Point2,
    p1: Point2,
    p2: Point2,
    p3: Point2,
    policy: CurveSamplingPolicy,
    total_samples: &mut usize,
) -> Result<Vec<CurveSample>, CurveSamplingError> {
    let mut output = Vec::new();
    push_sample(command_index, &mut output, total_samples, policy, 0.0, p0)?;
    subdivide_cubic(
        command_index, [p0, p1, p2, p3], 0.0, 1.0, 0, policy, &mut output, total_samples,
    )?;
    Ok(output)
}

fn subdivide_quadratic(
    command_index: usize,
    curve: [Point2; 3],
    t0: f64,
    t1: f64,
    depth: u8,
    policy: CurveSamplingPolicy,
    output: &mut Vec<CurveSample>,
    total_samples: &mut usize,
) -> Result<(), CurveSamplingError> {
    if point_segment_distance(curve[1], curve[0], curve[2])? <= policy.max_deviation {
        return push_sample(command_index, output, total_samples, policy, t1, curve[2]);
    }
    if depth >= policy.max_subdivision_depth {
        return Err(CurveSamplingError::SubdivisionDepthExceeded { command_index });
    }

    let p01 = midpoint(curve[0], curve[1])?;
    let p12 = midpoint(curve[1], curve[2])?;
    let middle = midpoint(p01, p12)?;
    let tm = midpoint_scalar(t0, t1);

    subdivide_quadratic(
        command_index, [curve[0], p01, middle], t0, tm, depth + 1,
        policy, output, total_samples,
    )?;
    subdivide_quadratic(
        command_index, [middle, p12, curve[2]], tm, t1, depth + 1,
        policy, output, total_samples,
    )
}

fn subdivide_cubic(
    command_index: usize,
    curve: [Point2; 4],
    t0: f64,
    t1: f64,
    depth: u8,
    policy: CurveSamplingPolicy,
    output: &mut Vec<CurveSample>,
    total_samples: &mut usize,
) -> Result<(), CurveSamplingError> {
    let flatness = point_segment_distance(curve[1], curve[0], curve[3])?
        .max(point_segment_distance(curve[2], curve[0], curve[3])?);
    if flatness <= policy.max_deviation {
        return push_sample(command_index, output, total_samples, policy, t1, curve[3]);
    }
    if depth >= policy.max_subdivision_depth {
        return Err(CurveSamplingError::SubdivisionDepthExceeded { command_index });
    }

    let p01 = midpoint(curve[0], curve[1])?;
    let p12 = midpoint(curve[1], curve[2])?;
    let p23 = midpoint(curve[2], curve[3])?;
    let p012 = midpoint(p01, p12)?;
    let p123 = midpoint(p12, p23)?;
    let middle = midpoint(p012, p123)?;
    let tm = midpoint_scalar(t0, t1);

    subdivide_cubic(
        command_index, [curve[0], p01, p012, middle], t0, tm, depth + 1,
        policy, output, total_samples,
    )?;
    subdivide_cubic(
        command_index, [middle, p123, p23, curve[3]], tm, t1, depth + 1,
        policy, output, total_samples,
    )
}

fn push_sample(
    command_index: usize,
    output: &mut Vec<CurveSample>,
    total_samples: &mut usize,
    policy: CurveSamplingPolicy,
    t: f64,
    point: Point2,
) -> Result<(), CurveSamplingError> {
    if output.len() >= policy.max_samples_per_curve {
        return Err(CurveSamplingError::PerCurveSampleBudgetExceeded { command_index });
    }
    if *total_samples >= policy.max_total_samples {
        return Err(CurveSamplingError::TotalSampleBudgetExceeded);
    }
    if !t.is_finite() || !(0.0..=1.0).contains(&t)
        || !point.x().is_finite() || !point.y().is_finite()
    {
        return Err(CurveSamplingError::ArithmeticOverflow);
    }
    output.push(CurveSample { t, point });
    *total_samples += 1;
    Ok(())
}

fn midpoint(a: Point2, b: Point2) -> Result<Point2, CurveSamplingError> {
    Point2::new(a.x() * 0.5 + b.x() * 0.5, a.y() * 0.5 + b.y() * 0.5)
        .map_err(|_| CurveSamplingError::ArithmeticOverflow)
}

fn midpoint_scalar(a: f64, b: f64) -> f64 {
    a * 0.5 + b * 0.5
}

/// Scale-normalized point-to-segment distance avoids squaring large coordinates.
fn point_segment_distance(
    point: Point2,
    start: Point2,
    end: Point2,
) -> Result<f64, CurveSamplingError> {
    let scale = point.x().abs().max(point.y().abs())
        .max(start.x().abs()).max(start.y().abs())
        .max(end.x().abs()).max(end.y().abs());
    if scale == 0.0 {
        return Ok(0.0);
    }

    let px = point.x() / scale;
    let py = point.y() / scale;
    let ax = start.x() / scale;
    let ay = start.y() / scale;
    let bx = end.x() / scale;
    let by = end.y() / scale;
    let dx = bx - ax;
    let dy = by - ay;
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / length_squared).clamp(0.0, 1.0)
    };
    let distance = ((px - (ax + t * dx)).hypot(py - (ay + t * dy))) * scale;
    if distance.is_finite() {
        Ok(distance)
    } else {
        Err(CurveSamplingError::ArithmeticOverflow)
    }
}

/// Transient mapping status for one adaptive curve sample. These records
/// are derived from the exact source snapshot and are not persistent point IDs.
#[derive(Clone, Debug, PartialEq)]
pub enum CurveSampleMapping {
    Mapped {
        face_index: u32,
        subface_index: u8,
        control_point_ids: Vec<crate::topology::ControlPointId>,
        coordinates: crate::binding::FaceCoordinates,
    },
    Clamped {
        face_index: u32,
        subface_index: u8,
        control_point_ids: Vec<crate::topology::ControlPointId>,
        coordinates: crate::binding::FaceCoordinates,
    },
    Extrapolated {
        face_index: u32,
        subface_index: u8,
        control_point_ids: Vec<crate::topology::ControlPointId>,
        coordinates: crate::binding::FaceCoordinates,
        boundary_offset: [f64; 2],
    },
    Unbound,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappedCurveSample {
    pub command_index: u32,
    pub subpath_index: u32,
    pub sample_index: usize,
    pub t: f64,
    pub point: Point2,
    pub mapping: CurveSampleMapping,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappedCurveSampleSet {
    pub policy_version: u32,
    pub policy: CurveSamplingPolicy,
    pub samples: Vec<MappedCurveSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CurveMappingError {
    PointLocation(crate::point_location::PointLocationError),
    OutsideCage(crate::outside_policy::OutsideCageError),
    InvalidFaceIndex(u32),
    InvalidSampleSet,
}

/// Map every adaptive sample using the same deterministic point locator and
/// explicit outside-cage policy used for authored source features. Failure
/// returns no partial mapped set. Samples remain transient and snapshot-local.
pub fn map_curve_samples(
    sample_set: &CurveSampleSet,
    geometry: &crate::binding::ValidatedCageGeometry,
    outside_policy: crate::binding::OutsideCagePolicy,
) -> Result<MappedCurveSampleSet, CurveMappingError> {
    use crate::outside_policy::{resolve_outside_point, OutsideCageResolution};
    use crate::point_location::{locate_point, PointLocationOutcome};

    if sample_set.policy_version != CURVE_SAMPLING_POLICY_VERSION
        || sample_set.policy.validate().is_err()
    {
        return Err(CurveMappingError::InvalidSampleSet);
    }

    if let crate::binding::OutsideCagePolicy::BoundedExtrapolation { max_distance } = outside_policy
        && (!max_distance.is_finite() || max_distance <= 0.0)
    {
        return Err(CurveMappingError::OutsideCage(
            crate::outside_policy::OutsideCageError::InvalidDistanceLimit,
        ));
    }

    let mut actual_total = 0usize;
    let mut previous_command = None;
    for segment in &sample_set.segments {
        if previous_command.is_some_and(|previous| previous >= segment.command_index)
            || segment.samples.len() < 2
            || segment.samples.first().is_none_or(|sample| sample.t != 0.0)
            || segment.samples.last().is_none_or(|sample| sample.t != 1.0)
            || segment.samples.windows(2).any(|pair| {
                !pair[0].t.is_finite() || !pair[1].t.is_finite() || pair[0].t >= pair[1].t
            })
        {
            return Err(CurveMappingError::InvalidSampleSet);
        }
        actual_total = actual_total.checked_add(segment.samples.len())
            .ok_or(CurveMappingError::InvalidSampleSet)?;
        if actual_total > sample_set.policy.max_total_samples
            || actual_total > MAX_TOTAL_SAMPLES
            || segment.samples.len() > sample_set.policy.max_samples_per_curve
        {
            return Err(CurveMappingError::InvalidSampleSet);
        }
        previous_command = Some(segment.command_index);
    }
    if actual_total != sample_set.total_samples {
        return Err(CurveMappingError::InvalidSampleSet);
    }

    let mut mapped = Vec::new();
    for segment in &sample_set.segments {
        for (sample_index, sample) in segment.samples.iter().enumerate() {
            let mapping = match locate_point(sample.point, geometry)
                .map_err(CurveMappingError::PointLocation)? {
                PointLocationOutcome::Located(face) => CurveSampleMapping::Mapped {
                    face_index: face.face_index,
                    subface_index: face.subface_index,
                    control_point_ids: face.control_point_ids,
                    coordinates: face.coordinates,
                },
                PointLocationOutcome::OutsideCage => {
                    match resolve_outside_point(sample.point, geometry, outside_policy)
                        .map_err(CurveMappingError::OutsideCage)? {
                        OutsideCageResolution::Unbound => CurveSampleMapping::Unbound,
                        OutsideCageResolution::Clamped(boundary) => {
                            let ids = geometry.face(boundary.face_index as usize)
                                .ok_or(CurveMappingError::InvalidFaceIndex(boundary.face_index))?
                                .control_point_ids().to_vec();
                            CurveSampleMapping::Clamped {
                                face_index: boundary.face_index,
                                subface_index: 0,
                                control_point_ids: ids,
                                coordinates: boundary.coordinates,
                            }
                        }
                        OutsideCageResolution::Extrapolated { boundary, coordinates, offset } => {
                            let ids = geometry.face(boundary.face_index as usize)
                                .ok_or(CurveMappingError::InvalidFaceIndex(boundary.face_index))?
                                .control_point_ids().to_vec();
                            CurveSampleMapping::Extrapolated {
                                face_index: boundary.face_index,
                                subface_index: 0,
                                control_point_ids: ids,
                                coordinates,
                                boundary_offset: offset,
                            }
                        }
                    }
                }
            };
            mapped.push(MappedCurveSample {
                command_index: segment.command_index,
                subpath_index: segment.subpath_index,
                sample_index,
                t: sample.t,
                point: sample.point,
                mapping,
            });
        }
    }
    Ok(MappedCurveSampleSet {
        policy_version: sample_set.policy_version,
        policy: sample_set.policy,
        samples: mapped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path2d::PathCommand;
    use crate::topology::TopologyView;

    fn point(x: f64, y: f64) -> Point2 {
        Point2::new(x, y).unwrap()
    }

    fn policy(max_deviation: f64) -> CurveSamplingPolicy {
        CurveSamplingPolicy {
            max_deviation,
            max_subdivision_depth: 16,
            max_samples_per_curve: 10_000,
            max_total_samples: 20_000,
        }
    }

    #[test]
    fn straight_quadratic_emits_only_its_endpoints() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::QuadraticTo { control: point(5.0, 0.0), to: point(10.0, 0.0) },
        ]).unwrap();
        let sampled = sample_path_curves(&path, policy(0.01)).unwrap();
        assert_eq!(sampled.total_samples, 2);
        assert_eq!(sampled.segments[0].samples.len(), 2);
        assert_eq!(sampled.segments[0].samples[0].t, 0.0);
        assert_eq!(sampled.segments[0].samples[1].t, 1.0);
    }

    #[test]
    fn curved_quadratic_subdivides_deterministically() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::QuadraticTo { control: point(5.0, 10.0), to: point(10.0, 0.0) },
        ]).unwrap();
        let first = sample_path_curves(&path, policy(0.1)).unwrap();
        let second = sample_path_curves(&path, policy(0.1)).unwrap();
        assert_eq!(first, second);
        assert!(first.segments[0].samples.len() > 2);
        assert_eq!(first.segments[0].samples.first().unwrap().point, point(0.0, 0.0));
        assert_eq!(first.segments[0].samples.last().unwrap().point, point(10.0, 0.0));
        assert!(first.segments[0].samples.windows(2).all(|pair| pair[0].t < pair[1].t));
    }

    #[test]
    fn cubic_sampling_preserves_original_command_and_endpoint_parameters() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::CubicTo {
                control1: point(0.0, 10.0),
                control2: point(10.0, 10.0),
                to: point(10.0, 0.0),
            },
        ]).unwrap();
        let sampled = sample_path_curves(&path, policy(0.1)).unwrap();
        let segment = &sampled.segments[0];
        assert_eq!(segment.command_index, 1);
        assert_eq!(segment.subpath_index, 0);
        assert_eq!(segment.samples.first().unwrap().t, 0.0);
        assert_eq!(segment.samples.last().unwrap().t, 1.0);
        assert!(segment.samples.len() > 2);
    }

    #[test]
    fn lines_and_close_are_not_emitted_as_curves() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::LineTo(point(1.0, 0.0)),
            PathCommand::QuadraticTo { control: point(2.0, 1.0), to: point(3.0, 0.0) },
            PathCommand::Close,
        ]).unwrap();
        let sampled = sample_path_curves(&path, policy(0.1)).unwrap();
        assert_eq!(sampled.segments.len(), 1);
        assert_eq!(sampled.segments[0].command_index, 2);
    }

    #[test]
    fn multiple_subpaths_keep_their_original_ordinals() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::QuadraticTo { control: point(1.0, 1.0), to: point(2.0, 0.0) },
            PathCommand::Close,
            PathCommand::MoveTo(point(3.0, 0.0)),
            PathCommand::CubicTo {
                control1: point(4.0, 1.0), control2: point(5.0, 1.0), to: point(6.0, 0.0),
            },
        ]).unwrap();
        let sampled = sample_path_curves(&path, policy(0.1)).unwrap();
        assert_eq!(sampled.segments[0].subpath_index, 0);
        assert_eq!(sampled.segments[1].subpath_index, 1);
    }

    #[test]
    fn invalid_policy_and_exhausted_budgets_fail_closed() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::CubicTo {
                control1: point(0.0, 10.0), control2: point(10.0, 10.0), to: point(10.0, 0.0),
            },
        ]).unwrap();
        assert_eq!(sample_path_curves(&path, policy(0.0)), Err(CurveSamplingError::InvalidPolicy));

        let mut depth_limited = policy(0.000_001);
        depth_limited.max_subdivision_depth = 0;
        assert_eq!(
            sample_path_curves(&path, depth_limited),
            Err(CurveSamplingError::SubdivisionDepthExceeded { command_index: 1 }),
        );

        let mut sample_limited = policy(0.000_001);
        sample_limited.max_samples_per_curve = 2;
        assert_eq!(
            sample_path_curves(&path, sample_limited),
            Err(CurveSamplingError::PerCurveSampleBudgetExceeded { command_index: 1 }),
        );

        let two_curves = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::QuadraticTo { control: point(0.5, 0.0), to: point(1.0, 0.0) },
            PathCommand::QuadraticTo { control: point(2.0, 1.0), to: point(3.0, 0.0) },
        ]).unwrap();
        let mut total_limited = policy(0.000_001);
        total_limited.max_samples_per_curve = 3;
        total_limited.max_total_samples = 3;
        assert_eq!(
            sample_path_curves(&two_curves, total_limited),
            Err(CurveSamplingError::TotalSampleBudgetExceeded),
        );
    }

    #[test]
    fn zero_length_chord_is_handled_without_nan() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(1.0, 1.0)),
            PathCommand::QuadraticTo { control: point(2.0, 2.0), to: point(1.0, 1.0) },
        ]).unwrap();
        let sampled = sample_path_curves(&path, policy(0.1)).unwrap();
        assert!(sampled.segments[0].samples.len() > 2);
        assert!(sampled.segments[0].samples.iter().all(|sample| sample.t.is_finite()));
    }

    #[test]
    fn adaptive_samples_use_the_shared_point_locator_and_outside_policy() {
        use crate::embedding::validate_cage_embedding;
        use crate::topology::{
            ControlPointId, ControlPointPosition, ControlPointState, ControlStateRevision,
            DeformationControlState, DeformationTopology, DeformationTopologyId,
            DeformationTopologyRevision, IrregularCageTopology, TopologyFace,
        };

        let ids = [1, 2, 3].map(|value| ControlPointId::new(value).unwrap());
        let topology = DeformationTopology::IrregularCage(IrregularCageTopology {
            id: DeformationTopologyId::new(7).unwrap(),
            revision: DeformationTopologyRevision::new(1).unwrap(),
            control_point_ids: ids.to_vec(),
            faces: vec![TopologyFace::Triangle(ids)],
        });
        let state = DeformationControlState {
            topology_id: topology.topology_id(),
            topology_revision: topology.topology_revision(),
            revision: ControlStateRevision::new(1).unwrap(),
            points: vec![
                (ids[0], 0.0, 0.0),
                (ids[1], 2.0, 0.0),
                (ids[2], 0.0, 2.0),
            ].into_iter().map(|(id, x, y)| ControlPointState {
                id,
                position: ControlPointPosition::new(x, y).unwrap(),
                pinned: false,
            }).collect(),
        };
        let certificate = validate_cage_embedding(&topology, &state).unwrap();
        let geometry = crate::binding::ValidatedCageGeometry::new(
            &topology, &state, &certificate,
        ).unwrap();

        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(-0.5, 0.5)),
            PathCommand::QuadraticTo {
                control: point(-0.25, 0.6),
                to: point(-0.5, 0.7),
            },
        ]).unwrap();
        let samples = sample_path_curves(&path, policy(0.01)).unwrap();
        let mapped = map_curve_samples(
            &samples, &geometry, crate::binding::OutsideCagePolicy::ClampToBoundary,
        ).unwrap();

        assert_eq!(mapped.samples.len(), samples.total_samples);
        assert!(mapped.samples.iter().all(|sample| matches!(
            sample.mapping, CurveSampleMapping::Clamped { face_index: 0, .. }
        )));
        assert!(mapped.samples.iter().all(|sample| sample.command_index == 1));
    }

    #[test]
    fn sampling_does_not_mutate_or_replace_the_source_path() {
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(0.0, 0.0)),
            PathCommand::CubicTo {
                control1: point(0.0, 1.0), control2: point(1.0, 1.0), to: point(1.0, 0.0),
            },
        ]).unwrap();
        let original = path.clone();
        let _ = sample_path_curves(&path, policy(0.05)).unwrap();
        assert_eq!(path, original);
        assert!(matches!(path.commands()[1], PathCommand::CubicTo { .. }));
    }
    fn cubic_point(p0: Point2, p1: Point2, p2: Point2, p3: Point2, t: f64) -> Point2 {
        let one_minus_t = 1.0 - t;
        let a = one_minus_t * one_minus_t * one_minus_t;
        let b = 3.0 * one_minus_t * one_minus_t * t;
        let c = 3.0 * one_minus_t * t * t;
        let d = t * t * t;
        point(
            a * p0.x() + b * p1.x() + c * p2.x() + d * p3.x(),
            a * p0.y() + b * p1.y() + c * p2.y() + d * p3.y(),
        )
    }

    #[test]
    fn high_curvature_cubic_respects_tolerance_and_tightening_is_monotonic() {
        let p0 = point(0.0, 0.0);
        let p1 = point(0.0, 100.0);
        let p2 = point(100.0, -100.0);
        let p3 = point(100.0, 0.0);
        let path = Path2D::new(vec![
            PathCommand::MoveTo(p0),
            PathCommand::CubicTo { control1: p1, control2: p2, to: p3 },
        ]).unwrap();

        let coarse = sample_path_curves(&path, policy(1.0)).unwrap();
        let fine = sample_path_curves(&path, policy(0.1)).unwrap();
        assert!(fine.total_samples >= coarse.total_samples);
        assert!(fine.total_samples > 2);

        let samples = &fine.segments[0].samples;
        assert_eq!(samples.first().unwrap().t, 0.0);
        assert_eq!(samples.last().unwrap().t, 1.0);
        assert!(samples.windows(2).all(|pair| {
            pair[0].t.is_finite()
                && pair[0].t < pair[1].t
                && pair[1].t.is_finite()
        }));

        for pair in samples.windows(2) {
            let midpoint_t = (pair[0].t + pair[1].t) * 0.5;
            let curve_midpoint = cubic_point(p0, p1, p2, p3, midpoint_t);
            let chord_deviation = point_segment_distance(
                curve_midpoint, pair[0].point, pair[1].point,
            ).unwrap();
            assert!(
                chord_deviation <= 0.1,
                "deviation {chord_deviation} exceeded tolerance for [{}, {}]",
                pair[0].t,
                pair[1].t,
            );
        }
    }

    #[test]
    fn closed_loop_cubic_and_large_coordinates_remain_finite() {
        let scale = 1.0e100;
        let path = Path2D::new(vec![
            PathCommand::MoveTo(point(scale, scale)),
            PathCommand::CubicTo {
                control1: point(scale, -scale),
                control2: point(-scale, -scale),
                to: point(-scale, scale),
            },
            PathCommand::CubicTo {
                control1: point(-scale, 3.0 * scale),
                control2: point(scale, 3.0 * scale),
                to: point(scale, scale),
            },
        ]).unwrap();

        let sampled = sample_path_curves(&path, policy(scale * 0.01)).unwrap();
        assert_eq!(sampled.segments.len(), 2);
        assert!(sampled.segments.iter().all(|segment| {
            segment.samples.len() >= 2
                && segment.samples.iter().all(|sample| {
                    sample.t.is_finite()
                        && sample.point.x().is_finite()
                        && sample.point.y().is_finite()
                })
        }));
    }

}
