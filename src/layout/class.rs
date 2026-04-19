use std::collections::HashMap;

use crate::ast::class::*;

const CLASS_MIN_WIDTH: f64 = 140.0;
const CLASS_H_PAD: f64 = 12.0;
const CLASS_V_PAD: f64 = 6.0;
const HEADER_HEIGHT: f64 = 36.0;
const MEMBER_HEIGHT: f64 = 20.0;
const FONT_CHAR_W: f64 = 7.5;
const RANK_V_GAP: f64 = 80.0;
const NODE_H_GAP: f64 = 40.0;
const SIDE_MARGIN: f64 = 30.0;
const TOP_MARGIN: f64 = 30.0;
const TITLE_HEIGHT: f64 = 30.0;

pub struct NodeLayout {
    pub name: String,
    pub display_name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub kind: ClassKind,
    pub stereotype: Option<String>,
    pub header_h: f64,
    pub member_sections: Vec<MemberSection>,
}

pub struct NoteBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub lines: Vec<String>,
    pub target_x: f64,
    pub target_y: f64,
}

pub struct MemberSection {
    pub separator: bool,
    pub members: Vec<RenderedMember>,
}

pub struct RenderedMember {
    pub text: String,
    pub is_static: bool,
    pub is_abstract: bool,
}

pub struct EdgeLayout {
    /// Polyline waypoints, first point on the source boundary, last on the
    /// target boundary. Orthogonal (horizontal/vertical only) in the
    /// common case; may be a two-point straight line when endpoints align.
    pub points: Vec<(f64, f64)>,
    pub kind: RelationKind,
    pub label: Option<String>,
    pub from_label: Option<String>,
    pub to_label: Option<String>,
}

pub struct ClassLayout {
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub notes: Vec<NoteBox>,
    pub total_width: f64,
    pub total_height: f64,
    pub title: Option<String>,
}

fn text_w(s: &str) -> f64 {
    s.len() as f64 * FONT_CHAR_W
}

fn display_name(node: &ClassNode) -> String {
    match &node.generics {
        Some(g) => format!("{}<{}>", node.name, g),
        None => node.name.clone(),
    }
}

fn node_width(node: &ClassNode) -> f64 {
    let name_w = text_w(&display_name(node)) + CLASS_H_PAD * 2.0;
    let member_w = node
        .members
        .iter()
        .map(|m| text_w(&member_text(m)) + CLASS_H_PAD * 2.0)
        .fold(0.0_f64, f64::max);
    name_w.max(member_w).max(CLASS_MIN_WIDTH)
}

fn node_height(node: &ClassNode, hide_empty: bool) -> f64 {
    if node.members.is_empty() && hide_empty {
        return HEADER_HEIGHT;
    }
    let member_count = node.members.len();
    HEADER_HEIGHT + (member_count as f64) * MEMBER_HEIGHT + CLASS_V_PAD
}

fn member_text(m: &Member) -> String {
    let vis = match m.visibility {
        Visibility::Public => "+",
        Visibility::Private => "-",
        Visibility::Protected => "#",
        Visibility::Package => "~",
        Visibility::None => "",
    };
    let params = m.params.as_deref().unwrap_or("");
    let type_part = m
        .type_annotation
        .as_ref()
        .map(|t| format!(": {}", t))
        .unwrap_or_default();
    if m.is_method {
        format!("{}{}{}{}", vis, m.name, params, type_part)
    } else {
        format!("{}{}{}", vis, m.name, type_part)
    }
}

use super::sugiyama::{assign_grid_columns, orthogonal_route, reorder_barycentric};

/// Assign each node a rank (layer) using a topological sort on Extension/Implementation edges.
/// All other nodes default to rank 0.
fn assign_ranks(diagram: &ClassDiagram) -> HashMap<String, usize> {
    let mut ranks: HashMap<String, usize> = diagram
        .classes
        .iter()
        .map(|c| (c.name.clone(), 0))
        .collect();
    // Repeatedly propagate: child rank = parent rank + 1
    for _ in 0..diagram.classes.len() {
        for rel in &diagram.relations {
            if matches!(
                rel.kind,
                RelationKind::Extension | RelationKind::Implementation
            ) {
                let parent_rank = ranks.get(&rel.to).copied().unwrap_or(0);
                let child_rank = ranks.entry(rel.from.clone()).or_insert(0);
                if *child_rank <= parent_rank {
                    *child_rank = parent_rank + 1;
                }
            }
        }
    }
    ranks
}

