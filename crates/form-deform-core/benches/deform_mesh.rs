use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use form_deform_core::{deform_mesh, BonePose, BoneTransform, BoneWeight, DeformMesh, MeshVertex, Vec2};

fn mesh(vertex_count: usize, bone_count: usize) -> (DeformMesh, Vec<BonePose>) {
    let bones: Vec<_> = (0..bone_count)
        .map(|id| BonePose {
            id: id as u64,
            world: BoneTransform {
                translation: Vec2 { x: id as f64 * 0.1, y: -(id as f64) * 0.05 },
                rotation_degrees: id as f64 * 0.25,
            },
        })
        .collect();
    let vertices = (0..vertex_count)
        .map(|i| MeshVertex {
            bind_position: Vec2 { x: i as f64 * 0.01, y: (i % 97) as f64 * 0.02 },
            influences: vec![
                BoneWeight { bone_id: 0, weight: 0.5 },
                BoneWeight { bone_id: (bone_count - 1) as u64, weight: 0.5 },
            ],
        })
        .collect();
    (DeformMesh { vertices, pins: vec![] }, bones)
}

fn bench_mesh_deformation(c: &mut Criterion) {
    let mut group = c.benchmark_group("deform_mesh_lbs");
    for count in [100usize, 1_000, 10_000, 100_000] {
        let (mesh, bones) = mesh(count, 64);
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| deform_mesh(black_box(&mesh), black_box(&bones)).unwrap())
        });
    }
    group.finish();
}

criterion_group!(benches, bench_mesh_deformation);
criterion_main!(benches);
