//! Reproducible, dependency-free microbenchmarks for adaptive Path2D sampling.
//!
//! Run with:
//! cargo bench -p form-deform-core --bench curve_sampling

use std::hint::black_box;
use std::time::{Duration, Instant};

use form_deform_core::curve_sampling::{sample_path_curves, CurveSamplingPolicy};
use form_deform_core::path2d::{Path2D, PathCommand, Point2};

struct Workload {
    name: &'static str,
    path: Path2D,
    policy: CurveSamplingPolicy,
}

fn point(x: f64, y: f64) -> Point2 {
    Point2::new(x, y).expect("benchmark coordinates must remain finite")
}

fn workload(
    name: &'static str,
    curve_count: usize,
    scale: f64,
    curvature: f64,
) -> Workload {
    let mut commands = Vec::with_capacity(curve_count + 1);
    commands.push(PathCommand::MoveTo(point(0.0, 0.0)));

    for index in 0..curve_count {
        let x = index as f64 * scale;
        commands.push(PathCommand::CubicTo {
            control1: point(x + scale / 3.0, curvature * scale),
            control2: point(x + 2.0 * scale / 3.0, -curvature * scale),
            to: point(x + scale, 0.0),
        });
    }

    Workload {
        name,
        path: Path2D::new(commands).expect("generated path must be valid"),
        policy: CurveSamplingPolicy {
            max_deviation: scale * 0.01,
            max_subdivision_depth: 20,
            max_samples_per_curve: 65_537,
            max_total_samples: 1_000_000,
        },
    }
}

fn report(workload: &Workload) {
    const WARMUP_ITERATIONS: usize = 20;
    const MEASURED_ITERATIONS: usize = 200;

    let reference = sample_path_curves(&workload.path, workload.policy)
        .expect("reference sample must succeed");
    assert!(reference.total_samples > 0);

    for _ in 0..WARMUP_ITERATIONS {
        let output = sample_path_curves(black_box(&workload.path), workload.policy)
            .expect("warmup sample must succeed");
        assert_eq!(output, reference, "sampling output must be deterministic");
    }

    let start = Instant::now();
    for _ in 0..MEASURED_ITERATIONS {
        let output = sample_path_curves(black_box(&workload.path), workload.policy)
            .expect("measured sample must succeed");
        assert_eq!(output, reference, "sampling output must be deterministic");
        black_box(output);
    }
    let elapsed = start.elapsed();
    print_result(workload.name, elapsed, reference.total_samples, MEASURED_ITERATIONS);
}

fn print_result(name: &str, elapsed: Duration, samples: usize, iterations: usize) {
    let nanos_per_iteration = elapsed.as_nanos() as f64 / iterations as f64;
    let nanos_per_sample = nanos_per_iteration / samples as f64;
    println!(
        "{name}: {samples} samples/iteration; {nanos_per_iteration:.0} ns/iteration; \
         {nanos_per_sample:.2} ns/sample; {iterations} measured iterations"
    );
}

fn main() {
    let workloads = [
        workload("single-high-curvature", 1, 100.0, 1.0),
        workload("many-high-curvature", 32, 100.0, 1.0),
        workload("many-moderate-curves", 128, 10.0, 0.2),
        workload("large-coordinate-curves", 32, 1.0e80, 1.0),
    ];

    println!("Adaptive Path2D sampling benchmark (release profile)");
    for item in &workloads {
        report(item);
    }
}
