//! Use case diagram layout: two columns (actors left, use cases right) with
//! barycentric row reordering to minimise edge crossings.
//!
//! Use case diagrams are essentially bipartite graphs — actors on one side,
//! use cases on the other, edges between them. The standard hierarchical
//! crossing-reduction trick (Sugiyama 1981, repeated barycentric sweeps)
//! converges quickly for the small graphs typical of use cases. Each sweep
//! reorders one column by the average row index of its neighbours in the
//! other column; alternating sweeps drive the total number of crossings
//! down to a local minimum.

use crate::ast::usecase::*;

const ACTOR_WIDTH: f64 = 60.0;
const ACTOR_HEIGHT: f64 = 80.0;
const USECASE_MIN_W: f64 = 120.0;
const USECASE_H: f64 = 50.0;
const SIDE_MARGIN: f64 = 40.0;
const TOP_MARGIN: f64 = 30.0;
const TITLE_H: f64 = 30.0;
const COLUMN_GAP: f64 = 120.0;
const ROW_GAP: f64 = 30.0;
const CHAR_W: f64 = 7.5;

/// Number of barycentric sweeps. For graphs with ≤ 30 nodes this converges
/// in 4–6 passes; doing 8 keeps headroom and is still microseconds.
const BARYCENTRIC_SWEEPS: usize = 8;

#[derive(Debug, Clone)]
pub struct UseCaseLayoutNode {
    pub name: String,
    pub display: String,
    pub kind: NodeKind,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub stereotype: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UseCaseLayoutEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub dashed: bool,
}

pub struct UseCaseLayout {
    pub nodes: Vec<UseCaseLayoutNode>,
    pub edges: Vec<UseCaseLayoutEdge>,
    pub title: Option<String>,
    pub total_width: f64,
    pub total_height: f64,
}

