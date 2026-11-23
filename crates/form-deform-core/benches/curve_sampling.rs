use std::hint::black_box;
use std::time::{Duration, Instant};

use form_deform_core::curve_sampling::{sample_path_curves, CurveSamplingPolicy};
use form_deform_core::path2d::{Path2D, PathCommand, Point2};

fn point(x: f64, y: f64) -> Point2 {
    Point2::new(x, y).expect("benchmark coordinates are finite")
}

fn workload(curve_count: usize, amplitude: f64) -> Path2D {
    let mut commands = Vec::with_capacity(curve_count + 1);
    commands.push(PathCommand::MoveTo(point(0.0, 0.0)));
    let mut x = 0.0;
    for index in 0..curve_count {
        let next_x = x + 10.0;
        let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
        commands.push(PathCommand::CubicTo {
            control1: point(x + 2.5, sign * amplitude),
            control2: point(x + 7.5, -sign * amplitude),
            to: point(next_x, 0.0),
        });
        x = next_x;
    }
    Path2D::new(commands).expect("benchmark path is valid")
}

fn measure(label: &str, path: &Path2D, policy: CurveSamplingPolicy, iterations: usize) {
    let warmup = sample_path_curves(path, policy).expect("warmup sampling succeeds");
    let expected_samples = warmup.total_samples;
    let start = Instant::now();
    let mut elapsed = Duration::ZERO;

    for _ in 0..iterations {
        let iteration_start = Instant::now();
        let result = sample_path_curves(black_box(path), black_box(policy))
            .expect("benchmark sampling succeeds");
        elapsed += iteration_start.elapsed();
        assert_eq!(result.total_samples, expected_samples, "sample count must be deterministic");
        black_box(result);
    }

    let nanos_per_iteration = elapsed.as_nanos() / iterations as u128;
    println!(
        "{label}: {iterations} iterations, {expected_samples} samples/run,          {nanos_per_iteration} ns/run, {} samples/s",
        if nanos_per_iteration == 0 { 0 } else {
            (expected_samples as u128 * 1_000_000_000 / nanos_per_iteration) as u64
        }
    );
    black_box(start.elapsed());
}

fn main() {
    let iterations = std::env::args()
        .skip_while(|argument| argument != "--iterations")
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(200);
    assert!(iterations > 0, "iterations must be greater than zero");

    let policy = CurveSamplingPolicy {
        max_deviation: 0.25,
        max_subdivision_depth: 16,
        max_samples_per_curve: 65_537,
        max_total_samples: 1_000_000,
    };
    let moderate = workload(128, 8.0);
    let high_curvature = workload(128, 80.0);

    measure("moderate-curvature", &moderate, policy, iterations);
    measure("high-curvature", &high_curvature, policy, iterations);
}
