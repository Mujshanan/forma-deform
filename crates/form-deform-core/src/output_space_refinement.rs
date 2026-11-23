//! Deterministic output-space adaptive refinement for Path2D Bézier commands.
//!
//! The reported deviation is a deterministic three-probe estimate (t = 1/4,
//! 1/2, 3/4 of each interval), not a certified maximum-error bound. A cage
//! mapping is piecewise/nonlinear, so source-space flatness is not sufficient.

use crate::binding::{OutsideCagePolicy, ValidatedCageGeometry};
use crate::binding_persistence::ValidatedBindingRecord;
use crate::curve_sample_evaluation::{
    evaluate_source_point, CurveSampleEvaluationError,
};
use crate::host_contract::GeometrySnapshot;
use crate::path2d::{PathCommand, Path2D, Point2};
use crate::topology::{DeformationTopology, TopologyView};

pub const OUTPUT_REFINEMENT_POLICY_VERSION: u32 = 1;
const MAX_DEPTH: u8 = 24;
const MAX_TOTAL_SAMPLES: usize = 1_000_000;
const MAX_TOTAL_EVALUATIONS: usize = 4_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutputRefinementPolicy {
    /// Maximum accepted three-probe deviation estimate in output coordinates.
    pub max_deviation: f64,
    pub max_subdivision_depth: u8,
    pub max_samples_per_curve: usize,
    pub max_total_samples: usize,
    /// Includes interval endpoints and all three probes at every visited node.
    pub max_total_evaluations: usize,
}

impl OutputRefinementPolicy {
    pub fn validate(self) -> Result<(), OutputRefinementError> {
        if !self.max_deviation.is_finite() || self.max_deviation <= 0.0
            || self.max_subdivision_depth > MAX_DEPTH
            || self.max_samples_per_curve < 2
            || self.max_total_samples < 2
            || self.max_total_samples > MAX_TOTAL_SAMPLES
            || self.max_samples_per_curve > self.max_total_samples
            || self.max_total_evaluations < 5
            || self.max_total_evaluations > MAX_TOTAL_EVALUATIONS
        {
            return Err(OutputRefinementError::InvalidPolicy);
        }
        Ok(())
    }
}

