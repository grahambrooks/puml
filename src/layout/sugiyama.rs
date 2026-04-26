//! Shared layout primitives from the Sugiyama hierarchical-layout family.
//!
//! Each diagram layout does its own rank assignment and placement, then
//! shares the steps in this module:
//!
//! - `reorder_barycentric` — crossing reduction within adjacent layers.
//! - `assign_x_median` / `assign_grid_columns` — horizontal placement.
//! - `orthogonal_through_ports` — orthogonal polyline between two
//!   pre-selected ports (paired with `layout::ports::pick_port`).
//! - `nudge_overlapping_segments` — channel-routing nudge that spreads
//!   parallel rails apart so they don't draw on top of each other.
//!
//! Network-simplex ranking and full Brandes–Köpf coordinate assignment are
//! still future work; the median-based placement here is enough for the
//! tree- and DAG-shaped inputs typical of UML class/state/component
//! diagrams.
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

/// Bidirectional median placement — a simplified Brandes & Köpf (2002).
///
/// Runs `assign_x_median` top-down (each node aims at the median of its
/// predecessors), then a second pass bottom-up (each node aims at the
/// median of its successors), and averages the two before compacting
/// left-to-right within each layer to enforce `min_gap`.
///
/// The averaging gives layouts whose long edges read closer to vertical
/// even when the graph is asymmetric — a node with one parent above and
/// three children below pulls toward the children too, instead of
/// stacking under its single parent and letting the children drift right.
/// Without the conflict-edge marking and 4-variant balancing of the full
/// algorithm, this is ~50 lines instead of ~300, and captures the main
/// visual benefit for the tree- and DAG-shaped diagrams we render.
pub fn assign_x_balanced(
    layers: &[Vec<usize>],
    edges: &[(usize, usize)],
    widths: &[f64],
    min_gap: f64,
    side_margin: f64,
) -> HashMap<usize, f64> {
    if layers.is_empty() {
        return HashMap::new();
    }

    let down = assign_x_median(layers, edges, widths, min_gap, side_margin);

    // Bottom-up: reverse the layer order, then run the same median
    // routine. `assign_x_median` only looks at adjacent-layer edges, so
    // reversing layers turns its "predecessor median" into a "successor
    // median" without any other change.
    let reversed_layers: Vec<Vec<usize>> = layers.iter().rev().cloned().collect();
    let up = assign_x_median(&reversed_layers, edges, widths, min_gap, side_margin);

    // Average both placements per node, then compact each layer
    // left-to-right so widths and min_gap are honoured. Compaction may
    // shift nodes right of their average target — that's fine; the
    // ordering and relative spacing are what matter visually.
    let mut target: HashMap<usize, f64> = HashMap::new();
    for layer in layers {
        for &n in layer {
            let d = down.get(&n).copied();
            let u = up.get(&n).copied();
            let avg = match (d, u) {
                (Some(a), Some(b)) => (a + b) / 2.0,
                (Some(a), None) | (None, Some(a)) => a,
                _ => side_margin,
            };
            target.insert(n, avg);
        }
    }

    let mut x: HashMap<usize, f64> = HashMap::new();
    for layer in layers {
        let mut cursor = side_margin;
        for &n in layer {
            let w = widths.get(n).copied().unwrap_or(0.0);
            let placed = target.get(&n).copied().unwrap_or(cursor).max(cursor);
            x.insert(n, placed);
            cursor = placed + w + min_gap;
        }
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

/// Assign every node an integer column index so the diagram lays out on a
/// strict uniform grid.
///
/// Intended for diagrams where every node has the same width (e.g. a class
/// diagram with all boxes sized to the largest class). On that grid an
/// edge between a parent and a child in the same column collapses to a
/// single straight vertical line; edges that cross columns pick up a clean
/// L-bend at the midway y because every rail aligns to a grid multiple.
///
/// Top layer is packed 0, 1, 2, …. Each subsequent layer places its nodes
/// using the *median column* of their predecessors, then enforces a strictly
/// increasing column sequence so no two nodes in the same layer share a
/// column. Orphans with no adjacent-layer neighbour advance by one column
/// past the previous node.
///
/// `edges` references global node indices; only adjacent-layer edges are
/// considered — longer edges need virtual nodes to influence placement.
pub fn assign_grid_columns(
    layers: &[Vec<usize>],
    edges: &[(usize, usize)],
) -> HashMap<usize, usize> {
    let mut col: HashMap<usize, usize> = HashMap::new();
    if layers.is_empty() {
        return col;
    }

    // Top layer: pack left-to-right.
    if let Some(top) = layers.first() {
        for (i, &n) in top.iter().enumerate() {
            col.insert(n, i);
        }
    }

    // Layer index and adjacent-layer predecessor lists.
    let mut layer_of: HashMap<usize, usize> = HashMap::new();
    for (li, layer) in layers.iter().enumerate() {
        for &idx in layer {
            layer_of.insert(idx, li);
        }
    }
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

    // Walk each subsequent layer left-to-right, placing each node at the
    // median column of its predecessors (or one past the previous node if
    // the median target is already taken).
    for layer in layers.iter().skip(1) {
        let mut next_min_col = 0usize;
        for &n in layer {
            let target = up
                .get(n)
                .and_then(|preds| median_column(preds, &col))
                .unwrap_or(next_min_col);
            let placed = target.max(next_min_col);
            col.insert(n, placed);
            next_min_col = placed + 1;
        }
    }

    col
}

/// Median column index of `nodes` according to the partially-filled `col`
/// map. `None` if no predecessor has a column assigned yet.
fn median_column(nodes: &[usize], col: &HashMap<usize, usize>) -> Option<usize> {
    let mut cols: Vec<usize> = nodes.iter().filter_map(|p| col.get(p).copied()).collect();
    if cols.is_empty() {
        return None;
    }
    cols.sort();
    Some(cols[cols.len() / 2])
}

/// Which side of a node an edge endpoint sits on. Determines whether the
/// first (or last) segment of the orthogonal route leaves the node
/// vertically (Top/Bottom) or horizontally (Left/Right).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    /// True if the first segment leaving this port is vertical.
    pub fn is_vertical(self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }
}

/// Route an orthogonal polyline between two pre-selected ports.
///
/// Callers pick the port (top/bottom/left/right) on each node — usually via
/// `layout::ports::pick_port` — and this function emits the polyline. A
/// decision diamond wants its top point for entry and side points for
/// branch exits, not its bounding-box midpoints; class boxes typically
/// want bottom→top for parent→child; sibling-rank associations want
/// side→side. Port selection makes that choice; this routes accordingly.
///
/// The router picks the bend point(s) based on whether each port exits its
/// node vertically or horizontally. Four combinations:
///
/// - Both vertical (top/bottom → top/bottom): one horizontal bend at the
///   midway y. Collapses to a straight line if `src.x == dst.x`.
/// - Both horizontal (left/right → left/right): one vertical bend at the
///   midway x. Collapses if `src.y == dst.y`.
/// - Vertical out, horizontal in: a single L-shaped bend at `(src.x, dst.y)`.
/// - Horizontal out, vertical in: mirror, bend at `(dst.x, src.y)`.
pub fn orthogonal_through_ports(
    src: (f64, f64),
    src_side: Side,
    dst: (f64, f64),
    dst_side: Side,
) -> Vec<(f64, f64)> {
    let (sx, sy) = src;
    let (dx, dy) = dst;
    const ALIGN_EPS: f64 = 0.75;

    match (src_side.is_vertical(), dst_side.is_vertical()) {
        (true, true) => {
            if (sx - dx).abs() < ALIGN_EPS {
                return vec![src, dst];
            }
            let my = (sy + dy) / 2.0;
            vec![src, (sx, my), (dx, my), dst]
        }
        (false, false) => {
            if (sy - dy).abs() < ALIGN_EPS {
                return vec![src, dst];
            }
            let mx = (sx + dx) / 2.0;
            vec![src, (mx, sy), (mx, dy), dst]
        }
        (true, false) => {
            // Out the top/bottom, into the left/right — one L-bend.
            if (sx - dx).abs() < ALIGN_EPS || (sy - dy).abs() < ALIGN_EPS {
                return vec![src, dst];
            }
            vec![src, (sx, dy), dst]
        }
        (false, true) => {
            if (sx - dx).abs() < ALIGN_EPS || (sy - dy).abs() < ALIGN_EPS {
                return vec![src, dst];
            }
            vec![src, (dx, sy), dst]
        }
    }
}

/// Spread overlapping orthogonal segments across parallel lanes so they
/// don't draw on top of each other.
///
/// Operates only on the middle segment of 4-point "Z-bend" routes — the
/// shape produced by `orthogonal_through_ports` for
/// rank-to-rank edges. The endpoint segments stay anchored to their ports;
/// only the interior rail moves, and the two segments adjacent to it
/// stretch or shrink to keep the polyline orthogonal.
///
/// Two routes are considered to share a rail if their middle-segment rail
/// coordinates differ by less than `RAIL_TOLERANCE`. They overlap if their
/// spans (extents along the rail's axis) intersect. Each overlap cluster
/// is distributed across lanes spaced `gap` apart, centred on the original
/// rail — so nudging is symmetric and doesn't bias one direction.
///
/// Two-point straight routes and three-point single-bend L-routes are left
/// alone: their bend points are anchored to box-edge midpoints, and moving
/// them would break port attachment.
pub fn nudge_overlapping_segments(routes: &mut [Vec<(f64, f64)>], gap: f64) {
    const RAIL_TOLERANCE: f64 = 0.5;
    const SPAN_EPS: f64 = 0.5;

    // (route_idx, rail_coord, span_lo, span_hi) for horizontal and vertical
    // middle segments respectively.
    let mut horiz: Vec<(usize, f64, f64, f64)> = Vec::new();
    let mut vert: Vec<(usize, f64, f64, f64)> = Vec::new();

    for (i, route) in routes.iter().enumerate() {
        if route.len() != 4 {
            continue;
        }
        let (x1, y1) = route[1];
        let (x2, y2) = route[2];
        if (y1 - y2).abs() < RAIL_TOLERANCE && (x1 - x2).abs() >= RAIL_TOLERANCE {
            horiz.push((i, (y1 + y2) / 2.0, x1.min(x2), x1.max(x2)));
        } else if (x1 - x2).abs() < RAIL_TOLERANCE && (y1 - y2).abs() >= RAIL_TOLERANCE {
            vert.push((i, (x1 + x2) / 2.0, y1.min(y2), y1.max(y2)));
        }
    }

    let dys = lane_offsets(&horiz, gap, RAIL_TOLERANCE, SPAN_EPS);
    let dxs = lane_offsets(&vert, gap, RAIL_TOLERANCE, SPAN_EPS);

    for (route_idx, dy) in dys {
        let route = &mut routes[route_idx];
        if route.len() == 4 {
            route[1].1 += dy;
            route[2].1 += dy;
        }
    }
    for (route_idx, dx) in dxs {
        let route = &mut routes[route_idx];
        if route.len() == 4 {
            route[1].0 += dx;
            route[2].0 += dx;
        }
    }
}

/// For a list of `(route_idx, rail, span_lo, span_hi)` entries, return the
/// per-route offset that pushes overlapping rails into distinct lanes.
fn lane_offsets(
    segs: &[(usize, f64, f64, f64)],
    gap: f64,
    rail_tol: f64,
    span_eps: f64,
) -> Vec<(usize, f64)> {
    if segs.len() < 2 {
        return Vec::new();
    }

    // Sort by rail so neighbouring rails cluster together.
    let mut by_rail: Vec<usize> = (0..segs.len()).collect();
    by_rail.sort_by(|&a, &b| {
        segs[a]
            .1
            .partial_cmp(&segs[b].1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut nudges: Vec<(usize, f64)> = Vec::new();
    let mut cluster_start = 0;
    while cluster_start < by_rail.len() {
        let mut cluster_end = cluster_start + 1;
        while cluster_end < by_rail.len()
            && (segs[by_rail[cluster_end]].1 - segs[by_rail[cluster_start]].1).abs() < rail_tol
        {
            cluster_end += 1;
        }
        let cluster = &by_rail[cluster_start..cluster_end];

        if cluster.len() > 1 {
            // Sweep within the cluster on span_lo to find overlap groups.
            let mut by_span: Vec<usize> = cluster.to_vec();
            by_span.sort_by(|&a, &b| {
                segs[a]
                    .2
                    .partial_cmp(&segs[b].2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut group_start = 0;
            while group_start < by_span.len() {
                let mut group_end = group_start + 1;
                let mut group_max = segs[by_span[group_start]].3;
                while group_end < by_span.len() && segs[by_span[group_end]].2 < group_max - span_eps
                {
                    group_max = group_max.max(segs[by_span[group_end]].3);
                    group_end += 1;
                }
                let group = &by_span[group_start..group_end];
                if group.len() > 1 {
                    // Lanes are centred on the original rail so the cluster
                    // doesn't bias upward or downward.
                    let count = group.len() as f64;
                    for (lane, &seg_idx) in group.iter().enumerate() {
                        let offset = (lane as f64 - (count - 1.0) / 2.0) * gap;
                        if offset.abs() > 1e-6 {
                            nudges.push((segs[seg_idx].0, offset));
                        }
                    }
                }
                group_start = group_end;
            }
        }
        cluster_start = cluster_end;
    }
    nudges
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

    #[test]
    fn ports_bottom_to_top_collapses_when_aligned() {
        // Diamond top exit + child top entry, aligned x.
        let pts = orthogonal_through_ports((100.0, 50.0), Side::Bottom, (100.0, 150.0), Side::Top);
        assert_eq!(pts, vec![(100.0, 50.0), (100.0, 150.0)]);
    }

    #[test]
    fn ports_bottom_to_top_bends_when_offset() {
        let pts = orthogonal_through_ports((100.0, 50.0), Side::Bottom, (200.0, 150.0), Side::Top);
        // Two bends at midway y, carrying the horizontal step.
        assert_eq!(pts.len(), 4);
        assert_eq!(pts[0], (100.0, 50.0));
        assert_eq!(pts[3], (200.0, 150.0));
        assert!((pts[1].1 - pts[2].1).abs() < 0.001);
        assert_eq!(pts[1].0, 100.0);
        assert_eq!(pts[2].0, 200.0);
    }

    #[test]
    fn ports_right_to_top_is_single_bend() {
        // Decision exits at its right point, target above-right enters at top.
        // Expect one L-bend at (dst.x, src.y).
        let pts = orthogonal_through_ports((100.0, 50.0), Side::Right, (250.0, 200.0), Side::Top);
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], (100.0, 50.0));
        assert_eq!(pts[1], (250.0, 50.0));
        assert_eq!(pts[2], (250.0, 200.0));
    }

    #[test]
    fn grid_columns_packs_top_layer() {
        let layers = vec![vec![0, 1, 2], vec![]];
        let col = assign_grid_columns(&layers, &[]);
        assert_eq!(col[&0], 0);
        assert_eq!(col[&1], 1);
        assert_eq!(col[&2], 2);
    }

    #[test]
    fn grid_columns_child_under_single_parent() {
        // One parent at col 2, one child. Child should land at col 2.
        let layers = vec![vec![10, 11, 12], vec![99]];
        let col = assign_grid_columns(&layers, &[(99, 12)]);
        assert_eq!(col[&12], 2);
        assert_eq!(col[&99], 2);
    }

    #[test]
    fn grid_columns_resolve_collision_via_bump() {
        // Two siblings both want column 1 (their parent). The second gets
        // bumped to column 2 to avoid sharing a column.
        let layers = vec![vec![7, 8], vec![20, 21]];
        let col = assign_grid_columns(&layers, &[(20, 8), (21, 8)]);
        assert_eq!(col[&8], 1);
        assert_eq!(col[&20], 1);
        assert_eq!(col[&21], 2);
    }

    #[test]
    fn grid_columns_median_of_two_parents() {
        // Child with two parents at cols 1 and 3 → median column is 1
        // (lower median by our sort convention). It doesn't have to be the
        // arithmetic mean, just a consistent tie-break.
        let layers = vec![vec![5, 6, 7, 8], vec![99]];
        let col = assign_grid_columns(&layers, &[(99, 6), (99, 8)]);
        assert_eq!(col[&6], 1);
        assert_eq!(col[&8], 3);
        assert!(col[&99] == 1 || col[&99] == 3);
    }

    #[test]
    fn balanced_centres_parent_over_children() {
        // Single parent above two children. The top-down pass anchors the
        // parent at the side margin and the children spread under it; the
        // bottom-up pass pulls the parent toward the children's median.
        // The averaged result should put the parent roughly above the
        // midpoint of its two children.
        let layers = vec![vec![0], vec![1, 2]];
        let widths = vec![100.0, 80.0, 80.0];
        let edges = vec![(1, 0), (2, 0)];
        let x = assign_x_balanced(&layers, &edges, &widths, 20.0, 10.0);
        let parent_centre = x[&0] + widths[0] / 2.0;
        let child_left = x[&1] + widths[1] / 2.0;
        let child_right = x[&2] + widths[2] / 2.0;
        let kids_midpoint = (child_left + child_right) / 2.0;
        // Allow a small margin — averaging plus min_gap compaction means
        // perfect alignment is not guaranteed, but the parent should be
        // close to the kids' midpoint.
        assert!(
            (parent_centre - kids_midpoint).abs() < 60.0,
            "parent centre {parent_centre} should be near kids' midpoint {kids_midpoint}"
        );
    }

    #[test]
    fn balanced_preserves_min_gap() {
        // Even with bidirectional pull, no two siblings should overlap.
        let layers = vec![vec![0], vec![1, 2]];
        let widths = vec![120.0, 80.0, 80.0];
        let edges = vec![(1, 0), (2, 0)];
        let x = assign_x_balanced(&layers, &edges, &widths, 20.0, 10.0);
        let left_end = x[&1] + widths[1];
        let right_start = x[&2];
        assert!(
            right_start >= left_end + 20.0 - 0.001,
            "children overlap: left ends at {left_end}, right starts at {right_start}"
        );
    }

    #[test]
    fn nudge_leaves_isolated_route_unchanged() {
        let mut routes = vec![vec![(0.0, 0.0), (0.0, 50.0), (100.0, 50.0), (100.0, 100.0)]];
        let original = routes.clone();
        nudge_overlapping_segments(&mut routes, 6.0);
        assert_eq!(routes, original);
    }

    #[test]
    fn nudge_separates_two_overlapping_horizontal_rails() {
        // Two 4-point routes with horizontal middle segments at the same y
        // and overlapping x spans. They should be split symmetrically.
        let mut routes = vec![
            vec![(0.0, 0.0), (0.0, 50.0), (100.0, 50.0), (100.0, 100.0)],
            vec![(20.0, 0.0), (20.0, 50.0), (80.0, 50.0), (80.0, 100.0)],
        ];
        nudge_overlapping_segments(&mut routes, 6.0);
        // Middle rails moved to y = 50 ± 3.
        let r0_y = routes[0][1].1;
        let r1_y = routes[1][1].1;
        assert!((r0_y - 47.0).abs() < 0.001 || (r0_y - 53.0).abs() < 0.001);
        assert!((r1_y - 47.0).abs() < 0.001 || (r1_y - 53.0).abs() < 0.001);
        assert!((r0_y - r1_y).abs() > 5.0);
        // Endpoint segments stay anchored to ports.
        assert_eq!(routes[0][0], (0.0, 0.0));
        assert_eq!(routes[0][3], (100.0, 100.0));
        assert_eq!(routes[1][0], (20.0, 0.0));
        assert_eq!(routes[1][3], (80.0, 100.0));
        // Rails still horizontal after nudging.
        assert!((routes[0][1].1 - routes[0][2].1).abs() < 0.001);
        assert!((routes[1][1].1 - routes[1][2].1).abs() < 0.001);
    }

    #[test]
    fn nudge_ignores_non_overlapping_spans() {
        // Same rail y, but the spans don't intersect — leave alone.
        let mut routes = vec![
            vec![(0.0, 0.0), (0.0, 50.0), (40.0, 50.0), (40.0, 100.0)],
            vec![(80.0, 0.0), (80.0, 50.0), (120.0, 50.0), (120.0, 100.0)],
        ];
        let original = routes.clone();
        nudge_overlapping_segments(&mut routes, 6.0);
        assert_eq!(routes, original);
    }

    #[test]
    fn nudge_preserves_two_point_straight_routes() {
        // Two-point routes are anchored to ports at both ends and must not
        // be touched even if they share a coordinate with other routes.
        let mut routes = vec![
            vec![(50.0, 0.0), (50.0, 100.0)],
            vec![(50.0, 0.0), (50.0, 100.0)],
        ];
        let original = routes.clone();
        nudge_overlapping_segments(&mut routes, 6.0);
        assert_eq!(routes, original);
    }

    #[test]
    fn nudge_separates_three_overlapping_rails_symmetrically() {
        let mut routes = vec![
            vec![(0.0, 0.0), (0.0, 50.0), (100.0, 50.0), (100.0, 100.0)],
            vec![(10.0, 0.0), (10.0, 50.0), (90.0, 50.0), (90.0, 100.0)],
            vec![(20.0, 0.0), (20.0, 50.0), (80.0, 50.0), (80.0, 100.0)],
        ];
        nudge_overlapping_segments(&mut routes, 6.0);
        // Three lanes: -gap, 0, +gap → y = 44, 50, 56.
        let mut ys: Vec<f64> = routes.iter().map(|r| r[1].1).collect();
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((ys[0] - 44.0).abs() < 0.001);
        assert!((ys[1] - 50.0).abs() < 0.001);
        assert!((ys[2] - 56.0).abs() < 0.001);
    }
}
