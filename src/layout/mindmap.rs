use crate::ast::mindmap::*;

const NODE_H: f64 = 32.0;
const NODE_V_GAP: f64 = 12.0;
const LEVEL_GAP: f64 = 50.0;
const NODE_PAD_X: f64 = 16.0;
const CHAR_W: f64 = 7.5;
const SIDE_MARGIN: f64 = 30.0;
const TOP_MARGIN: f64 = 30.0;
const TITLE_H: f64 = 30.0;

#[derive(Debug, Clone)]
pub struct MindLayoutNode {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub depth: usize,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MindLayoutEdge {
    pub from_x: f64,
    pub from_y: f64,
    pub to_x: f64,
    pub to_y: f64,
}

pub struct MindMapLayout {
    pub nodes: Vec<MindLayoutNode>,
    pub edges: Vec<MindLayoutEdge>,
    pub title: Option<String>,
    pub total_width: f64,
    pub total_height: f64,
}

/// Two-pass layout:
///   1. Partition children into left/right subtrees (auto alternates between
///      sides; `+`/`-` force placement).
///   2. Assign y positions so every leaf is stacked vertically with fixed
///      gaps, then parents centre over their subtree's y range.
pub fn layout(diagram: &MindMapDiagram) -> MindMapLayout {
    let title_off = if diagram.title.is_some() {
        TITLE_H
    } else {
        0.0
    };

    if diagram.nodes.is_empty() {
        return MindMapLayout {
            nodes: Vec::new(),
            edges: Vec::new(),
            title: diagram.title.clone(),
            total_width: 400.0,
            total_height: TOP_MARGIN + title_off + SIDE_MARGIN,
        };
    }

    // Build a tree of indices using the depth sequence.
    let n = diagram.nodes.len();
    let mut parent: Vec<Option<usize>> = vec![None; n];
    // stack holds (index, depth) for the ancestor chain under construction.
    let mut stack: Vec<usize> = Vec::new();
    for (i, node) in diagram.nodes.iter().enumerate() {
        while let Some(&top) = stack.last() {
            if diagram.nodes[top].depth >= node.depth {
                stack.pop();
            } else {
                break;
            }
        }
        if let Some(&p) = stack.last() {
            parent[i] = Some(p);
        }
        stack.push(i);
    }

    // Children list per node.
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, p) in parent.iter().enumerate() {
        if let Some(p) = *p {
            children[p].push(i);
        }
    }

    // Assign side per node. Root = centre. Children inherit parent side
    // unless they force otherwise; auto-direct children of root alternate.
    let mut side: Vec<Side> = vec![Side::Auto; n];
    // Root index is the first depth-1 node; any later depth-1 nodes are
    // treated as siblings of root (rare but tolerated).
    let root_idx = 0;
    side[root_idx] = Side::Auto;

    let direct_children_of_root = &children[root_idx];
    let mut auto_flip = true;
    for &c in direct_children_of_root {
        side[c] = match diagram.nodes[c].side {
            Side::Right => Side::Right,
            Side::Left => Side::Left,
            Side::Auto => {
                let s = if auto_flip { Side::Right } else { Side::Left };
                auto_flip = !auto_flip;
                s
            }
        };
    }
    // Propagate side down from each root child.
    fn propagate(side: &mut [Side], children: &[Vec<usize>], idx: usize, inherit: Side) {
        for &c in &children[idx] {
            let s = match side[c] {
                Side::Right | Side::Left => side[c].clone(),
                Side::Auto => inherit.clone(),
            };
            side[c] = s.clone();
            propagate(side, children, c, s);
        }
    }
    for &c in direct_children_of_root {
        let s = side[c].clone();
        propagate(&mut side, &children, c, s);
    }

    // Y-positions: DFS in source order, allocate a row per leaf on each side.
    let mut ys: Vec<f64> = vec![0.0; n];
    let mut cursor_right = TOP_MARGIN + title_off;
    let mut cursor_left = TOP_MARGIN + title_off;

