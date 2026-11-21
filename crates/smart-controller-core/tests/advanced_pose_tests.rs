use smart_controller_core::{
    mapping::advanced::{
        grid::{GridBoundaryPolicy, GridMapping},
        triangulated::{MeshBoundaryPolicy, Triangle, TriangulatedMapping, TriangulatedPose},
    },
    BindingId, Pose2D, TypedValue, Vector2,
};

fn pose(value: f64) -> Pose2D {
    Pose2D::new(vec![(
        BindingId::new("position").unwrap(),
        TypedValue::scalar(value).unwrap(),
    )]).unwrap()
}
fn scalar(pose: &Pose2D) -> f64 {
    pose.get(&BindingId::new("position").unwrap()).unwrap().as_scalar().unwrap()
}
fn point(x: f64, y: f64) -> Vector2 { Vector2::new(x, y).unwrap() }

fn triangle_mesh(policy: MeshBoundaryPolicy) -> TriangulatedMapping {
    TriangulatedMapping::new(
        vec![
            TriangulatedPose::new(point(0.0, 0.0), pose(0.0)),
            TriangulatedPose::new(point(2.0, 0.0), pose(10.0)),
            TriangulatedPose::new(point(0.0, 2.0), pose(20.0)),
        ],
        vec![Triangle::new(0, 1, 2)],
        policy,
    ).unwrap()
}

#[test]
fn triangulated_mapping_interpolates_non_uniform_triangle_barycentrically() {
    let result = triangle_mesh(MeshBoundaryPolicy::Reject).evaluate(point(0.5, 0.5)).unwrap();
    assert!((scalar(&result) - 7.5).abs() < 1.0e-9);
}

#[test]
fn triangulated_mapping_returns_exact_authored_vertex_pose() {
    let result = triangle_mesh(MeshBoundaryPolicy::Reject).evaluate(point(2.0, 0.0)).unwrap();
    assert_eq!(scalar(&result), 10.0);
}

#[test]
fn triangulated_mapping_rejects_outside_points_when_configured() {
    assert!(triangle_mesh(MeshBoundaryPolicy::Reject).evaluate(point(3.0, 3.0)).is_err());
}

#[test]
fn triangulated_mapping_clamps_to_nearest_mesh_edge() {
    let result = triangle_mesh(MeshBoundaryPolicy::ClampToMesh).evaluate(point(2.0, 2.0)).unwrap();
    assert!((scalar(&result) - 15.0).abs() < 1.0e-9);
}

#[test]
fn triangulated_mapping_rejects_degenerate_triangles() {
    let result = TriangulatedMapping::new(
        vec![
            TriangulatedPose::new(point(0.0, 0.0), pose(0.0)),
            TriangulatedPose::new(point(1.0, 1.0), pose(1.0)),
            TriangulatedPose::new(point(2.0, 2.0), pose(2.0)),
        ],
        vec![Triangle::new(0, 1, 2)],
        MeshBoundaryPolicy::Reject,
    );
    assert!(result.is_err());
}

#[test]
fn triangulated_mapping_rejects_incompatible_pose_schemas() {
    let incompatible = Pose2D::new(vec![(
        BindingId::new("other").unwrap(),
        TypedValue::scalar(1.0).unwrap(),
    )]).unwrap();
    let result = TriangulatedMapping::new(
        vec![
            TriangulatedPose::new(point(0.0, 0.0), pose(0.0)),
            TriangulatedPose::new(point(1.0, 0.0), incompatible.clone()),
            TriangulatedPose::new(point(0.0, 1.0), pose(2.0)),
        ],
        vec![Triangle::new(0, 1, 2)],
        MeshBoundaryPolicy::Reject,
    );
    assert!(result.is_err());
}

#[test]
fn triangulated_mapping_rejects_ambiguous_overlapping_triangles() {
    let mapping = TriangulatedMapping::new(
        vec![
            TriangulatedPose::new(point(0.0, 0.0), pose(0.0)),
            TriangulatedPose::new(point(2.0, 0.0), pose(0.0)),
            TriangulatedPose::new(point(0.0, 2.0), pose(0.0)),
            TriangulatedPose::new(point(2.0, 2.0), pose(100.0)),
            TriangulatedPose::new(point(2.0, -1.0), pose(100.0)),
        ],
        vec![Triangle::new(0, 1, 2), Triangle::new(0, 3, 4)],
        MeshBoundaryPolicy::Reject,
    ).unwrap();
    assert!(mapping.evaluate(point(0.5, 0.0)).is_err());
}

fn grid(policy: GridBoundaryPolicy) -> GridMapping {
    GridMapping::new(
        vec![0.0, 2.0],
        vec![0.0, 4.0],
        vec![pose(0.0), pose(10.0), pose(20.0), pose(30.0)],
        policy,
    ).unwrap()
}

