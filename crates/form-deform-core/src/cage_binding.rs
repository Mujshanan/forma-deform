//! Point-to-cage bindings built on the globally validated cage contract.
//!
//! Bindings store triangle-local barycentric coordinates keyed by stable cage
//! vertex IDs. They do not retain triangle indices, so harmless triangle-array
//! reordering does not invalidate an attachment.

use crate::cage_embedding::{CageEmbedding, CageEmbeddingError};
use crate::path2d::Point2;
use std::collections::{BTreeMap, BTreeSet};

const BARYCENTRIC_EPSILON: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BindingPoint {
    pub id: u64,
    pub position: Point2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundPoint {
    pub id: u64,
    /// Stable cage vertex IDs in ascending order.
    pub cage_vertex_ids: [u64; 3],
    /// Weights corresponding to cage_vertex_ids; sum to one.
    pub barycentric: [f64; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct CageBinding {
    cage_vertex_ids: Vec<u64>,
    points: Vec<BoundPoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CageBindingError {
    InvalidCage(CageEmbeddingError),
    DuplicatePointId { id: u64 },
    PointOutsideCage { point: u64 },
    InconsistentCageVertexSet,
    NonFiniteEvaluation { point: u64 },
}

impl std::fmt::Display for CageBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cage binding failed: {self:?}")
    }
}
impl std::error::Error for CageBindingError {}

impl CageBinding {
    /// Bind points to a globally valid reference cage.
    ///
    /// If a point lies in multiple triangles because it is on a shared edge
    /// or vertex, the triangle with the lexicographically smallest sorted
    /// stable vertex-ID triple is selected. Points outside the cage are
    /// rejected; no nearest-edge or extrapolation fallback is performed.
    pub fn bind(
        cage: &CageEmbedding,
        points: &[BindingPoint],
    ) -> Result<Self, CageBindingError> {
        cage.validate_global().map_err(CageBindingError::InvalidCage)?;

        let mut point_ids = BTreeSet::new();
        for point in points {
            if !point_ids.insert(point.id) {
                return Err(CageBindingError::DuplicatePointId { id: point.id });
            }
        }

        let mut bound = Vec::with_capacity(points.len());
        for point in points {
            let mut candidates = Vec::new();
            for triangle in &cage.triangles {
                let [ia, ib, ic] = triangle.vertices;
                let a = cage.vertices[ia].position;
                let b = cage.vertices[ib].position;
                let c = cage.vertices[ic].position;
                let denominator = orient(a, b, c);
                let wa = orient(point.position, b, c) / denominator;
                let wb = orient(a, point.position, c) / denominator;
                let wc = orient(a, b, point.position) / denominator;
                if !wa.is_finite() || !wb.is_finite() || !wc.is_finite() {
                    return Err(CageBindingError::PointOutsideCage { point: point.id });
                }
                if wa >= -BARYCENTRIC_EPSILON
                    && wb >= -BARYCENTRIC_EPSILON
                    && wc >= -BARYCENTRIC_EPSILON
                {
                    let mut pairs = [
                        (cage.vertices[ia].id, wa),
                        (cage.vertices[ib].id, wb),
                        (cage.vertices[ic].id, wc),
                    ];
                    pairs.sort_by_key(|pair| pair.0);
                    let ids = [pairs[0].0, pairs[1].0, pairs[2].0];
                    let mut weights = [pairs[0].1, pairs[1].1, pairs[2].1];
                    let sum = weights.iter().sum::<f64>();
                    if !sum.is_finite() || sum.abs() <= f64::MIN_POSITIVE {
                        return Err(CageBindingError::PointOutsideCage { point: point.id });
                    }
                    for weight in &mut weights {
                        *weight /= sum;
                    }
                    candidates.push((ids, weights));
                }
            }
            candidates.sort_by_key(|candidate| candidate.0);
            let Some((cage_vertex_ids, barycentric)) = candidates.into_iter().next() else {
                return Err(CageBindingError::PointOutsideCage { point: point.id });
            };
            bound.push(BoundPoint { id: point.id, cage_vertex_ids, barycentric });
        }

        let mut cage_vertex_ids = cage.vertices.iter().map(|vertex| vertex.id).collect::<Vec<_>>();
        cage_vertex_ids.sort_unstable();
        Ok(Self { cage_vertex_ids, points: bound })
    }

    pub fn points(&self) -> &[BoundPoint] {
        &self.points
    }

    /// Evaluate attachments against a deformed cage with the same stable
    /// vertex identity set. The current cage is passed through the same global
    /// validator used at bind time; no second topology validator is maintained.
    pub fn evaluate(
        &self,
        cage: &CageEmbedding,
    ) -> Result<Vec<BindingPoint>, CageBindingError> {
        cage.validate_global().map_err(CageBindingError::InvalidCage)?;
        let mut current_ids = cage.vertices.iter().map(|vertex| vertex.id).collect::<Vec<_>>();
        current_ids.sort_unstable();
        if current_ids != self.cage_vertex_ids {
            return Err(CageBindingError::InconsistentCageVertexSet);
        }

        let positions = cage.vertices.iter()
            .map(|vertex| (vertex.id, vertex.position))
            .collect::<BTreeMap<_, _>>();
        let mut evaluated = Vec::with_capacity(self.points.len());
        for binding in &self.points {
            let mut x = 0.0;
            let mut y = 0.0;
            for index in 0..3 {
                let position = positions.get(&binding.cage_vertex_ids[index])
                    .ok_or(CageBindingError::InconsistentCageVertexSet)?;
                x += position.x() * binding.barycentric[index];
                y += position.y() * binding.barycentric[index];
            }
            let position = Point2::new(x, y)
                .map_err(|_| CageBindingError::NonFiniteEvaluation { point: binding.id })?;
            evaluated.push(BindingPoint { id: binding.id, position });
        }
        Ok(evaluated)
    }
}

fn orient(a: Point2, b: Point2, c: Point2) -> f64 {
    (b.x() - a.x()) * (c.y() - a.y()) - (b.y() - a.y()) * (c.x() - a.x())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cage_embedding::{CageTriangle, CageVertex};

    fn point(x: f64, y: f64) -> Point2 { Point2::new(x, y).unwrap() }

    fn square(offset_x: f64) -> CageEmbedding {
        CageEmbedding {
            vertices: vec![
                CageVertex { id: 10, position: point(offset_x, 0.0) },
                CageVertex { id: 20, position: point(offset_x + 10.0, 0.0) },
                CageVertex { id: 30, position: point(offset_x + 10.0, 10.0) },
                CageVertex { id: 40, position: point(offset_x, 10.0) },
            ],
            triangles: vec![
                CageTriangle { vertices: [0, 1, 2] },
                CageTriangle { vertices: [0, 2, 3] },
            ],
        }
    }

    #[test]
    fn bind_and_evaluate_preserves_point_at_rest() {
        let cage = square(0.0);
        let binding = CageBinding::bind(&cage, &[BindingPoint { id: 1, position: point(2.0, 3.0) }]).unwrap();
        assert_eq!(binding.evaluate(&cage).unwrap()[0].position, point(2.0, 3.0));
    }

    #[test]
    fn evaluation_follows_deformed_cage_vertices() {
        let rest = square(0.0);
        let binding = CageBinding::bind(&rest, &[BindingPoint { id: 1, position: point(2.0, 3.0) }]).unwrap();
        let deformed = square(5.0);
        assert_eq!(binding.evaluate(&deformed).unwrap()[0].position, point(7.0, 3.0));
    }

    #[test]
    fn shared_edge_binding_is_deterministic() {
        let cage = square(0.0);
        let point = BindingPoint { id: 1, position: point(5.0, 5.0) };
        let first = CageBinding::bind(&cage, &[point]).unwrap();
        let second = CageBinding::bind(&cage, &[point]).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.points()[0].cage_vertex_ids, [10, 20, 30]);
    }

    #[test]
    fn outside_points_and_duplicate_point_ids_are_rejected() {
        let cage = square(0.0);
        assert_eq!(
            CageBinding::bind(&cage, &[BindingPoint { id: 1, position: point(20.0, 20.0) }]),
            Err(CageBindingError::PointOutsideCage { point: 1 }),
        );
        assert_eq!(
            CageBinding::bind(&cage, &[
                BindingPoint { id: 1, position: point(1.0, 1.0) },
                BindingPoint { id: 1, position: point(2.0, 2.0) },
            ]),
            Err(CageBindingError::DuplicatePointId { id: 1 }),
        );
    }

    #[test]
    fn changed_vertex_identity_set_is_rejected() {
        let rest = square(0.0);
        let binding = CageBinding::bind(&rest, &[BindingPoint { id: 1, position: point(2.0, 3.0) }]).unwrap();
        let mut changed = square(0.0);
        changed.vertices[0].id = 99;
        assert_eq!(binding.evaluate(&changed), Err(CageBindingError::InconsistentCageVertexSet));
    }
}