pub fn layout(diagram: &UseCaseDiagram) -> UseCaseLayout {
    let title_off = if diagram.title.is_some() {
        TITLE_H
    } else {
        0.0
    };

    if diagram.nodes.is_empty() {
        return UseCaseLayout {
            nodes: Vec::new(),
            edges: Vec::new(),
            title: diagram.title.clone(),
            total_width: 400.0,
            total_height: TOP_MARGIN + title_off + SIDE_MARGIN,
        };
    }

    // Split into left (actors) and right (use cases) columns, preserving
    // document order as the initial sort.
    let mut left: Vec<usize> = Vec::new();
    let mut right: Vec<usize> = Vec::new();
    for (i, node) in diagram.nodes.iter().enumerate() {
        match node.kind {
            NodeKind::Actor => left.push(i),
            NodeKind::UseCase => right.push(i),
        }
    }

    // Adjacency: for each node index, the list of cross-column neighbours.
    // Same-column edges (use case → use case include/extend) don't bias the
    // ordering — they're rendered but don't drive layout.
    let name_to_idx: std::collections::HashMap<&str, usize> = diagram
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.name.as_str(), i))
        .collect();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); diagram.nodes.len()];
    for a in &diagram.associations {
        let (Some(&i), Some(&j)) = (
            name_to_idx.get(a.from.as_str()),
            name_to_idx.get(a.to.as_str()),
        ) else {
            continue;
        };
        let cross = matches!(
            (&diagram.nodes[i].kind, &diagram.nodes[j].kind),
            (NodeKind::Actor, NodeKind::UseCase) | (NodeKind::UseCase, NodeKind::Actor)
        );
        if cross {
            adj[i].push(j);
            adj[j].push(i);
        }
    }

    // Alternating barycentric sweeps: sort right by left positions, then
    // left by right positions, and repeat.
    for _ in 0..BARYCENTRIC_SWEEPS {
        sort_by_barycenter(&mut right, &left, &adj);
        sort_by_barycenter(&mut left, &right, &adj);
    }

    // Per-row heights and widths so each column lays out independently.
    let widths: Vec<f64> = diagram
        .nodes
        .iter()
        .map(|n| match n.kind {
            NodeKind::Actor => ACTOR_WIDTH,
            NodeKind::UseCase => usecase_width(n),
        })
        .collect();
    let heights: Vec<f64> = diagram
        .nodes
        .iter()
        .map(|n| match n.kind {
            NodeKind::Actor => ACTOR_HEIGHT,
            NodeKind::UseCase => USECASE_H,
        })
        .collect();

    let left_max_w = left.iter().map(|&i| widths[i]).fold(0.0_f64, f64::max);
    let right_max_w = right.iter().map(|&i| widths[i]).fold(0.0_f64, f64::max);
    let left_col_x = SIDE_MARGIN + left_max_w / 2.0;
    let right_col_x = left_col_x + left_max_w / 2.0 + COLUMN_GAP + right_max_w / 2.0;

    let top_y = TOP_MARGIN + title_off;

    // Stack each column with row gaps. The two columns advance independently,
    // so a column with fewer rows ends earlier. Total height is the larger.
    let mut centres: Vec<(f64, f64)> = vec![(0.0, 0.0); diagram.nodes.len()];
    let mut left_y = top_y;
    for &i in &left {
        let cx = left_col_x;
        let cy = left_y + heights[i] / 2.0;
        centres[i] = (cx, cy);
        left_y += heights[i] + ROW_GAP;
    }
    let mut right_y = top_y;
    for &i in &right {
        let cx = right_col_x;
        let cy = right_y + heights[i] / 2.0;
        centres[i] = (cx, cy);
        right_y += heights[i] + ROW_GAP;
    }

    let total_height = left_y.max(right_y) - ROW_GAP + SIDE_MARGIN;
    let total_width = right_col_x + right_max_w / 2.0 + SIDE_MARGIN;

    let nodes_out: Vec<UseCaseLayoutNode> = diagram
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let (cx, cy) = centres[i];
            UseCaseLayoutNode {
                name: node.name.clone(),
                display: node.label.clone().unwrap_or_else(|| node.name.clone()),
                kind: node.kind.clone(),
                x: cx - widths[i] / 2.0,
                y: cy - heights[i] / 2.0,
                w: widths[i],
                h: heights[i],
                stereotype: node.stereotype.clone(),
            }
        })
        .collect();

    let edges_out: Vec<UseCaseLayoutEdge> = diagram
        .associations
        .iter()
        .map(|a| UseCaseLayoutEdge {
            from: a.from.clone(),
            to: a.to.clone(),
            label: a.label.clone(),
            dashed: matches!(a.kind, AssocKind::Dashed),
        })
        .collect();

    UseCaseLayout {
        nodes: nodes_out,
        edges: edges_out,
        title: diagram.title.clone(),
        total_width,
        total_height,
    }
}

fn usecase_width(case: &UseCaseNode) -> f64 {
    let text = case.label.as_deref().unwrap_or(&case.name);
    (text.len() as f64 * CHAR_W + 30.0).max(USECASE_MIN_W)
}

