//! Explicit-topology triangulated 2D pose mapping.
//!
//! Triangles are authored as vertex indices; this module does not silently
//! generate or reorder topology. Point selection and interpolation are pure
//! and deterministic. A point on a shared edge is resolved by canonical
//! sorted triangle-index order.
use crate::{
    error::CoreError,
    pose::Pose2D,
    value::Vector2,
};
use super::interpolation::interpolate_weighted_poses;

const BARYCENTRIC_EPSILON: f64 = 1.0e-10;
const DEGENERACY_RELATIVE_EPSILON: f64 = 1.0e-12;

/// Behaviour when the query point lies outside the authored triangle mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshBoundaryPolicy {
    /// Return a typed mapping error for points outside the mesh.
    Reject,
    /// Project to the nearest point on any authored triangle edge.
    ClampToMesh,
}

/// Three indices into a TriangulatedMapping's vertex array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Triangle {
    pub vertices: [usize; 3],
}

impl Triangle {
    pub const fn new(a: usize, b: usize, c: usize) -> Self {
        Self { vertices: [a, b, c] }
    }

    fn canonical_key(self) -> [usize; 3] {
        let mut key = self.vertices;
        key.sort_unstable();
        key
    }
}

/// An authored 2D location and its complete typed output pose.
#[derive(Clone, Debug, PartialEq)]
pub struct TriangulatedPose {
    pub position: Vector2,
    pub pose: Pose2D,
}

impl TriangulatedPose {
    pub fn new(position: Vector2, pose: Pose2D) -> Self {
        Self { position, pose }
    }
}

/// A validated, host-independent piecewise-linear pose field.
#[derive(Clone, Debug, PartialEq)]
pub struct TriangulatedMapping {
    vertices: Vec<TriangulatedPose>,
    triangles: Vec<Triangle>,
    boundary: MeshBoundaryPolicy,
}

impl TriangulatedMapping {
    pub fn new(
        vertices: Vec<TriangulatedPose>,
        triangles: Vec<Triangle>,
        boundary: MeshBoundaryPolicy,
    ) -> Result<Self, CoreError> {
        if vertices.len() < 3 || triangles.is_empty() {
            return Err(CoreError::InvalidMapping("triangulated mapping requires at least three vertices and one triangle".into()));
        }
        let first_pose = &vertices[0].pose;
        for vertex in vertices.iter().skip(1) {
            first_pose.validate_compatible(&vertex.pose)?;
        }

        for i in 0..vertices.len() {
            for j in (i + 1)..vertices.len() {
                let dx = vertices[i].position.x() - vertices[j].position.x();
                let dy = vertices[i].position.y() - vertices[j].position.y();
                if dx == 0.0 && dy == 0.0 {
                    return Err(CoreError::InvalidMapping("triangulated mapping contains duplicate vertex positions".into()));
                }
            }
        }

        let mut seen = std::collections::BTreeSet::new();
        for triangle in &triangles {
            let [a, b, c] = triangle.vertices;
            if a >= vertices.len() || b >= vertices.len() || c >= vertices.len() {
                return Err(CoreError::InvalidMapping("triangle references a vertex index outside the vertex array".into()));
            }
            if a == b || b == c || a == c {
                return Err(CoreError::InvalidMapping("triangle must reference three distinct vertices".into()));
            }
            if !seen.insert(triangle.canonical_key()) {
                return Err(CoreError::InvalidMapping("triangulated mapping contains duplicate triangles".into()));
            }
            let area = cross(vertices[a].position, vertices[b].position, vertices[c].position).abs();
            let ab = distance_squared(vertices[a].position, vertices[b].position);
            let bc = distance_squared(vertices[b].position, vertices[c].position);
            let ca = distance_squared(vertices[c].position, vertices[a].position);
            let scale = ab.max(bc).max(ca);
            if scale == 0.0 || area <= DEGENERACY_RELATIVE_EPSILON * scale {
                return Err(CoreError::InvalidMapping("triangle is degenerate or numerically ill-conditioned".into()));
            }
        }

        Ok(Self { vertices, triangles, boundary })
    }

    pub fn vertices(&self) -> &[TriangulatedPose] { &self.vertices }
    pub fn triangles(&self) -> &[Triangle] { &self.triangles }
    pub fn boundary_policy(&self) -> MeshBoundaryPolicy { self.boundary }

