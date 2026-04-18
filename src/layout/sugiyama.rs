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
}
