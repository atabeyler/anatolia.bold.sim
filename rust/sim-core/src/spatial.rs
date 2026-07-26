use rand::Rng;
use std::collections::HashMap;

/// A uniform grid over 2D positions used to bound neighbor lookups to O(1)
/// amortized instead of scanning the whole population for every individual.
pub struct SpatialGrid {
    cell_size: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
}

fn cell_key(x: f64, y: f64, cell_size: f64) -> (i64, i64) {
    ((x / cell_size).floor() as i64, (y / cell_size).floor() as i64)
}

impl SpatialGrid {
    pub fn build(positions: &[(f64, f64)], cell_size: f64) -> Self {
        let cell_size = cell_size.max(1e-6);
        let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (idx, (x, y)) in positions.iter().enumerate() {
            cells.entry(cell_key(*x, *y, cell_size)).or_default().push(idx);
        }
        Self { cell_size, cells }
    }

    /// Returns indices of points whose cell is within the 3x3 neighborhood of
    /// (x, y)'s cell. Callers must still check exact distance against `radius`
    /// since this is a coarse (over-inclusive) candidate set.
    ///
    /// `limit` bounds how many indices are ever *returned* -- every caller in
    /// this codebase already discards all but a small bounded sample
    /// downstream (MAX_CANDIDATE_SCAN / MAX_NEARBY_SAMPLE) -- but every
    /// candidate in the neighborhood is still visited once via reservoir
    /// sampling (Algorithm R) so each has an equal chance of ending up in the
    /// returned sample, regardless of insertion order. An earlier version
    /// `break`'d as soon as `limit` was reached, which always kept whichever
    /// elements were encountered first (a settlement's earliest-inserted,
    /// i.e. earliest-born, individuals) and silently starved everyone else --
    /// once a single cell's population exceeded `limit`, later-born
    /// individuals could never be selected as mates/trade partners/teachers/
    /// group-mates at all. Visiting every candidate costs O(cell population)
    /// instead of O(limit), but correctness here matters more than the
    /// constant-factor savings, and this is still bounded by `limit` in the
    /// one dimension that actually scaled unboundedly before (the copy/return
    /// size).
    pub fn candidates_within(&self, x: f64, y: f64, radius: f64, limit: usize) -> Vec<usize> {
        let span = (radius / self.cell_size).ceil().max(1.0) as i64;
        let (cx, cy) = cell_key(x, y, self.cell_size);
        let mut out: Vec<usize> = Vec::with_capacity(limit.min(256));
        let mut seen: usize = 0;
        let mut rng = rand::thread_rng();
        for dx in -span..=span {
            for dy in -span..=span {
                if let Some(bucket) = self.cells.get(&(cx + dx, cy + dy)) {
                    for &idx in bucket {
                        if out.len() < limit {
                            out.push(idx);
                        } else {
                            let j = rng.gen_range(0..=seen);
                            if j < limit {
                                out[j] = idx;
                            }
                        }
                        seen += 1;
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_all_points_within_radius_and_none_farther_away() {
        let positions = vec![(0.0, 0.0), (0.5, 0.5), (10.0, 10.0), (1.9, 0.0)];
        let grid = SpatialGrid::build(&positions, 2.0);
        let candidates = grid.candidates_within(0.0, 0.0, 2.0, usize::MAX);
        assert!(candidates.contains(&0));
        assert!(candidates.contains(&1));
        assert!(candidates.contains(&3));
        // The far point may or may not appear as a *candidate* (coarse), but an
        // exact-distance filter by the caller must exclude it.
        let exact: Vec<usize> = candidates
            .into_iter()
            .filter(|&i| {
                let (px, py) = positions[i];
                (px * px + py * py).sqrt() < 2.0
            })
            .collect();
        assert!(!exact.contains(&2));
    }

    #[test]
    fn candidates_within_samples_a_crowded_cell_uniformly_not_by_insertion_order() {
        // 200 points share one cell, well beyond `limit`. Before the fix,
        // candidates_within always kept indices 0..limit (whichever were
        // inserted first) and index 199 (the "last-born" individual) would
        // never once appear across any number of calls -- a real fairness
        // bug, not just a sampling quirk. With reservoir sampling every index
        // should show up with roughly equal frequency over many draws.
        let n = 200;
        let limit = 20;
        let positions: Vec<(f64, f64)> = (0..n).map(|_| (0.0, 0.0)).collect();
        let grid = SpatialGrid::build(&positions, 2.0);

        let mut seen_last_index = false;
        let mut counts = vec![0u32; n];
        let draws = 500;
        for _ in 0..draws {
            let candidates = grid.candidates_within(0.0, 0.0, 1.0, limit);
            assert_eq!(candidates.len(), limit);
            for idx in candidates {
                counts[idx] += 1;
                if idx == n - 1 {
                    seen_last_index = true;
                }
            }
        }
        assert!(seen_last_index, "the last-inserted point should be selectable, not permanently excluded");
        // Every point had an equal chance of being sampled; none should be
        // starved entirely across 500 independent draws of 20-from-200.
        let never_sampled = counts.iter().filter(|&&c| c == 0).count();
        assert!(never_sampled == 0, "{never_sampled} of {n} points were never sampled across {draws} draws");
    }
}