impl Default for OutputRefinementPolicy {
    fn default() -> Self {
        Self {
            max_deviation: 0.25,
            max_subdivision_depth: 16,
            max_samples_per_curve: 65_537,
            max_total_samples: MAX_TOTAL_SAMPLES,
            max_total_evaluations: MAX_TOTAL_EVALUATIONS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefinedCurveSample {
    pub t: f64,
    pub source_point: Point2,
    pub output_point: Point2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefinedCurveSegment {
    pub command_index: u32,
    pub subpath_index: u32,
    pub samples: Vec<RefinedCurveSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefinedCurveSampleSet {
    pub policy_version: u32,
    pub policy: OutputRefinementPolicy,
    pub segments: Vec<RefinedCurveSegment>,
    pub total_samples: usize,
    pub total_evaluations: usize,
    /// Maximum observed probe deviation among accepted intervals.
    /// This is an estimate, not a certified global error bound.
    pub max_observed_deviation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutputRefinementError {
    InvalidPolicy,
    InvalidSourceSnapshot,
    StaleCageTopology,
    StaleRestCage,
    InvalidPathSequence { command_index: usize },
    SubdivisionDepthExceeded { command_index: u32, t0: f64, t1: f64 },
    PerCurveSampleBudgetExceeded { command_index: u32 },
    TotalSampleBudgetExceeded,
    EvaluationBudgetExceeded,
    Evaluation(CurveSampleEvaluationError),
    ArithmeticOverflow,
}

impl std::fmt::Display for OutputRefinementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "output-space curve refinement failed: {self:?}")
    }
}
impl std::error::Error for OutputRefinementError {}

/// Refine each original quadratic/cubic command against its evaluated output.
/// Identical inputs and policy use a fixed left-to-right subdivision order.
pub fn refine_path_curves(
    validated: &ValidatedBindingRecord,
    source: &GeometrySnapshot,
    topology: &DeformationTopology,
    rest_cage: &ValidatedCageGeometry,
    current_cage: &ValidatedCageGeometry,
    outside_policy: OutsideCagePolicy,
    policy: OutputRefinementPolicy,
) -> Result<RefinedCurveSampleSet, OutputRefinementError> {
    policy.validate()?;
    source.validate_envelope().map_err(|_| OutputRefinementError::InvalidSourceSnapshot)?;
    validated.binding().stamp().validate_freshness(source, topology)
        .map_err(|_| OutputRefinementError::InvalidSourceSnapshot)?;
    if rest_cage.topology_id() != topology.topology_id()
        || rest_cage.topology_revision() != topology.topology_revision()
        || current_cage.topology_id() != topology.topology_id()
        || current_cage.topology_revision() != topology.topology_revision()
    {
        return Err(OutputRefinementError::StaleCageTopology);
    }
    if validated.rest_cage_fingerprint() != rest_cage.geometry_fingerprint() {
        return Err(OutputRefinementError::StaleRestCage);
    }
    if outside_policy != validated.binding().stamp().outside_policy {
        return Err(OutputRefinementError::InvalidSourceSnapshot);
    }

    let path = source.payload();
    if path.commands().len() > u32::MAX as usize {
        return Err(OutputRefinementError::InvalidPathSequence { command_index: path.commands().len() });
    }

    let mut result = Vec::new();
    let mut current = None;
    let mut subpath_index = 0u32;
    let mut has_seen_move = false;
    let mut total_samples = 0usize;
    let mut total_evaluations = 0usize;
    let mut max_observed_deviation: f64 = 0.0;

    for (index, command) in path.commands().iter().enumerate() {
        match command {
            PathCommand::MoveTo(point) => {
                if has_seen_move {
                    subpath_index = subpath_index.checked_add(1).ok_or(OutputRefinementError::ArithmeticOverflow)?;
                }
                has_seen_move = true;
                current = Some(*point);
            }
            PathCommand::LineTo(point) => {
                if current.is_none() { return Err(OutputRefinementError::InvalidPathSequence { command_index: index }); }
                current = Some(*point);
            }
            PathCommand::QuadraticTo { to, .. } => {
                let from = current.ok_or(OutputRefinementError::InvalidPathSequence { command_index: index })?;
                let command_index = index as u32;
                let mut samples = Vec::new();
                let first = evaluate_at(validated, source, topology, rest_cage, current_cage, outside_policy,
                    command, from, 0.0, &mut total_evaluations, policy)?;
                push_sample(&mut samples, RefinedCurveSample { t: 0.0, source_point: from, output_point: first },
                    command_index, &mut total_samples, policy)?;
                let last = evaluate_at(validated, source, topology, rest_cage, current_cage, outside_policy,
                    command, from, 1.0, &mut total_evaluations, policy)?;
                refine_interval(validated, source, topology, rest_cage, current_cage, outside_policy,
                    command, from, 0.0, 1.0, first, last,
                    0, command_index, &mut samples, &mut total_samples, &mut total_evaluations,
                    &mut max_observed_deviation, policy)?;
                result.push(RefinedCurveSegment { command_index, subpath_index, samples });
                current = Some(*to);
            }
            PathCommand::CubicTo { to, .. } => {
                let from = current.ok_or(OutputRefinementError::InvalidPathSequence { command_index: index })?;
                let command_index = index as u32;
                let mut samples = Vec::new();
                let first = evaluate_at(validated, source, topology, rest_cage, current_cage, outside_policy,
                    command, from, 0.0, &mut total_evaluations, policy)?;
                push_sample(&mut samples, RefinedCurveSample { t: 0.0, source_point: from, output_point: first },
                    command_index, &mut total_samples, policy)?;
                refine_interval(validated, source, topology, rest_cage, current_cage, outside_policy,
                    command, from, 0.0, 1.0, first, point_at(command, from, 1.0)?,
                    0, command_index, &mut samples, &mut total_samples, &mut total_evaluations,
                    &mut max_observed_deviation, policy)?;
                result.push(RefinedCurveSegment { command_index, subpath_index, samples });
                current = Some(*to);
            }
            PathCommand::Close => {
                if current.is_none() { return Err(OutputRefinementError::InvalidPathSequence { command_index: index }); }
                current = None;
            }
        }
    }

    Ok(RefinedCurveSampleSet {
        policy_version: OUTPUT_REFINEMENT_POLICY_VERSION,
        policy,
        segments: result,
        total_samples,
        total_evaluations,
        max_observed_deviation,
    })
}

fn evaluate_at(
    validated: &ValidatedBindingRecord, source: &GeometrySnapshot, topology: &DeformationTopology,
    rest: &ValidatedCageGeometry, current: &ValidatedCageGeometry, outside: OutsideCagePolicy,
    command: &PathCommand, from: Point2, t: f64, evaluations: &mut usize,
    policy: OutputRefinementPolicy,
) -> Result<Point2, OutputRefinementError> {
    *evaluations = evaluations.checked_add(1).ok_or(OutputRefinementError::ArithmeticOverflow)?;
    if *evaluations > policy.max_total_evaluations { return Err(OutputRefinementError::EvaluationBudgetExceeded); }
    let source_point = point_at(command, from, t)?;
    evaluate_source_point(validated, source, topology, rest, current, outside, source_point)
        .map_err(OutputRefinementError::Evaluation)
}

fn refine_interval(
    validated: &ValidatedBindingRecord, source: &GeometrySnapshot, topology: &DeformationTopology,
    rest: &ValidatedCageGeometry, current: &ValidatedCageGeometry, outside: OutsideCagePolicy,
    command: &PathCommand, from: Point2, t0: f64, t1: f64, out0: Point2, out1: Point2,
    depth: u8, command_index: u32, samples: &mut Vec<RefinedCurveSample>,
    total_samples: &mut usize, evaluations: &mut usize, max_observed: &mut f64,
    policy: OutputRefinementPolicy,
) -> Result<(), OutputRefinementError> {
    let q1 = t0 + (t1 - t0) * 0.25;
    let mid = t0 + (t1 - t0) * 0.5;
    let q3 = t0 + (t1 - t0) * 0.75;
    let mut probes = Vec::with_capacity(3);
    for t in [q1, mid, q3] {
        let output = evaluate_at(validated, source, topology, rest, current, outside,
            command, from, t, evaluations, policy)?;
        let source_point = point_at(command, from, t)?;
        probes.push(RefinedCurveSample { t, source_point, output_point: output });
    }
    let deviation = probes.iter().map(|probe| point_segment_distance(probe.output_point, out0, out1))
        .collect::<Result<Vec<_>, _>>()?.into_iter().fold(0.0, f64::max);
    if deviation <= policy.max_deviation {
        *max_observed = max_observed.max(deviation);
        let source_end = point_at(command, from, t1)?;
        push_sample(samples, RefinedCurveSample { t: t1, source_point: source_end, output_point: out1 },
            command_index, total_samples, policy)?;
        return Ok(());
    }
    if depth >= policy.max_subdivision_depth {
        return Err(OutputRefinementError::SubdivisionDepthExceeded { command_index, t0, t1 });
    }
    let middle = probes[1];
    refine_interval(validated, source, topology, rest, current, outside, command, from,
        t0, mid, out0, middle.output_point, depth + 1, command_index, samples,
        total_samples, evaluations, max_observed, policy)?;
    refine_interval(validated, source, topology, rest, current, outside, command, from,
        mid, t1, middle.output_point, out1, depth + 1, command_index, samples,
        total_samples, evaluations, max_observed, policy)
}

fn push_sample(
    samples: &mut Vec<RefinedCurveSample>, sample: RefinedCurveSample, command_index: u32,
    total: &mut usize, policy: OutputRefinementPolicy,
) -> Result<(), OutputRefinementError> {
    if samples.len() >= policy.max_samples_per_curve {
        return Err(OutputRefinementError::PerCurveSampleBudgetExceeded { command_index });
    }
    if *total >= policy.max_total_samples { return Err(OutputRefinementError::TotalSampleBudgetExceeded); }
    if !sample.t.is_finite() || !(0.0..=1.0).contains(&sample.t)
        || !sample.source_point.x().is_finite() || !sample.source_point.y().is_finite()
        || !sample.output_point.x().is_finite() || !sample.output_point.y().is_finite()
    { return Err(OutputRefinementError::ArithmeticOverflow); }
    samples.push(sample);
    *total += 1;
    Ok(())
}

fn point_at(command: &PathCommand, from: Point2, t: f64) -> Result<Point2, OutputRefinementError> {
    let s = 1.0 - t;
    let (x, y) = match command {
        PathCommand::QuadraticTo { control, to } => (
            s*s*from.x() + 2.0*s*t*control.x() + t*t*to.x(),
            s*s*from.y() + 2.0*s*t*control.y() + t*t*to.y(),
        ),
        PathCommand::CubicTo { control1, control2, to } => (
            s*s*s*from.x() + 3.0*s*s*t*control1.x() + 3.0*s*t*t*control2.x() + t*t*t*to.x(),
            s*s*s*from.y() + 3.0*s*s*t*control1.y() + 3.0*s*t*t*control2.y() + t*t*t*to.y(),
        ),
        _ => return Err(OutputRefinementError::InvalidPathSequence { command_index: 0 }),
    };
    Point2::new(x, y).map_err(|_| OutputRefinementError::ArithmeticOverflow)
}

fn point_segment_distance(point: Point2, start: Point2, end: Point2) -> Result<f64, OutputRefinementError> {
    let scale = point.x().abs().max(point.y().abs()).max(start.x().abs())
        .max(start.y().abs()).max(end.x().abs()).max(end.y().abs());
    if scale == 0.0 { return Ok(0.0); }
    let (px,py,ax,ay,bx,by) = (point.x()/scale,point.y()/scale,start.x()/scale,start.y()/scale,end.x()/scale,end.y()/scale);
    let (dx,dy) = (bx-ax,by-ay);
    let length = dx*dx + dy*dy;
    let t = if length == 0.0 { 0.0 } else { (((px-ax)*dx+(py-ay)*dy)/length).clamp(0.0,1.0) };
    let distance = ((px-(ax+t*dx)).hypot(py-(ay+t*dy)))*scale;
    if distance.is_finite() { Ok(distance) } else { Err(OutputRefinementError::ArithmeticOverflow) }
}