#[test]
fn grid_mapping_bilinearly_interpolates_non_uniform_axes() {
    let result = grid(GridBoundaryPolicy::Reject).evaluate(point(1.0, 2.0)).unwrap();
    assert!((scalar(&result) - 15.0).abs() < 1.0e-9);
}

#[test]
fn grid_mapping_preserves_exact_grid_corner() {
    let result = grid(GridBoundaryPolicy::Reject).evaluate(point(2.0, 4.0)).unwrap();
    assert_eq!(scalar(&result), 30.0);
}

#[test]
fn grid_mapping_rejects_outside_points_when_configured() {
    assert!(grid(GridBoundaryPolicy::Reject).evaluate(point(-0.1, 2.0)).is_err());
}

#[test]
fn grid_mapping_clamps_outside_points_to_grid_rectangle() {
    let result = grid(GridBoundaryPolicy::Clamp).evaluate(point(-10.0, 10.0)).unwrap();
    assert_eq!(scalar(&result), 20.0);
}

#[test]
fn grid_mapping_rejects_non_monotonic_axes_and_wrong_pose_count() {
    assert!(GridMapping::new(
        vec![0.0, 1.0, 1.0], vec![0.0, 1.0],
        vec![pose(0.0); 6], GridBoundaryPolicy::Reject,
    ).is_err());
    assert!(GridMapping::new(
        vec![0.0, 1.0], vec![0.0, 1.0],
        vec![pose(0.0); 3], GridBoundaryPolicy::Reject,
    ).is_err());
}

#[test]
fn grid_mapping_rejects_incompatible_pose_schemas() {
    let other = Pose2D::new(vec![(
        BindingId::new("different").unwrap(),
        TypedValue::scalar(1.0).unwrap(),
    )]).unwrap();
    assert!(GridMapping::new(
        vec![0.0, 1.0], vec![0.0, 1.0],
        vec![pose(0.0), pose(1.0), pose(2.0), other],
        GridBoundaryPolicy::Reject,
    ).is_err());
}

#[test]
fn grid_mapping_interpolates_wrapped_angles_using_shortest_path() {
    let angle_pose = |radians| Pose2D::new(vec![(
        BindingId::new("rotation").unwrap(),
        TypedValue::angle(radians, true).unwrap(),
    )]).unwrap();
    let mapping = GridMapping::new(
        vec![0.0, 1.0], vec![0.0, 1.0],
        vec![
            angle_pose(179.0_f64.to_radians()),
            angle_pose(-179.0_f64.to_radians()),
            angle_pose(179.0_f64.to_radians()),
            angle_pose(-179.0_f64.to_radians()),
        ],
        GridBoundaryPolicy::Reject,
    ).unwrap();
    let result = mapping.evaluate(point(0.5, 0.5)).unwrap();
    let angle = result.get(&BindingId::new("rotation").unwrap()).unwrap().as_angle().unwrap();
    assert!((angle.radians().abs() - std::f64::consts::PI).abs() < 1.0e-6);
}


#[test]
fn triangulated_mapping_is_continuous_across_a_shared_mesh_edge() {
    let mapping = TriangulatedMapping::new(
        vec![
            TriangulatedPose::new(point(0.0, 0.0), pose(0.0)),
            TriangulatedPose::new(point(1.0, 0.0), pose(10.0)),
            TriangulatedPose::new(point(1.0, 1.0), pose(20.0)),
            TriangulatedPose::new(point(0.0, 1.0), pose(30.0)),
        ],
        vec![Triangle::new(0, 1, 2), Triangle::new(0, 2, 3)],
        MeshBoundaryPolicy::Reject,
    ).unwrap();
    let result = mapping.evaluate(point(0.5, 0.5)).unwrap();
    assert!((scalar(&result) - 10.0).abs() < 1.0e-9);
}

#[test]
fn triangulated_mapping_rejects_duplicate_and_unreferenced_topology() {
    let vertices = vec![
        TriangulatedPose::new(point(0.0, 0.0), pose(0.0)),
        TriangulatedPose::new(point(1.0, 0.0), pose(1.0)),
        TriangulatedPose::new(point(0.0, 1.0), pose(2.0)),
        TriangulatedPose::new(point(2.0, 2.0), pose(3.0)),
    ];
    assert!(TriangulatedMapping::new(
        vertices.clone(),
        vec![Triangle::new(0, 1, 2), Triangle::new(2, 1, 0)],
        MeshBoundaryPolicy::Reject,
    ).is_err());
    assert!(TriangulatedMapping::new(
        vertices,
        vec![Triangle::new(0, 1, 2)],
        MeshBoundaryPolicy::Reject,
    ).is_err());
}