    fn assign_y(
        idx: usize,
        children: &[Vec<usize>],
        side: &[Side],
        ys: &mut [f64],
        cursor_r: &mut f64,
        cursor_l: &mut f64,
    ) {
        if children[idx].is_empty() {
            match side[idx] {
                Side::Left => {
                    ys[idx] = *cursor_l;
                    *cursor_l += NODE_H + NODE_V_GAP;
                }
                _ => {
                    ys[idx] = *cursor_r;
                    *cursor_r += NODE_H + NODE_V_GAP;
                }
            }
            return;
        }
        for &c in &children[idx] {
            assign_y(c, children, side, ys, cursor_r, cursor_l);
        }
        // Parent centred over its own-side children only.
        let own_side = &side[idx];
        let kid_ys: Vec<f64> = children[idx]
            .iter()
            .filter(|&&c| {
                matches!(
                    (own_side, &side[c]),
                    (Side::Left, Side::Left) | (_, Side::Right) | (Side::Auto, _)
                )
            })
            .map(|&c| ys[c])
            .collect();
        if let (Some(&min), Some(&max)) = (
            kid_ys.iter().min_by(|a, b| a.partial_cmp(b).unwrap()),
            kid_ys.iter().max_by(|a, b| a.partial_cmp(b).unwrap()),
        ) {
            ys[idx] = (min + max) / 2.0;
        } else if !children[idx].is_empty() {
            let c0 = children[idx][0];
            ys[idx] = ys[c0];
        }
    }
    // Root y is the midpoint between its left and right subtree spans.
    for &c in direct_children_of_root {
        assign_y(
            c,
            &children,
            &side,
            &mut ys,
            &mut cursor_right,
            &mut cursor_left,
        );
    }
    let max_y = cursor_right.max(cursor_left);
    ys[root_idx] = (TOP_MARGIN + title_off + max_y) / 2.0;

    // Widths based on label length.
    let widths: Vec<f64> = diagram
        .nodes
        .iter()
        .map(|n| (n.label.len() as f64 * CHAR_W + NODE_PAD_X * 2.0).max(80.0))
        .collect();

    // X-positions: root centre derived later; children offset by depth.
    let mut xs: Vec<f64> = vec![0.0; n];
    let root_w = widths[root_idx];
    // Deepest node on each side determines canvas width.
    let max_depth_right = (0..n)
        .filter(|&i| matches!(side[i], Side::Right) || i == root_idx)
        .map(|i| diagram.nodes[i].depth)
        .max()
        .unwrap_or(1);
    let max_depth_left = (0..n)
        .filter(|&i| matches!(side[i], Side::Left))
        .map(|i| diagram.nodes[i].depth)
        .max()
        .unwrap_or(0);

    let _ = max_depth_right; // reserved for future use in canvas-width calc
    let left_width: f64 = (1..=max_depth_left)
        .map(|d| {
            (0..n)
                .filter(|&i| diagram.nodes[i].depth == d && matches!(side[i], Side::Left))
                .map(|i| widths[i])
                .fold(0.0_f64, f64::max)
        })
        .sum::<f64>()
        + LEVEL_GAP * max_depth_left.saturating_sub(1) as f64;

    let root_cx = SIDE_MARGIN + left_width + root_w / 2.0 + LEVEL_GAP;
    xs[root_idx] = root_cx - widths[root_idx] / 2.0;

    // Place each node by walking depth layers on both sides.
    fn place(
        idx: usize,
        children: &[Vec<usize>],
        side: &[Side],
        widths: &[f64],
        xs: &mut [f64],
        parent_cx: f64,
    ) {
        for &c in &children[idx] {
            let w = widths[c];
            let cx = match side[c] {
                Side::Left => parent_cx - LEVEL_GAP - w / 2.0 - widths[idx] / 2.0,
                _ => parent_cx + LEVEL_GAP + w / 2.0 + widths[idx] / 2.0,
            };
            xs[c] = cx - w / 2.0;
            place(c, children, side, widths, xs, cx);
        }
    }
    place(root_idx, &children, &side, &widths, &mut xs, root_cx);

    let mut out_nodes: Vec<MindLayoutNode> = Vec::with_capacity(n);
    for (i, node) in diagram.nodes.iter().enumerate() {
        out_nodes.push(MindLayoutNode {
            label: node.label.clone(),
            x: xs[i],
            y: ys[i],
            w: widths[i],
            h: NODE_H,
            depth: node.depth,
            color: node.color.clone(),
        });
    }

    let mut edges: Vec<MindLayoutEdge> = Vec::new();
    for i in 0..n {
        if let Some(p) = parent[i] {
            let from = &out_nodes[p];
            let to = &out_nodes[i];
            // Connect from the side of `from` facing `to`.
            let (fx, tx) = if to.x > from.x {
                (from.x + from.w, to.x)
            } else {
                (from.x, to.x + to.w)
            };
            edges.push(MindLayoutEdge {
                from_x: fx,
                from_y: from.y + from.h / 2.0,
                to_x: tx,
                to_y: to.y + to.h / 2.0,
            });
        }
    }

    let total_width = out_nodes.iter().map(|n| n.x + n.w).fold(0.0_f64, f64::max) + SIDE_MARGIN;
    let total_height = out_nodes.iter().map(|n| n.y + n.h).fold(0.0_f64, f64::max) + SIDE_MARGIN;

    MindMapLayout {
        nodes: out_nodes,
        edges,
        title: diagram.title.clone(),
        total_width,
        total_height,
    }
}