pub fn layout(diagram: &ClassDiagram) -> ClassLayout {
    let title_off = if diagram.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let ranks = assign_ranks(diagram);
    let max_rank = ranks.values().copied().max().unwrap_or(0);

    // Group nodes by rank, carrying their *index* into `diagram.classes`
    // rather than a borrow — the barycentric reorder below works on
    // indices so it can return new per-layer orderings without fighting
    // the borrow checker.
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];
    for (i, node) in diagram.classes.iter().enumerate() {
        let r = ranks.get(&node.name).copied().unwrap_or(0);
        // Invert: layer 0 (top of canvas) = max rank so children draw
        // above their parents. Matches UML convention with `--|>` arrows
        // pointing up from child to parent.
        let layer = max_rank.saturating_sub(r);
        layers[layer].push(i);
    }

    // Build an edge list over node indices — ALL relations participate,
    // not just inheritance, so composition / aggregation / association
    // also influence within-rank ordering.
    let name_to_idx: HashMap<&str, usize> = diagram
        .classes
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.as_str(), i))
        .collect();
    let edge_pairs: Vec<(usize, usize)> = diagram
        .relations
        .iter()
        .filter_map(|r| {
            let a = name_to_idx.get(r.from.as_str()).copied()?;
            let b = name_to_idx.get(r.to.as_str()).copied()?;
            Some((a, b))
        })
        .collect();

    // Reduce edge crossings before x-assignment. 6 sweeps is enough to
    // converge for the diagrams we care about; any more is diminishing.
    reorder_barycentric(&mut layers, &edge_pairs, 6);

    // Uniform sizing: every class box gets the dimensions of the largest
    // one. Edges between grid-aligned parent/child collapse to straight
    // vertical lines (no jogs), and the visual rhythm across the diagram is
    // uniform. Cost: a little extra whitespace in small classes; worth it
    // for the overall readability and consistent with the outline-first
    // aesthetic.
    let uniform_width = diagram
        .classes
        .iter()
        .map(node_width)
        .fold(CLASS_MIN_WIDTH, f64::max);
    let uniform_height = diagram
        .classes
        .iter()
        .map(|c| node_height(c, diagram.hide_empty_members))
        .fold(HEADER_HEIGHT, f64::max);

    // Integer column index per class via median-parent placement.
    let columns = assign_grid_columns(&layers, &edge_pairs);
    let col_step = uniform_width + NODE_H_GAP;

    let mut name_to_layout: HashMap<String, NodeLayout> = HashMap::new();
    let mut y = TOP_MARGIN + title_off;

    for layer in &layers {
        if layer.is_empty() {
            continue;
        }
        for &class_idx in layer {
            let node = &diagram.classes[class_idx];
            let col = columns.get(&class_idx).copied().unwrap_or(0);
            let x = SIDE_MARGIN + col as f64 * col_step;

            let member_sections = build_member_sections(node);

            name_to_layout.insert(
                node.name.clone(),
                NodeLayout {
                    name: node.name.clone(),
                    display_name: display_name(node),
                    x,
                    y,
                    width: uniform_width,
                    height: uniform_height,
                    kind: node.kind.clone(),
                    stereotype: node.stereotype.clone(),
                    header_h: HEADER_HEIGHT,
                    member_sections,
                },
            );
        }
        y += uniform_height + RANK_V_GAP;
    }

    // Canvas: span all occupied grid columns + side margins.
    let max_col = columns.values().copied().max().unwrap_or(0);
    let grid_width = uniform_width + max_col as f64 * col_step;
    let mut total_width = grid_width + SIDE_MARGIN * 2.0;
    let total_height = y + SIDE_MARGIN;

    // Centre the graph horizontally inside the canvas.
    let (min_x, max_right) = name_to_layout.values().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min_x, max_right), nl| (min_x.min(nl.x), max_right.max(nl.x + nl.width)),
    );
    if min_x.is_finite() {
        let content_w = max_right - min_x;
        let target_left = (total_width - content_w) / 2.0;
        let shift = target_left - min_x;
        for nl in name_to_layout.values_mut() {
            nl.x += shift;
        }
    }

    // Build edges — orthogonal polylines via the sugiyama router. For
    // parent-child chains where nodes share an x-centre the router
    // collapses to a straight vertical line; when they don't it emits a
    // two-bend elbow (out of the source, across, into the target).
    let edges: Vec<EdgeLayout> = diagram
        .relations
        .iter()
        .filter_map(|rel| {
            let from_nl = name_to_layout.get(&rel.from)?;
            let to_nl = name_to_layout.get(&rel.to)?;

            let from_box = (from_nl.x, from_nl.y, from_nl.width, from_nl.height);
            let to_box = (to_nl.x, to_nl.y, to_nl.width, to_nl.height);
            let points = orthogonal_route(from_box, to_box);

            Some(EdgeLayout {
                points,
                kind: rel.kind.clone(),
                label: rel.label.clone(),
                from_label: rel.from_label.clone(),
                to_label: rel.to_label.clone(),
            })
        })
        .collect();

    // Preserve declaration order from the diagram for deterministic SVG output
    let nodes: Vec<NodeLayout> = diagram
        .classes
        .iter()
        .filter_map(|c| name_to_layout.remove(&c.name))
        .collect();

    let notes = place_notes(&diagram.notes, &nodes, &mut total_width);

    ClassLayout {
        nodes,
        edges,
        notes,
        total_width,
        total_height,
        title: diagram.title.clone(),
    }
}

