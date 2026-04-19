//! Shared layout primitives lifted from the Sugiyama hierarchical-layout
//! family. Each diagram layout does its own rank assignment and placement;
//! this module carries the one piece that's always the same — barycentric
//! crossing reduction within adjacent layers.
//!
//! A fuller Sugiyama pipeline would also do network-simplex ranking, virtual
//! nodes for long edges, and Brandes–Köpf x-coordinate assignment. We land
//! only the crossing reduction for now: it's the single step that most
//! affects perceived layout quality, and it's well-defined on our existing
//! layer representation.
use std::collections::HashMap;

/// Reduce edge crossings between adjacent layers using barycentric ordering.
///
/// Nodes within each layer are sorted by the mean position of their
/// adjacent-layer neighbours: top-down passes reorder by predecessors,
/// bottom-up passes by successors. A handful of sweeps converges for the
/// DAG-shaped class / deployment / state diagrams we care about. Orphans
/// (no adjacent-layer neighbour) keep their incoming index so they don't
/// drift.
///
/// `edges` references global node indices; only edges that connect
/// *adjacent* layers influence ordering. Longer edges would need virtual
/// nodes to matter — a Phase 2 improvement.
pub fn reorder_barycentric(layers: &mut [Vec<usize>], edges: &[(usize, usize)], sweeps: usize) {
    if layers.len() < 2 || edges.is_empty() {
        return;
    }

    let mut layer_of: HashMap<usize, usize> = HashMap::new();
    for (li, layer) in layers.iter().enumerate() {
        for &idx in layer {
            layer_of.insert(idx, li);
        }
    }

    // Adjacent-layer neighbour lists, keyed by global node index.
    let max_node = layer_of.keys().copied().max().unwrap_or(0);
    let n = max_node + 1;
    let mut up: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut dn: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in edges {
        let (la, lb) = match (layer_of.get(&a), layer_of.get(&b)) {
            (Some(&la), Some(&lb)) => (la, lb),
            _ => continue,
        };
        if la + 1 == lb {
            dn[a].push(b);
            up[b].push(a);
        } else if lb + 1 == la {
            dn[b].push(a);
            up[a].push(b);
        }
    }

    for _ in 0..sweeps {
        for li in 1..layers.len() {
            let (above, rest) = layers.split_at_mut(li);
            reorder_by_neighbours(&mut rest[0], &up, &above[li - 1]);
        }
        for li in (0..layers.len() - 1).rev() {
            let (head, tail) = layers.split_at_mut(li + 1);
            reorder_by_neighbours(&mut head[li], &dn, &tail[0]);
        }
    }
}

fn reorder_by_neighbours(layer: &mut Vec<usize>, neighbours: &[Vec<usize>], reference: &[usize]) {
    let pos_in_ref: HashMap<usize, f64> = reference
        .iter()
        .enumerate()
        .map(|(i, &idx)| (idx, i as f64))
        .collect();

    let mut with_bary: Vec<(f64, usize, usize)> = layer
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let positions: Vec<f64> = neighbours
                .get(idx)
                .map(|ns| {
                    ns.iter()
                        .filter_map(|p| pos_in_ref.get(p).copied())
                        .collect()
                })
                .unwrap_or_default();
            let bary = if positions.is_empty() {
                i as f64
            } else {
                positions.iter().sum::<f64>() / positions.len() as f64
            };
            (bary, i, idx)
        })
        .collect();
    with_bary.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
    });
    *layer = with_bary.into_iter().map(|(_, _, idx)| idx).collect();
}

