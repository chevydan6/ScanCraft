//! Deterministic exact nearest-neighbor search over finite millimeter positions.
use crate::work::{Stage, WorkControl};
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Neighbor {
    pub index: usize,
    pub distance_mm: f64,
}
#[derive(Clone, Copy)]
struct Ranked(Neighbor);
impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Ranked {}
impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .distance_mm
            .total_cmp(&other.0.distance_mm)
            .then(self.0.index.cmp(&other.0.index))
    }
}
struct KQuery<'a> {
    point: [f64; 3],
    k: usize,
    exclude: Option<usize>,
    heap: BinaryHeap<Ranked>,
    visits: usize,
    work: &'a WorkControl,
}
struct Node {
    point: usize,
    minimum_index: usize,
    axis: usize,
    left: Option<usize>,
    right: Option<usize>,
}
pub struct SpatialIndex<'a> {
    points: &'a [[f64; 3]],
    nodes: Vec<Node>,
    root: Option<usize>,
}
impl<'a> SpatialIndex<'a> {
    /// Balanced median tree, bounded to one million points. Equal coordinate
    /// splits use original index; query distance ties select the lowest index.
    pub fn build(points: &'a [[f64; 3]], work: &WorkControl) -> Result<Self, String> {
        work.check()?;
        if points.len() > 1_000_000 || !points.iter().flatten().all(|v| v.is_finite()) {
            return Err("spatial index requires at most one million finite points".into());
        }
        let mut result = Self {
            points,
            nodes: Vec::with_capacity(points.len()),
            root: None,
        };
        let mut indices: Vec<_> = (0..points.len()).collect();
        result.root = result.partition(&mut indices, 0, work)?;
        work.check()?;
        Ok(result)
    }
    fn partition(
        &mut self,
        indices: &mut [usize],
        depth: usize,
        work: &WorkControl,
    ) -> Result<Option<usize>, String> {
        if indices.is_empty() {
            return Ok(None);
        }
        work.update(Stage::Indexing, self.nodes.len(), self.points.len())?;
        let axis = depth % 3;
        let middle = indices.len() / 2;
        indices.select_nth_unstable_by(middle, |a, b| {
            self.points[*a][axis]
                .total_cmp(&self.points[*b][axis])
                .then(a.cmp(b))
        });
        let (left, rest) = indices.split_at_mut(middle);
        let (point, right) = rest.split_first_mut().unwrap();
        let node = self.nodes.len();
        self.nodes.push(Node {
            point: *point,
            minimum_index: *point,
            axis,
            left: None,
            right: None,
        });
        let left = self.partition(left, depth + 1, work)?;
        let right = self.partition(right, depth + 1, work)?;
        self.nodes[node].left = left;
        self.nodes[node].right = right;
        self.nodes[node].minimum_index = [left, right]
            .into_iter()
            .flatten()
            .map(|child| self.nodes[child].minimum_index)
            .fold(*point, usize::min);
        Ok(Some(node))
    }
    /// None for an empty index or distances outside finite f64 representation.
    /// Cancellation never yields a partial nearest-neighbor answer.
    pub fn nearest(&self, point: [f64; 3], work: &WorkControl) -> Result<Option<Neighbor>, String> {
        work.check()?;
        if !point.iter().all(|v| v.is_finite()) {
            return Err("nearest-neighbor query must be finite".into());
        }
        let mut best = None;
        let mut visits = 0;
        if let Some(root) = self.root {
            self.search(root, point, &mut best, &mut visits, work)?;
        }
        work.check()?;
        Ok(best)
    }
    /// Exact sorted k-neighbor result; ties use original index. Exclusion is by
    /// index only, so distinct coincident points still count. No partial answer.
    pub fn k_nearest(
        &self,
        point: [f64; 3],
        k: usize,
        exclude: Option<usize>,
        work: &WorkControl,
    ) -> Result<Vec<Neighbor>, String> {
        work.check()?;
        if !point.iter().all(|x| x.is_finite())
            || !(1..=1024).contains(&k)
            || exclude.is_some_and(|i| i >= self.points.len())
            || k > self
                .points
                .len()
                .saturating_sub(usize::from(exclude.is_some()))
        {
            return Err("k-neighbor query requires finite coordinates, k=1..1024 available points and a valid excluded index".into());
        }
        let mut query = KQuery {
            point,
            k,
            exclude,
            heap: BinaryHeap::with_capacity(k),
            visits: 0,
            work,
        };
        if let Some(root) = self.root {
            self.search_k(root, &mut query)?;
        }
        work.check()?;
        if query.heap.len() != k {
            return Err("fewer than k finite neighbor distances".into());
        }
        Ok(query
            .heap
            .into_sorted_vec()
            .into_iter()
            .map(|n| n.0)
            .collect())
    }
    fn search_k(&self, index: usize, query: &mut KQuery<'_>) -> Result<(), String> {
        // Distances cannot improve on zero. Once k zero-distance neighbors
        // exist, only a lower original index can improve the ranked result.
        // This subtree minimum is conservative even when it includes exclude.
        // Do not apply this shortcut to positive radii or approximate zeros.
        if query.heap.len() == query.k
            && let Some(worst) = query.heap.peek()
            && worst.0.distance_mm == 0.
            && self.nodes[index].minimum_index >= worst.0.index
        {
            return Ok(());
        }
        query.visits += 1;
        if query.visits.is_multiple_of(32) {
            query.work.check()?;
        }
        let node = &self.nodes[index];
        let point = self.points[node.point];
        if query.exclude != Some(node.point) {
            let distance_mm = (query.point[0] - point[0])
                .hypot(query.point[1] - point[1])
                .hypot(query.point[2] - point[2]);
            if distance_mm.is_finite() {
                let candidate = Ranked(Neighbor {
                    index: node.point,
                    distance_mm,
                });
                if query.heap.len() < query.k {
                    query.heap.push(candidate);
                } else if query.heap.peek().is_some_and(|worst| candidate < *worst) {
                    query.heap.pop();
                    query.heap.push(candidate);
                }
            }
        }
        let delta = query.point[node.axis] - point[node.axis];
        let (near, far) = if delta <= 0. {
            (node.left, node.right)
        } else {
            (node.right, node.left)
        };
        if let Some(near) = near {
            self.search_k(near, query)?;
        }
        let bound = if query.heap.len() < query.k {
            f64::INFINITY
        } else {
            query.heap.peek().unwrap().0.distance_mm
        };
        if delta.abs() <= bound
            && let Some(far) = far
        {
            self.search_k(far, query)?;
        }
        Ok(())
    }
    fn search(
        &self,
        node: usize,
        point: [f64; 3],
        best: &mut Option<Neighbor>,
        visits: &mut usize,
        work: &WorkControl,
    ) -> Result<(), String> {
        *visits += 1;
        if (*visits).is_multiple_of(32) {
            work.check()?;
        }
        let node = &self.nodes[node];
        let q = self.points[node.point];
        let distance = (point[0] - q[0])
            .hypot(point[1] - q[1])
            .hypot(point[2] - q[2]);
        if distance.is_finite()
            && best.is_none_or(|b| {
                distance < b.distance_mm || (distance == b.distance_mm && node.point < b.index)
            })
        {
            *best = Some(Neighbor {
                index: node.point,
                distance_mm: distance,
            });
        }
        let delta = point[node.axis] - q[node.axis];
        let (near, far) = if delta <= 0. {
            (node.left, node.right)
        } else {
            (node.right, node.left)
        };
        if let Some(near) = near {
            self.search(near, point, best, visits, work)?;
        }
        // Include equality so a lower-index equidistant point cannot be pruned.
        if delta.abs() <= best.map_or(f64::INFINITY, |b| b.distance_mm)
            && let Some(far) = far
        {
            self.search(far, point, best, visits, work)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coincident_neighbors_are_exact_without_visiting_the_whole_tree() {
        let points = vec![[0.; 3]; 4096];
        let work = WorkControl::default();
        let index = SpatialIndex::build(&points, &work).unwrap();
        for exclude in [None, Some(0), Some(2), Some(4095)] {
            let mut query = KQuery {
                point: [0.; 3],
                k: 4,
                exclude,
                heap: BinaryHeap::with_capacity(4),
                visits: 0,
                work: &work,
            };
            index.search_k(index.root.unwrap(), &mut query).unwrap();
            let actual: Vec<_> = query
                .heap
                .into_sorted_vec()
                .into_iter()
                .map(|n| n.0.index)
                .collect();
            let expected: Vec<_> = (0..points.len())
                .filter(|i| Some(*i) != exclude)
                .take(4)
                .collect();
            assert_eq!(actual, expected);
            // A structural work bound, not a timing assertion. Previously all
            // 4096 nodes were visited per query, causing quadratic filtering.
            assert!(query.visits < 128, "visited {} nodes", query.visits);
        }
    }
    #[test]
    fn mixed_coincident_groups_keep_brute_force_order_for_all_query_types() {
        let points: Vec<_> = (0..257)
            .map(|i| [((i * 17) % 5) as f64, ((i * 13) % 3) as f64, 0.])
            .collect();
        let work = WorkControl::default();
        let index = SpatialIndex::build(&points, &work).unwrap();
        for point in [[0.; 3], [1., 2., 0.], [0.5, 0.5, 0.], [100., 20., 1.]] {
            for exclude in [None, Some(0), Some(19), Some(256)] {
                for k in [1, 4, 17, 256] {
                    let mut expected: Vec<_> = points
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| Some(*i) != exclude)
                        .map(|(i, p)| Neighbor {
                            index: i,
                            distance_mm: (point[0] - p[0])
                                .hypot(point[1] - p[1])
                                .hypot(point[2] - p[2]),
                        })
                        .collect();
                    expected.sort_by(|a, b| {
                        a.distance_mm
                            .total_cmp(&b.distance_mm)
                            .then(a.index.cmp(&b.index))
                    });
                    assert_eq!(
                        index.k_nearest(point, k, exclude, &work).unwrap(),
                        expected[..k]
                    );
                }
            }
        }
    }
    #[test]
    fn k_queries_match_brute_force_with_duplicates_ties_exclusion_and_guards() {
        let mut points: Vec<_> = (0..250)
            .map(|i| {
                [
                    ((i * 37) % 43) as f64,
                    ((i * 11) % 29) as f64,
                    ((i * 7) % 13) as f64,
                ]
            })
            .collect();
        points.extend([[0.; 3], [0.; 3], [1., 0., 0.], [-1., 0., 0.]]);
        let work = WorkControl::default();
        let index = SpatialIndex::build(&points, &work).unwrap();
        for query in [[0.; 3], [10., 10., 5.], points[25]] {
            for exclude in [None, Some(25), Some(250)] {
                for k in [1, 2, 17, 100] {
                    let mut expected: Vec<_> = points
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| Some(*i) != exclude)
                        .map(|(i, p)| {
                            Ranked(Neighbor {
                                index: i,
                                distance_mm: (query[0] - p[0])
                                    .hypot(query[1] - p[1])
                                    .hypot(query[2] - p[2]),
                            })
                        })
                        .collect();
                    expected.sort();
                    assert_eq!(
                        index.k_nearest(query, k, exclude, &work).unwrap(),
                        expected[..k].iter().map(|n| n.0).collect::<Vec<_>>()
                    );
                }
            }
        }
        for k in [0, 1025, points.len() + 1] {
            assert!(index.k_nearest([0.; 3], k, None, &work).is_err());
        }
        assert!(
            index
                .k_nearest([0.; 3], 1, Some(points.len()), &work)
                .is_err()
        );
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(index.k_nearest([0.; 3], 1, None, &cancelled).is_err());
        assert!(index.k_nearest([f64::NAN, 0., 0.], 1, None, &work).is_err());
    }
    #[test]
    fn exact_queries_match_independent_brute_force_including_duplicates_and_ties() {
        let mut points: Vec<_> = (0..4096)
            .map(|i| {
                [
                    ((i * 37) % 101) as f64 - 50.,
                    ((i * 53) % 173) as f64 - 86.,
                    ((i * 29) % 67) as f64 - 33.,
                ]
            })
            .collect();
        points.extend([[0.; 3], [0.; 3], [-1., 0., 0.], [1., 0., 0.]]);
        let work = WorkControl::default();
        let index = SpatialIndex::build(&points, &work).unwrap();
        for i in 0..500 {
            let p = [
                i as f64 * 0.31 - 75.,
                i as f64 * 0.13 - 20.,
                i as f64 * 0.19 - 45.,
            ];
            let expected = points
                .iter()
                .enumerate()
                .map(|(i, q)| Neighbor {
                    index: i,
                    distance_mm: (p[0] - q[0]).hypot(p[1] - q[1]).hypot(p[2] - q[2]),
                })
                .min_by(|a, b| {
                    a.distance_mm
                        .total_cmp(&b.distance_mm)
                        .then(a.index.cmp(&b.index))
                })
                .unwrap();
            assert_eq!(index.nearest(p, &work).unwrap(), Some(expected));
        }
        let ties = [[1., 0., 0.], [-1., 0., 0.], [1., 0., 0.]];
        assert_eq!(
            SpatialIndex::build(&ties, &work)
                .unwrap()
                .nearest([0.; 3], &work)
                .unwrap()
                .unwrap()
                .index,
            0
        );
    }
    #[test]
    fn empty_large_coordinates_invalid_inputs_and_cancellation_are_controlled() {
        let work = WorkControl::default();
        assert!(
            SpatialIndex::build(&[], &work)
                .unwrap()
                .nearest([0.; 3], &work)
                .unwrap()
                .is_none()
        );
        assert!(SpatialIndex::build(&[[f64::NAN, 0., 0.]], &work).is_err());
        let points = [[1e200, 0., 0.]];
        let index = SpatialIndex::build(&points, &work).unwrap();
        assert_eq!(
            index.nearest([0.; 3], &work).unwrap().unwrap().distance_mm,
            1e200
        );
        assert!(index.nearest([f64::INFINITY, 0., 0.], &work).is_err());
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(SpatialIndex::build(&points, &cancelled).is_err());
        assert!(index.nearest([0.; 3], &cancelled).is_err());
    }
}