/// Sort `column` by the mean row index of each node's neighbours in `other`.
/// Nodes with no cross-column neighbours keep their current relative order
/// (stable sort with their existing index as the tie-break key).
fn sort_by_barycenter(column: &mut [usize], other: &[usize], adj: &[Vec<usize>]) {
    let other_pos: std::collections::HashMap<usize, f64> = other
        .iter()
        .enumerate()
        .map(|(pos, &idx)| (idx, pos as f64))
        .collect();
    let mut keyed: Vec<(usize, f64, usize)> = column
        .iter()
        .enumerate()
        .map(|(orig_pos, &node)| {
            let neigh: Vec<f64> = adj[node]
                .iter()
                .filter_map(|n| other_pos.get(n).copied())
                .collect();
            let bary = if neigh.is_empty() {
                orig_pos as f64
            } else {
                neigh.iter().sum::<f64>() / neigh.len() as f64
            };
            (node, bary, orig_pos)
        })
        .collect();
    keyed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.2.cmp(&b.2)));
    for (i, (node, _, _)) in keyed.into_iter().enumerate() {
        column[i] = node;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(name: &str, kind: NodeKind) -> UseCaseNode {
        UseCaseNode {
            name: name.to_string(),
            label: None,
            kind,
            stereotype: None,
        }
    }

    fn make_assoc(from: &str, to: &str) -> Association {
        Association {
            from: from.into(),
            to: to.into(),
            label: None,
            kind: AssocKind::Solid,
        }
    }

    #[test]
    fn empty_diagram_returns_empty_layout() {
        let d = UseCaseDiagram::default();
        let l = layout(&d);
        assert!(l.nodes.is_empty());
    }

    #[test]
    fn actors_land_left_of_use_cases() {
        let mut d = UseCaseDiagram::default();
        d.nodes.push(make_node("a", NodeKind::Actor));
        d.nodes.push(make_node("u", NodeKind::UseCase));
        d.associations.push(make_assoc("a", "u"));
        let l = layout(&d);
        let actor = l.nodes.iter().find(|n| n.name == "a").unwrap();
        let usecase = l.nodes.iter().find(|n| n.name == "u").unwrap();
        assert!(actor.x + actor.w <= usecase.x);
    }

    #[test]
    fn nodes_dont_overlap_after_layout() {
        let mut d = UseCaseDiagram::default();
        d.nodes.push(make_node("a1", NodeKind::Actor));
        d.nodes.push(make_node("a2", NodeKind::Actor));
        d.nodes.push(make_node("u1", NodeKind::UseCase));
        d.nodes.push(make_node("u2", NodeKind::UseCase));
        d.nodes.push(make_node("u3", NodeKind::UseCase));
        d.associations.push(make_assoc("a1", "u1"));
        d.associations.push(make_assoc("a1", "u2"));
        d.associations.push(make_assoc("a2", "u3"));
        let l = layout(&d);
        for i in 0..l.nodes.len() {
            for j in (i + 1)..l.nodes.len() {
                let a = &l.nodes[i];
                let b = &l.nodes[j];
                let x_overlap = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
                let y_overlap = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
                assert!(
                    x_overlap < 0.0 || y_overlap < 0.0,
                    "nodes {} and {} overlap",
                    a.name,
                    b.name
                );
            }
        }
    }

    #[test]
    fn barycentric_reorders_to_reduce_crossings() {
        // Worst-case ordering: a1→u3 and a2→u1 starting from document order
        // (a1, a2 left; u1, u2, u3 right) yields one crossing. After sweeps
        // the two edges should land non-crossing — verified by checking that
        // the higher actor connects to the higher use case.
        let mut d = UseCaseDiagram::default();
        d.nodes.push(make_node("a1", NodeKind::Actor));
        d.nodes.push(make_node("a2", NodeKind::Actor));
        d.nodes.push(make_node("u1", NodeKind::UseCase));
        d.nodes.push(make_node("u2", NodeKind::UseCase));
        d.nodes.push(make_node("u3", NodeKind::UseCase));
        d.associations.push(make_assoc("a1", "u3"));
        d.associations.push(make_assoc("a2", "u1"));
        let l = layout(&d);
        let y = |name: &str| l.nodes.iter().find(|n| n.name == name).unwrap().y;
        let (a1y, a2y, u1y, u3y) = (y("a1"), y("a2"), y("u1"), y("u3"));
        // No crossing iff the actor ordering matches the use-case ordering
        // for connected pairs. a1↔u3 and a2↔u1: if a1 above a2 then u3 above u1.
        let actors_inv = a1y < a2y;
        let usecases_inv = u3y < u1y;
        assert_eq!(
            actors_inv, usecases_inv,
            "edges cross: a1y={a1y} a2y={a2y} u1y={u1y} u3y={u3y}"
        );
    }
}