fn place_notes(notes: &[ClassNote], nodes: &[NodeLayout], total_width: &mut f64) -> Vec<NoteBox> {
    const NOTE_PAD: f64 = 12.0;
    const NOTE_LINE_H: f64 = 16.0;
    const NOTE_GAP: f64 = 20.0;

    let mut out: Vec<NoteBox> = Vec::new();
    for note in notes {
        let Some(target) = nodes.iter().find(|n| n.name == note.target) else {
            continue;
        };
        let width = note
            .lines
            .iter()
            .map(|l| l.len() as f64 * FONT_CHAR_W)
            .fold(80.0_f64, f64::max)
            + NOTE_PAD * 2.0;
        let height = (note.lines.len().max(1) as f64) * NOTE_LINE_H + NOTE_PAD;
        let (x, y, tx, ty) = match note.position {
            NotePosition::Left => (
                target.x - width - NOTE_GAP,
                target.y + target.height / 2.0 - height / 2.0,
                target.x,
                target.y + target.height / 2.0,
            ),
            NotePosition::Right => (
                target.x + target.width + NOTE_GAP,
                target.y + target.height / 2.0 - height / 2.0,
                target.x + target.width,
                target.y + target.height / 2.0,
            ),
            NotePosition::Top => (
                target.x + target.width / 2.0 - width / 2.0,
                target.y - height - NOTE_GAP,
                target.x + target.width / 2.0,
                target.y,
            ),
            NotePosition::Bottom => (
                target.x + target.width / 2.0 - width / 2.0,
                target.y + target.height + NOTE_GAP,
                target.x + target.width / 2.0,
                target.y + target.height,
            ),
        };
        *total_width = total_width.max(x + width + SIDE_MARGIN);
        out.push(NoteBox {
            x,
            y,
            width,
            height,
            lines: note.lines.clone(),
            target_x: tx,
            target_y: ty,
        });
    }
    out
}

fn build_member_sections(node: &ClassNode) -> Vec<MemberSection> {
    if node.members.is_empty() {
        return vec![MemberSection {
            separator: false,
            members: Vec::new(),
        }];
    }
    let members: Vec<RenderedMember> = node
        .members
        .iter()
        .map(|m| RenderedMember {
            text: member_text(m),
            is_static: m.is_static,
            is_abstract: m.is_abstract,
        })
        .collect();
    vec![MemberSection {
        separator: false,
        members,
    }]
}

// `connect_nodes` previously chose an attachment pair between two nodes;
// the orthogonal router in `sugiyama::orthogonal_route` now owns that
// decision and the bend-point generation, so the old helper is gone.