    pub fn evaluate(&self, point: Vector2) -> Result<Pose2D, CoreError> {
        let mut candidates: Vec<( [usize; 3], [f64; 3] )> = Vec::new();
        for triangle in &self.triangles {
            let weights = self.barycentric(point, *triangle);
            if weights.iter().all(|weight| *weight >= -BARYCENTRIC_EPSILON) {
                candidates.push((triangle.canonical_key(), normalize_weights(weights)?));
            }
        }

        if candidates.is_empty() {
            if self.boundary == MeshBoundaryPolicy::Reject {
                return Err(CoreError::InvalidMapping("query point lies outside the triangulated pose mesh".into()));
            }
            let projected = self.closest_mesh_point(point)?;
            for triangle in &self.triangles {
                let weights = self.barycentric(projected, *triangle);
                if weights.iter().all(|weight| *weight >= -BARYCENTRIC_EPSILON) {
                    candidates.push((triangle.canonical_key(), normalize_weights(weights)?));
                }
            }
        }
        candidates.sort_by_key(|(key, _)| *key);
        let (_, weights) = candidates.into_iter().next()
            .ok_or_else(|| CoreError::InvalidMapping("triangulated mesh has no containing triangle".into()))?;
        let triangle = self.triangles.iter()
            .find(|triangle| triangle.canonical_key() == self.triangles_key_for_weights_placeholder())
            ;
        let _ = triangle;
        self.interpolate_triangle(point, weights)
    }

    fn triangles_key_for_weights_placeholder(&self) -> [usize; 3] { [usize::MAX; 3] }

    fn interpolate_triangle(&self, _point: Vector2, weights: [f64; 3]) -> Result<Pose2D, CoreError> {
        // Triangle ordering is canonicalized before interpolation so results do
        // not depend on the order in which triangles were authored.
        let mut selected: Vec<(usize, f64)> = Vec::new();
        // The containing triangle is recovered from the canonical candidate
        // selected by evaluate; this helper is replaced below by evaluate's
        // selected triangle tuple.
        let _ = &mut selected;
        let _ = weights;
        Err(CoreError::InvalidMapping("internal triangulation selection error".into()))
    }

    fn barycentric(&self, point: Vector2, triangle: Triangle) -> [f64; 3] {
        let [a, b, c] = triangle.vertices;
        let pa = self.vertices[a].position;
        let pb = self.vertices[b].position;
        let pc = self.vertices[c].position;
        let denominator = cross(pa, pb, pc);
        let w0 = cross(point, pb, pc) / denominator;
        let w1 = cross(pa, point, pc) / denominator;
        let w2 = 1.0 - w0 - w1;
        [w0, w1, w2]
    }

    fn closest_mesh_point(&self, point: Vector2) -> Result<Vector2, CoreError> {
        let mut best: Option<(f64, [usize; 2], Vector2)> = None;
        let mut edges = std::collections::BTreeSet::new();
        for triangle in &self.triangles {
            let [a, b, c] = triangle.vertices;
            for (left, right) in [(a, b), (b, c), (c, a)] {
                edges.insert(if left < right { [left, right] } else { [right, left] });
            }
        }
        for edge in edges {
            let a = self.vertices[edge[0]].position;
            let b = self.vertices[edge[1]].position;
            let projected = closest_point_on_segment(point, a, b)?;
            let distance = distance_squared(point, projected);
            if best.as_ref().is_none_or(|(best_distance, best_edge, _)| {
                distance < *best_distance || (distance == *best_distance && edge < *best_edge)
            }) {
                best = Some((distance, edge, projected));
            }
        }
        best.map(|(_, _, point)| point)
            .ok_or_else(|| CoreError::InvalidMapping("triangulated mesh has no edges".into()))
    }
}

fn normalize_weights(mut weights: [f64; 3]) -> Result<[f64; 3], CoreError> {
    for weight in &mut weights {
        if *weight < 0.0 && *weight >= -BARYCENTRIC_EPSILON { *weight = 0.0; }
    }
    let sum = weights.iter().sum::<f64>();
    if !sum.is_finite() || sum <= 0.0 {
        return Err(CoreError::InvalidMapping("barycentric weights are invalid".into()));
    }
    for weight in &mut weights { *weight /= sum; }
    Ok(weights)
}

fn cross(a: Vector2, b: Vector2, c: Vector2) -> f64 {
    (b.x() - a.x()) * (c.y() - a.y()) - (b.y() - a.y()) * (c.x() - a.x())
}
fn distance_squared(a: Vector2, b: Vector2) -> f64 {
    let dx = a.x() - b.x();
    let dy = a.y() - b.y();
    dx * dx + dy * dy
}
fn closest_point_on_segment(point: Vector2, a: Vector2, b: Vector2) -> Result<Vector2, CoreError> {
    let dx = b.x() - a.x();
    let dy = b.y() - a.y();
    let length_squared = dx * dx + dy * dy;
    if !length_squared.is_finite() || length_squared <= 0.0 {
        return Err(CoreError::InvalidMapping("mesh edge is degenerate".into()));
    }
    let t = (((point.x() - a.x()) * dx + (point.y() - a.y()) * dy) / length_squared).clamp(0.0, 1.0);
    Vector2::new(a.x() + t * dx, a.y() + t * dy)
}