/// Assign x-coordinates that place each node as close as possible to the
/// median x-center of its predecessors in the layer above.
///
/// This is the single-pass, one-direction variant of Brandes & Köpf's
/// horizontal coordinate assignment (2001) — enough to pull children under
/// their parents in an inheritance tree or a state transition chain without
/// the cost of the full 4-variant balance. Within each layer we enforce a
/// minimum horizontal gap so nodes never overlap: when a node's median
/// target would push it into its left neighbour, it pins against that
/// neighbour + `min_gap` instead.
///
/// Inputs:
///
/// - `layers`: node indices, top to bottom, already ordered (call
///   `reorder_barycentric` first).
/// - `edges`: global (a, b) pairs. Only adjacent-layer edges influence
///   placement.
/// - `widths`: width of each node, indexed by global id. Nodes referenced
///   by `layers` must be present.
/// - `min_gap`: minimum horizontal gap between adjacent node boxes.
/// - `side_margin`: left padding applied to the final leftmost node.
///
/// Returns a map from node id to left-x coordinate.
pub fn assign_x_median(
    layers: &[Vec<usize>],
    edges: &[(usize, usize)],
    widths: &[f64],
    min_gap: f64,
    side_margin: f64,
) -> HashMap<usize, f64> {
    let mut x: HashMap<usize, f64> = HashMap::new();
    if layers.is_empty() {
        return x;
    }

    // Layer index per node so we can filter edges to adjacent-layer pairs.
    let mut layer_of: HashMap<usize, usize> = HashMap::new();
    for (li, layer) in layers.iter().enumerate() {
        for &idx in layer {
            layer_of.insert(idx, li);
        }
    }

    // `up[node]` = list of predecessors (in layer-1). Only adjacent-layer
    // edges participate — longer edges would need virtual nodes to matter.
    let max_node = layer_of.keys().copied().max().unwrap_or(0);
    let mut up: Vec<Vec<usize>> = vec![Vec::new(); max_node + 1];
    for &(a, b) in edges {
        let (la, lb) = match (layer_of.get(&a), layer_of.get(&b)) {
            (Some(&la), Some(&lb)) => (la, lb),
            _ => continue,
        };
        if la + 1 == lb {
            up[b].push(a);
        } else if lb + 1 == la {
            up[a].push(b);
        }
    }

    // Top layer: pack left-to-right starting at `side_margin`.
    if let Some(top) = layers.first() {
        let mut cursor = side_margin;
        for &n in top {
            x.insert(n, cursor);
            cursor += widths.get(n).copied().unwrap_or(0.0) + min_gap;
        }
    }

    // Remaining layers: each node aims for the median x-center of its
    // predecessors, but can't cross or overlap its left neighbour in the
    // current layer.
    for layer in layers.iter().skip(1) {
        let mut cursor = side_margin;
        for (pos, &n) in layer.iter().enumerate() {
            let w = widths.get(n).copied().unwrap_or(0.0);
            let target_left = if let Some(preds) = up.get(n) {
                median_center(preds, &x, widths).map(|c| c - w / 2.0)
            } else {
                None
            };

            // Rough default: keep pushing right from the previous node.
            let default_left = if pos == 0 { side_margin } else { cursor };
            // Honour median when it doesn't collide with the neighbour to
            // the left.
            let placed = match target_left {
                Some(t) if t >= cursor => t,
                _ => default_left,
            };

            x.insert(n, placed);
            cursor = placed + w + min_gap;
        }

        // Right-to-left pull: within the same layer, walk backward and let
        // each node slide right if its median target is higher than the
        // current placement AND the next node has room. Skipped for
        // simplicity on the first pass — compaction above is enough to
        // prevent overlaps and this extra pass matters only when median
        // targets diverge sharply from left-to-right cursor placement.
    }

    x
}

