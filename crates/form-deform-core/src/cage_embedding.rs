//! Global validation for a piecewise-linear 2D cage embedding.
//!
//! A cage is an indexed triangular complex embedded in one Euclidean plane.
//! Validation is global: non-adjacent edges must not cross or touch, faces
//! must not overlap, and shared-edge topology must be manifold and consistent.
//! Numerically ambiguous or near-degenerate inputs are rejected, never repaired.

use crate::path2d::Point2;
use std::collections::BTreeMap;

const RELATIVE_EPSILON: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CageVertex {
    pub id: u64,
    pub position: Point2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CageTriangle {
    pub vertices: [usize; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct CageEmbedding {
    pub vertices: Vec<CageVertex>,
    pub triangles: Vec<CageTriangle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatedCageEmbedding {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub boundary_edge_count: usize,
    pub interior_edge_count: usize,
    pub connected_component_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CageEmbeddingError {
    EmptyVertices,
    EmptyTriangles,
    DuplicateVertexId { vertex: usize, id: u64 },
    DuplicateVertexPosition { first: usize, second: usize },
    UnusedVertex { vertex: usize },
    NonManifoldVertex { vertex: usize },
    SharedVertexEdgeOverlap { vertex: usize, first: usize, second: usize },
    InvalidVertexIndex { triangle: usize, vertex: usize },
    RepeatedTriangleVertex { triangle: usize },
    DuplicateTriangle { first: usize, second: usize },
    DegenerateTriangle { triangle: usize },
    InconsistentFaceOrientation { triangle: usize },
    NonManifoldEdge { a: usize, b: usize, incidence: usize },
    InconsistentSharedEdgeWinding { a: usize, b: usize },
    CrossingEdges { first: (usize, usize), second: (usize, usize) },
    NonAdjacentTouchingEdges { first: (usize, usize), second: (usize, usize) },
    OverlappingFaces { first: usize, second: usize },
    NumericallyAmbiguous { first: usize, second: usize },
    ArithmeticOverflow,
}

impl std::fmt::Display for CageEmbeddingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyVertices => write!(f, "cage must contain vertices"),
            Self::EmptyTriangles => write!(f, "cage must contain triangles"),
            Self::DuplicateVertexId { vertex, id } => write!(f, "vertex {vertex} duplicates stable ID {id}"),
            Self::DuplicateVertexPosition { first, second } => write!(f, "vertices {first} and {second} occupy the same position"),
            Self::UnusedVertex { vertex } => write!(f, "vertex {vertex} is not referenced by any face"),
            Self::NonManifoldVertex { vertex } => write!(f, "vertex {vertex} has a disconnected or non-manifold face link"),
            Self::SharedVertexEdgeOverlap { vertex, first, second } => write!(f, "edges from shared vertex {vertex} to {first} and {second} overlap"),
            Self::InvalidVertexIndex { triangle, vertex } => write!(f, "triangle {triangle} references missing vertex {vertex}"),
            Self::RepeatedTriangleVertex { triangle } => write!(f, "triangle {triangle} repeats a vertex"),
            Self::DuplicateTriangle { first, second } => write!(f, "triangles {first} and {second} have the same vertex set"),
            Self::DegenerateTriangle { triangle } => write!(f, "triangle {triangle} is degenerate or numerically ambiguous"),
            Self::InconsistentFaceOrientation { triangle } => write!(f, "triangle {triangle} has inconsistent winding"),
            Self::NonManifoldEdge { a, b, incidence } => write!(f, "edge ({a}, {b}) is incident to {incidence} faces"),
            Self::InconsistentSharedEdgeWinding { a, b } => write!(f, "adjacent faces traverse shared edge ({a}, {b}) in the same direction"),
            Self::CrossingEdges { first, second } => write!(f, "non-adjacent edges {first:?} and {second:?} cross"),
            Self::NonAdjacentTouchingEdges { first, second } => write!(f, "non-adjacent edges {first:?} and {second:?} touch"),
            Self::OverlappingFaces { first, second } => write!(f, "faces {first} and {second} overlap"),
            Self::NumericallyAmbiguous { first, second } => write!(f, "relationship between faces {first} and {second} is numerically ambiguous"),
            Self::ArithmeticOverflow => write!(f, "cage extent or geometric predicate overflowed"),
        }
    }
}
impl std::error::Error for CageEmbeddingError {}

#[derive(Clone, Copy)]
struct Tolerance { length: f64, area: f64 }

impl CageEmbedding {
    /// Validate the complete planar embedding and return its topology summary.
    /// Each undirected edge must have one or two incident faces; two-face
    /// edges must be traversed in opposite directions by consistently wound faces.
    pub fn validate_global(&self) -> Result<ValidatedCageEmbedding, CageEmbeddingError> {
        if self.vertices.is_empty() { return Err(CageEmbeddingError::EmptyVertices); }
        if self.triangles.is_empty() { return Err(CageEmbeddingError::EmptyTriangles); }
        let tol = self.tolerance()?;
        self.validate_vertex_identity(tol)?;
        self.validate_faces(tol)?;
        self.validate_vertex_links()?;
        let (edges, components) = self.validate_topology()?;
        self.validate_global_intersections(tol)?;
        Ok(ValidatedCageEmbedding {
            vertex_count: self.vertices.len(),
            triangle_count: self.triangles.len(),
            boundary_edge_count: edges.values().filter(|e| e.len() == 1).count(),
            interior_edge_count: edges.values().filter(|e| e.len() == 2).count(),
            connected_component_count: components,
        })
    }

    fn tolerance(&self) -> Result<Tolerance, CageEmbeddingError> {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for v in &self.vertices {
            let (x, y) = (v.position.x(), v.position.y());
            if !x.is_finite() || !y.is_finite() { return Err(CageEmbeddingError::ArithmeticOverflow); }
            min_x = min_x.min(x); min_y = min_y.min(y);
            max_x = max_x.max(x); max_y = max_y.max(y);
        }
        let diagonal = (max_x - min_x).hypot(max_y - min_y);
        let area = diagonal * diagonal * RELATIVE_EPSILON;
        let length = diagonal * RELATIVE_EPSILON;
        if !diagonal.is_finite() || diagonal == 0.0 || !area.is_finite() || !length.is_finite() {
            return Err(CageEmbeddingError::ArithmeticOverflow);
        }
        Ok(Tolerance { length: length.max(f64::MIN_POSITIVE), area: area.max(f64::MIN_POSITIVE) })
    }

    fn validate_vertex_identity(&self, tol: Tolerance) -> Result<(), CageEmbeddingError> {
        let mut ids = BTreeMap::new();
        for (i, v) in self.vertices.iter().enumerate() {
            if ids.insert(v.id, i).is_some() {
                return Err(CageEmbeddingError::DuplicateVertexId { vertex: i, id: v.id });
            }
        }
        for i in 0..self.vertices.len() {
            for j in (i + 1)..self.vertices.len() {
                if distance(self.vertices[i].position, self.vertices[j].position) <= tol.length {
                    return Err(CageEmbeddingError::DuplicateVertexPosition { first: i, second: j });
                }
            }
        }
        Ok(())
    }

    fn validate_faces(&self, tol: Tolerance) -> Result<(), CageEmbeddingError> {
        let mut seen = BTreeMap::new();
        let mut winding = 0_i8;
        for (i, face) in self.triangles.iter().enumerate() {
            let [a, b, c] = face.vertices;
            for v in face.vertices {
                if v >= self.vertices.len() { return Err(CageEmbeddingError::InvalidVertexIndex { triangle: i, vertex: v }); }
            }
            if a == b || b == c || c == a { return Err(CageEmbeddingError::RepeatedTriangleVertex { triangle: i }); }
            let mut key = [a, b, c]; key.sort_unstable();
            if let Some(first) = seen.insert(key, i) { return Err(CageEmbeddingError::DuplicateTriangle { first, second: i }); }
            let area = orient(self.p(a), self.p(b), self.p(c));
            if !area.is_finite() { return Err(CageEmbeddingError::ArithmeticOverflow); }
            if area.abs() <= tol.area { return Err(CageEmbeddingError::DegenerateTriangle { triangle: i }); }
            let sign = if area > 0.0 { 1 } else { -1 };
            if winding != 0 && winding != sign { return Err(CageEmbeddingError::InconsistentFaceOrientation { triangle: i }); }
            winding = sign;
        }
        Ok(())
    }

    /// Every used vertex must have one connected link: a path on the boundary
    /// or a cycle in the interior. This rejects bow-tie vertices where two
    /// otherwise valid triangle fans touch at only one indexed vertex.
    fn validate_vertex_links(&self) -> Result<(), CageEmbeddingError> {
        let mut links = vec![BTreeMap::<usize, Vec<usize>>::new(); self.vertices.len()];
        let mut used = vec![false; self.vertices.len()];
        for tri in &self.triangles {
            let [a, b, c] = tri.vertices;
            used[a] = true; used[b] = true; used[c] = true;
            links[a].entry(b).or_default().push(c);
            links[a].entry(c).or_default().push(b);
            links[b].entry(a).or_default().push(c);
            links[b].entry(c).or_default().push(a);
            links[c].entry(a).or_default().push(b);
            links[c].entry(b).or_default().push(a);
        }
        for vertex in 0..self.vertices.len() {
            if !used[vertex] {
                return Err(CageEmbeddingError::UnusedVertex { vertex });
            }
            let link = &links[vertex];
            let mut visited = std::collections::BTreeSet::new();
            let Some(start) = link.keys().next().copied() else {
                return Err(CageEmbeddingError::NonManifoldVertex { vertex });
            };
            let mut stack = vec![start];
            while let Some(node) = stack.pop() {
                if !visited.insert(node) { continue; }
                let Some(neighbors) = link.get(&node) else {
                    return Err(CageEmbeddingError::NonManifoldVertex { vertex });
                };
                if neighbors.len() > 2 || neighbors.is_empty() {
                    return Err(CageEmbeddingError::NonManifoldVertex { vertex });
                }
                stack.extend(neighbors.iter().copied());
            }
            if visited.len() != link.len() {
                return Err(CageEmbeddingError::NonManifoldVertex { vertex });
            }
            let endpoints = link.values().filter(|neighbors| neighbors.len() == 1).count();
            if endpoints != 0 && endpoints != 2 {
                return Err(CageEmbeddingError::NonManifoldVertex { vertex });
            }
        }
        Ok(())
    }

    fn validate_topology(&self) -> Result<(BTreeMap<(usize, usize), Vec<(usize, i8)>>, usize), CageEmbeddingError> {
        let mut edges = BTreeMap::<(usize, usize), Vec<(usize, i8)>>::new();
        let mut parent: Vec<usize> = (0..self.triangles.len()).collect();
        for (face, tri) in self.triangles.iter().enumerate() {
            let [a, b, c] = tri.vertices;
            for (from, to) in [(a,b), (b,c), (c,a)] {
                let key = (from.min(to), from.max(to));
                let direction = if from < to { 1 } else { -1 };
                let incident = edges.entry(key).or_default();
                incident.push((face, direction));
                if incident.len() > 2 {
                    return Err(CageEmbeddingError::NonManifoldEdge { a: key.0, b: key.1, incidence: incident.len() });
                }
                if incident.len() == 2 {
                    if incident[0].1 == incident[1].1 {
                        return Err(CageEmbeddingError::InconsistentSharedEdgeWinding { a: key.0, b: key.1 });
                    }
                    union(&mut parent, incident[0].0, incident[1].0);
                }
            }
        }
        let count = (0..parent.len()).filter(|&i| find(&mut parent, i) == i).count();
        Ok((edges, count))
    }

    fn validate_global_intersections(&self, tol: Tolerance) -> Result<(), CageEmbeddingError> {
        let mut edges = Vec::new();
        for tri in &self.triangles {
            let [a,b,c] = tri.vertices;
            edges.extend([(a.min(b),a.max(b)), (b.min(c),b.max(c)), (c.min(a),c.max(a))]);
        }
        edges.sort_unstable(); edges.dedup();
        for i in 0..edges.len() {
            for j in (i+1)..edges.len() {
                let (a,b) = edges[i]; let (c,d) = edges[j];
                let shared = [a, b].into_iter().find(|v| *v == c || *v == d);
                if let Some(vertex) = shared {
                    let first = if a == vertex { b } else { a };
                    let second = if c == vertex { d } else { c };
                    let origin = self.p(vertex);
                    let p = self.p(first);
                    let q = self.p(second);
                    let collinear = orient(origin, p, q).abs() <= tol.area;
                    let dot = (p.x() - origin.x()) * (q.x() - origin.x())
                        + (p.y() - origin.y()) * (q.y() - origin.y());
                    if !dot.is_finite() { return Err(CageEmbeddingError::ArithmeticOverflow); }
                    if collinear && dot > tol.length * tol.length {
                        return Err(CageEmbeddingError::SharedVertexEdgeOverlap { vertex, first, second });
                    }
                    continue;
                }
                match segment_relation(self.p(a), self.p(b), self.p(c), self.p(d), tol)? {
                    SegmentRelation::Disjoint => {},
                    SegmentRelation::Crossing => return Err(CageEmbeddingError::CrossingEdges { first: (a,b), second: (c,d) }),
                    SegmentRelation::Touching => return Err(CageEmbeddingError::NonAdjacentTouchingEdges { first: (a,b), second: (c,d) }),
                    SegmentRelation::Ambiguous => return Err(CageEmbeddingError::NumericallyAmbiguous { first: 0, second: 0 }),
                }
            }
        }
        // Detect containment: no edge crossing is required for one disconnected
        // face to lie wholly inside another.
        for i in 0..self.triangles.len() {
            for j in (i+1)..self.triangles.len() {
                let a = self.triangles[i].vertices;
                let b = self.triangles[j].vertices;
                if a.iter().any(|v| b.contains(v)) { continue; }
                let pa = a.map(|v| self.p(v));
                let pb = b.map(|v| self.p(v));
                if pa.iter().any(|p| inside_triangle(*p, pb, tol))
                    || pb.iter().any(|p| inside_triangle(*p, pa, tol)) {
                    return Err(CageEmbeddingError::OverlappingFaces { first: i, second: j });
                }
            }
        }
        Ok(())
    }

    fn p(&self, index: usize) -> Point2 { self.vertices[index].position }
}

#[derive(Clone, Copy)]
enum SegmentRelation { Disjoint, Crossing, Touching, Ambiguous }

fn orient(a: Point2, b: Point2, c: Point2) -> f64 {
    (b.x()-a.x())*(c.y()-a.y()) - (b.y()-a.y())*(c.x()-a.x())
}
fn distance(a: Point2, b: Point2) -> f64 { (a.x()-b.x()).hypot(a.y()-b.y()) }
fn sign(v: f64, eps: f64) -> i8 { if v.abs() <= eps { 0 } else if v > 0.0 { 1 } else { -1 } }
fn on_segment(a: Point2, b: Point2, p: Point2, eps: f64) -> bool {
    p.x() >= a.x().min(b.x())-eps && p.x() <= a.x().max(b.x())+eps
        && p.y() >= a.y().min(b.y())-eps && p.y() <= a.y().max(b.y())+eps
}
fn segment_relation(a: Point2, b: Point2, c: Point2, d: Point2, tol: Tolerance) -> Result<SegmentRelation, CageEmbeddingError> {
    let o = [orient(a,b,c), orient(a,b,d), orient(c,d,a), orient(c,d,b)];
    if o.iter().any(|v| !v.is_finite()) { return Err(CageEmbeddingError::ArithmeticOverflow); }
    let s = o.map(|v| sign(v, tol.area));
    if s.contains(&0) && o.iter().zip(s).any(|(v, sign)| sign == 0 && v.abs() > tol.area * 0.5) {
        return Ok(SegmentRelation::Ambiguous);
    }
    if s[0]*s[1] < 0 && s[2]*s[3] < 0 { return Ok(SegmentRelation::Crossing); }
    if (s[0] == 0 && on_segment(a,b,c,tol.length))
        || (s[1] == 0 && on_segment(a,b,d,tol.length))
        || (s[2] == 0 && on_segment(c,d,a,tol.length))
        || (s[3] == 0 && on_segment(c,d,b,tol.length)) {
        return Ok(SegmentRelation::Touching);
    }
    Ok(SegmentRelation::Disjoint)
}
fn inside_triangle(p: Point2, t: [Point2;3], tol: Tolerance) -> bool {
    let s = [sign(orient(t[0],t[1],p),tol.area), sign(orient(t[1],t[2],p),tol.area), sign(orient(t[2],t[0],p),tol.area)];
    s.iter().all(|v| *v > 0) || s.iter().all(|v| *v < 0)
}
fn find(parent: &mut [usize], i: usize) -> usize {
    if parent[i] != i { parent[i] = find(parent, parent[i]); }
    parent[i]
}
fn union(parent: &mut [usize], a: usize, b: usize) {
    let (ra,rb) = (find(parent,a),find(parent,b));
    if ra != rb { parent[rb] = ra; }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn v(id:u64,x:f64,y:f64)->CageVertex { CageVertex { id, position: Point2::new(x,y).unwrap() } }
    fn t(a:usize,b:usize,c:usize)->CageTriangle { CageTriangle { vertices:[a,b,c] } }
    fn square()->CageEmbedding {
        CageEmbedding { vertices:vec![v(1,0.,0.),v(2,10.,0.),v(3,10.,10.),v(4,0.,10.)], triangles:vec![t(0,1,2),t(0,2,3)] }
    }

    #[test]
    fn accepts_global_manifold_embedding_and_reports_topology() {
        let result = square().validate_global().unwrap();
        assert_eq!((result.boundary_edge_count,result.interior_edge_count,result.connected_component_count),(4,1,1));
    }
    #[test]
    fn rejects_duplicate_ids_positions_and_bad_indices() {
        let mut c=square(); c.vertices[1].id=1;
        assert!(matches!(c.validate_global(),Err(CageEmbeddingError::DuplicateVertexId{..})));
        let mut c=square(); c.vertices[1].position=c.vertices[0].position;
        assert!(matches!(c.validate_global(),Err(CageEmbeddingError::DuplicateVertexPosition{..})));
        let mut c=square(); c.triangles[0]=t(0,1,9);
        assert!(matches!(c.validate_global(),Err(CageEmbeddingError::InvalidVertexIndex{..})));
    }
    #[test]
    fn rejects_bow_tie_vertex_with_disconnected_face_fans() {
        let cage = CageEmbedding {
            vertices: vec![v(1,0.,0.),v(2,2.,0.),v(3,1.,1.),v(4,0.,2.),v(5,2.,2.)],
            triangles: vec![t(0,1,2),t(0,3,4)],
        };
        assert!(matches!(cage.validate_global(), Err(CageEmbeddingError::NonManifoldVertex { vertex: 0 })));
    }
    #[test]
    fn rejects_unused_vertices() {
        let mut cage = square();
        cage.vertices.push(v(5,20.,20.));
        assert!(matches!(cage.validate_global(), Err(CageEmbeddingError::UnusedVertex { vertex: 4 })));
    }
    #[test]
    fn rejects_collinear_edges_overlapping_after_shared_vertex() {
        let cage = CageEmbedding {
            vertices: vec![v(1,0.,0.),v(2,4.,0.),v(3,2.,0.),v(4,0.,3.),v(5,2.,3.)],
            triangles: vec![t(0,1,3),t(0,2,4)],
        };
        assert!(matches!(cage.validate_global(), Err(CageEmbeddingError::SharedVertexEdgeOverlap { .. })));
    }
    #[test]
    fn rejects_degenerate_duplicate_and_mixed_winding_faces() {
        let mut c=square(); c.vertices[2].position=Point2::new(5.,0.).unwrap();
        assert!(matches!(c.validate_global(),Err(CageEmbeddingError::DegenerateTriangle{..})));
        let mut c=square(); c.triangles[1]=t(0,2,3);
        assert!(matches!(c.validate_global(),Err(CageEmbeddingError::DuplicateTriangle{..})));
        let mut c=square(); c.triangles[1]=t(3,2,0);
        assert!(matches!(c.validate_global(),Err(CageEmbeddingError::InconsistentFaceOrientation{..})));
    }
    #[test]
    fn rejects_crossing_edges_and_contained_overlapping_faces() {
        let crossing=CageEmbedding { vertices:vec![v(1,0.,0.),v(2,4.,4.),v(3,0.,4.),v(4,4.,0.),v(5,5.,5.)], triangles:vec![t(0,1,4),t(2,3,4)] };
        assert!(matches!(crossing.validate_global(),Err(CageEmbeddingError::CrossingEdges{..})|Err(CageEmbeddingError::InconsistentFaceOrientation{..})));
        let overlap=CageEmbedding { vertices:vec![v(1,0.,0.),v(2,10.,0.),v(3,0.,10.),v(4,1.,1.),v(5,2.,1.),v(6,1.,2.)], triangles:vec![t(0,1,2),t(3,4,5)] };
        assert!(matches!(overlap.validate_global(),Err(CageEmbeddingError::OverlappingFaces{..})));
    }
    #[test]
    fn rejects_non_manifold_edges_and_accepts_separate_components() {
        let mut c=square(); c.vertices.push(v(5,5.,-5.)); c.triangles.push(t(1,0,4));
        assert!(matches!(c.validate_global(),Err(CageEmbeddingError::NonManifoldEdge{..})|Err(CageEmbeddingError::InconsistentFaceOrientation{..})));
        let separate=CageEmbedding { vertices:vec![v(1,0.,0.),v(2,2.,0.),v(3,0.,2.),v(4,5.,0.),v(5,7.,0.),v(6,5.,2.)], triangles:vec![t(0,1,2),t(3,4,5)] };
        assert_eq!(separate.validate_global().unwrap().connected_component_count,2);
    }
}
