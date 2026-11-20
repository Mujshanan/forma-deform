#![no_main]
use form_deform_core::{
    deform_mesh_with_diagnostics, BonePose, BoneTransform, BoneWeight, DeformDiagnosticsOptions,
    DeformMesh, MeshPin, MeshVertex, Vec2,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Bound decoding work independently of the fuzzer's input-size setting.
    if data.len() > 64 * 1024 { return; }
    let mut cursor = 0usize;
    let take = |cursor: &mut usize| -> Option<u8> {
        let value = *data.get(*cursor)?;
        *cursor += 1;
        Some(value)
    };
    let Some(vertex_count) = take(&mut cursor).map(|v| (v as usize).min(64)) else { return; };
    let Some(bone_count) = take(&mut cursor).map(|v| (v as usize).min(16)) else { return; };
    let mut bones = Vec::new();
    for id in 0..bone_count {
        let Some(x) = data.get(cursor..cursor + 8) else { return; };
        cursor += 8;
        let tx = f64::from_le_bytes(x.try_into().unwrap());
        bones.push(BonePose { id: id as u64, world: BoneTransform {
            translation: Vec2 { x: tx, y: 0.0 }, rotation_degrees: 0.0,
        }});
    }
    let mut vertices = Vec::new();
    for _ in 0..vertex_count {
        let Some(bytes) = data.get(cursor..cursor + 16) else { return; };
        cursor += 16;
        let x = f64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let y = f64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let Some(influence_byte) = take(&mut cursor) else { return; };
        let influences = if bone_count == 0 { vec![] } else {
            vec![BoneWeight { bone_id: (influence_byte as usize % bone_count) as u64, weight: 1.0 }]
        };
        vertices.push(MeshVertex { bind_position: Vec2 { x, y }, influences });
    }
    let pins = if vertices.is_empty() {
        vec![]
    } else {
        vec![MeshPin { id: 1, vertex_index: 0, target: Vec2::default() }]
    };
    let mesh = DeformMesh { vertices, pins };
    let _ = deform_mesh_with_diagnostics(&mesh, &bones, DeformDiagnosticsOptions::default());
});