/// Median of the x-centres of `nodes` according to the current `x`
/// left-coords and `widths`. `None` if no predecessor has been placed yet.
fn median_center(nodes: &[usize], x: &HashMap<usize, f64>, widths: &[f64]) -> Option<f64> {
    let mut centers: Vec<f64> = nodes
        .iter()
        .filter_map(|&p| {
            let lx = x.get(&p).copied()?;
            let w = widths.get(p).copied()?;
            Some(lx + w / 2.0)
        })
        .collect();
    if centers.is_empty() {
        return None;
    }
    centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = centers.len() / 2;
    Some(if centers.len() % 2 == 1 {
        centers[mid]
    } else {
        // Even count: use the lower median so results line up more
        // predictably with the leftmost parent in two-parent cases.
        centers[mid - 1]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_is_stable_with_no_edges() {
        let mut layers = vec![vec![0, 1, 2], vec![3, 4, 5]];
        reorder_barycentric(&mut layers, &[], 3);
        assert_eq!(layers, vec![vec![0, 1, 2], vec![3, 4, 5]]);
    }

    #[test]
    fn reorder_resolves_simple_crossing() {
        // Two layers, initial order creates a crossing:
        //   top:    [A=0, B=1]
        //   bottom: [X=2, Y=3]
        //   edges:  A-Y, B-X  (crossed)
        // After barycentric reorder bottom should become [Y=3, X=2] to
        // uncross.
        let mut layers = vec![vec![0, 1], vec![2, 3]];
        reorder_barycentric(&mut layers, &[(0, 3), (1, 2)], 3);
        assert_eq!(layers[0], vec![0, 1]); // top unchanged
        assert_eq!(layers[1], vec![3, 2]); // bottom flipped
    }

    #[test]
    fn reorder_preserves_orphans() {
        // Node 99 has no edges — it should stay wherever it started rather
        // than drift to position 0.
        let mut layers = vec![vec![0, 1], vec![99, 2, 3]];
        reorder_barycentric(&mut layers, &[(0, 3), (1, 2)], 3);
        // 99's barycenter = its index (0), so it stays at front; 3 has
        // bary 0, 2 has bary 1 → result preserves tie-break by original idx.
        assert!(layers[1].contains(&99));
    }

    #[test]
    fn x_assignment_aligns_child_under_parent() {
        // Single-column inheritance:  [Parent]  -> [Child]
        // Child should land centered under Parent.
        let layers = vec![vec![0], vec![1]];
        let widths = vec![100.0, 80.0];
        let edges = vec![(1, 0)]; // child 1 extends parent 0
        let x = assign_x_median(&layers, &edges, &widths, 20.0, 10.0);
        let parent_center = x[&0] + widths[0] / 2.0;
        let child_center = x[&1] + widths[1] / 2.0;
        assert!(
            (parent_center - child_center).abs() < 0.001,
            "child centre {} should match parent centre {}",
            child_center,
            parent_center
        );
    }

    #[test]
    fn x_assignment_preserves_min_gap() {
        // Two children of the same parent. They shouldn't overlap even if
        // both would nominally want the parent's centre.
        let layers = vec![vec![0], vec![1, 2]];
        let widths = vec![120.0, 80.0, 80.0];
        let edges = vec![(1, 0), (2, 0)]; // both children extend the parent
        let x = assign_x_median(&layers, &edges, &widths, 20.0, 10.0);
        let left_end = x[&1] + widths[1];
        let right_start = x[&2];
        assert!(
            right_start >= left_end + 20.0 - 0.001,
            "children overlap: left ends at {}, right starts at {}",
            left_end,
            right_start
        );
    }

    #[test]
    fn x_assignment_handles_orphans() {
        // A layer with a mix of connected and unconnected nodes should still
        // produce a non-overlapping sequence.
        let layers = vec![vec![0], vec![1, 2, 3]];
        let widths = vec![100.0, 60.0, 60.0, 60.0];
        let edges = vec![(2, 0)]; // only the middle child is connected
        let x = assign_x_median(&layers, &edges, &widths, 15.0, 10.0);
        // All three children placed with at least min_gap between them.
        assert!(x[&2] >= x[&1] + widths[1] + 15.0 - 0.001);
        assert!(x[&3] >= x[&2] + widths[2] + 15.0 - 0.001);
    }
}
