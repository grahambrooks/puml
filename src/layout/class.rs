use std::collections::{HashMap, VecDeque};

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
const SAME_RANK_RAIL_GAP: f64 = 22.0;

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

/// A C4 boundary post-layout: a labeled rectangle covering the bounding
/// box of the contained nodes plus padding for the title and breathing room.
pub struct BoundaryBox {
    pub label: String,
    pub kind: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct ClassLayout {
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<EdgeLayout>,
    pub notes: Vec<NoteBox>,
    pub boundaries: Vec<BoundaryBox>,
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

use super::ports::pick_port;
use super::sugiyama::{
    assign_grid_columns, nudge_overlapping_segments, orthogonal_through_ports, reorder_barycentric,
};

const EDGE_NUDGE_GAP: f64 = 6.0;

/// Assign each node a rank (layer) using a topological sort on directed
/// edges (`Extension`, `Implementation`, `Dependency`, `DashedLink`,
/// `Realization`).
///
/// `Composition` and `Aggregation` are intentionally excluded: in UML the
/// container is the "owner" of its parts but doesn't sit at a rank above
/// or below them in the inheritance sense — letting them propagate ranks
/// would push containers into the inheritance hierarchy and stack
/// composition graphs into deep towers. `Association` is bidirectional and
/// has no natural direction to propagate.
///
/// Including `Dependency` is what makes component and deployment diagrams
/// (which use `-->` exclusively) lay out as a multi-row DAG instead of a
/// single horizontal row.
///
/// Endpoints may reference a node by either canonical `name` or `alias`,
/// so we resolve through `name_to_canonical` before reading/writing ranks.
fn assign_ranks(diagram: &ClassDiagram) -> HashMap<String, usize> {
    let source_to_target_dependencies = uses_source_to_target_dependency_ranks(diagram);

    // alias → canonical name lookup, so relations using the alias still
    // resolve to the same rank slot.
    let canonical = |s: &str| -> Option<String> {
        diagram
            .classes
            .iter()
            .find(|c| c.name == s || c.alias.as_deref() == Some(s))
            .map(|c| c.name.clone())
    };

    let name_to_idx: HashMap<&str, usize> = diagram
        .classes
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.as_str(), i))
        .collect();
    let mut edges: Vec<(usize, usize)> = Vec::new();

    // Propagate as parent → child.
    //
    // In C4 mode, Dependency/DashedLink edges flip direction: the relation
    // `Rel(a, b)` reads "a calls b" and the convention is caller-above-callee.
    // So for those edges we treat `to` as the rank-child of `from`. Inheritance
    // and realization (Extension/Implementation/Realization) stay UML-oriented
    // — they're rare in C4 anyway.
    for rel in &diagram.relations {
        let propagates = matches!(
            rel.kind,
            RelationKind::Extension
                | RelationKind::Implementation
                | RelationKind::Dependency
                | RelationKind::DashedLink
                | RelationKind::Realization
        );
        if !propagates {
            continue;
        }
        let (Some(a), Some(b)) = (canonical(rel.from.as_str()), canonical(rel.to.as_str())) else {
            continue;
        };
        let dependency = matches!(
            rel.kind,
            RelationKind::Dependency | RelationKind::DashedLink
        );
        let (parent_name, child_name) = if source_to_target_dependencies && dependency {
            (a, b)
        } else {
            (b, a)
        };
        let (Some(&parent), Some(&child)) = (
            name_to_idx.get(parent_name.as_str()),
            name_to_idx.get(child_name.as_str()),
        ) else {
            continue;
        };
        if parent != child {
            edges.push((parent, child));
        }
    }

    let dag_edges = break_ranking_cycles(diagram.classes.len(), &edges);
    let ranks = longest_path_ranks(diagram.classes.len(), &dag_edges);
    diagram
        .classes
        .iter()
        .enumerate()
        .map(|(i, c)| (c.name.clone(), ranks[i]))
        .collect()
}

fn uses_source_to_target_dependency_ranks(diagram: &ClassDiagram) -> bool {
    diagram.c4_mode
        || diagram.classes.iter().any(|c| {
            matches!(
                c.kind,
                ClassKind::Node
                    | ClassKind::Cloud
                    | ClassKind::Database
                    | ClassKind::Folder
                    | ClassKind::Frame
                    | ClassKind::Artifact
                    | ClassKind::Queue
            )
        })
}

fn break_ranking_cycles(n: usize, edges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    fn reaches(v: usize, target: usize, graph: &[Vec<usize>], seen: &mut [bool]) -> bool {
        if v == target {
            return true;
        }
        if seen[v] {
            return false;
        }
        seen[v] = true;
        for &to in &graph[v] {
            if reaches(to, target, graph, seen) {
                return true;
            }
        }
        false
    }

    let mut graph = vec![Vec::new(); n];
    let mut kept = Vec::new();
    for &(parent, child) in edges {
        let mut seen = vec![false; n];
        if reaches(child, parent, &graph, &mut seen) {
            continue;
        }
        graph[parent].push(child);
        kept.push((parent, child));
    }
    kept.sort_unstable();
    kept.dedup();
    kept
}

fn longest_path_ranks(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut outgoing = vec![Vec::new(); n];
    let mut indegree = vec![0_usize; n];
    for &(a, b) in edges {
        outgoing[a].push(b);
        indegree[b] += 1;
    }

    let mut ranks = vec![0_usize; n];
    let mut queue: VecDeque<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(i, &d)| (d == 0).then_some(i))
        .collect();
    while let Some(v) = queue.pop_front() {
        for &to in &outgoing[v] {
            ranks[to] = ranks[to].max(ranks[v] + 1);
            indegree[to] -= 1;
            if indegree[to] == 0 {
                queue.push_back(to);
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
    let source_to_target_dependencies = uses_source_to_target_dependency_ranks(diagram);
    let max_rank = ranks.values().copied().max().unwrap_or(0);

    // Group nodes by rank, carrying their *index* into `diagram.classes`
    // rather than a borrow — the barycentric reorder below works on
    // indices so it can return new per-layer orderings without fighting
    // the borrow checker.
    let real_n = diagram.classes.len();
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];
    for (i, node) in diagram.classes.iter().enumerate() {
        let r = ranks.get(&node.name).copied().unwrap_or(0);
        // Rank directly drives layer: parent at rank 0 → layer 0 (top of
        // canvas), child at rank N → layer N (below). This is the standard
        // UML convention — parents/supertypes/interfaces sit above their
        // children, with `--|>` arrows pointing up from child to parent.
        layers[r].push(i);
    }

    // Map both canonical names and aliases to node index, so relations
    // like `Foo --> alias_of_bar` resolve to the existing Bar node instead
    // of being treated as a phantom new class.
    let mut name_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, c) in diagram.classes.iter().enumerate() {
        name_to_idx.insert(c.name.as_str(), i);
        if let Some(ref alias) = c.alias {
            name_to_idx.insert(alias.as_str(), i);
        }
    }

    // Insert virtual (dummy) nodes for any relation whose endpoints sit
    // more than one layer apart, so barycentric reordering and column
    // assignment can route the long edge through stable intermediate
    // anchors. Without this, a grandparent → grandchild edge would draw
    // as one long Z that ignores intervening nodes; with virtuals it
    // becomes a stair-step that respects layer rhythm and avoids running
    // through unrelated boxes.
    //
    // For each relation we keep a `chain` of node ids (real + virtual)
    // describing the path the rendered edge will follow. Direct edges
    // get a chain of two; multi-rank edges get one virtual per skipped
    // layer.
    let mut layer_of: Vec<usize> = vec![0; real_n];
    for (li, layer) in layers.iter().enumerate() {
        for &idx in layer {
            layer_of[idx] = li;
        }
    }

    let mut chains: Vec<Vec<usize>> = Vec::with_capacity(diagram.relations.len());
    let mut next_virtual = real_n;
    let mut edge_pairs: Vec<(usize, usize)> = Vec::new();

    for rel in &diagram.relations {
        let (Some(&a), Some(&b)) = (
            name_to_idx.get(rel.from.as_str()),
            name_to_idx.get(rel.to.as_str()),
        ) else {
            chains.push(Vec::new());
            continue;
        };
        let la = layer_of[a];
        let lb = layer_of[b];
        let diff = la.abs_diff(lb);
        if diff <= 1 {
            chains.push(vec![a, b]);
            edge_pairs.push((a, b));
        } else {
            let direction: i32 = if lb > la { 1 } else { -1 };
            let mut chain = vec![a];
            for step in 1..diff {
                let v = next_virtual;
                next_virtual += 1;
                let v_layer = (la as i32 + direction * step as i32) as usize;
                layers[v_layer].push(v);
                chain.push(v);
            }
            chain.push(b);
            for w in chain.windows(2) {
                edge_pairs.push((w[0], w[1]));
            }
            chains.push(chain);
        }
    }

    // Reduce edge crossings on the augmented graph (virtual nodes
    // included). 6 sweeps is enough to converge for the diagrams we
    // care about; any more is diminishing.
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

    // Integer column index per node (real + virtual) via median-parent
    // placement on the augmented graph.
    let columns = assign_grid_columns(&layers, &edge_pairs);
    let col_step = uniform_width + NODE_H_GAP;

    // Per-layer y for both real and virtual nodes.
    let mut layer_y: Vec<f64> = Vec::with_capacity(layers.len());
    let mut y_cursor = TOP_MARGIN + title_off;
    for layer in &layers {
        layer_y.push(y_cursor);
        if !layer.is_empty() {
            y_cursor += uniform_height + RANK_V_GAP;
        }
    }
    let y = y_cursor;

    // Refresh layer_of after reorder, including virtuals. Indexed by node
    // id (could exceed real_n), so use a HashMap.
    let mut layer_of_all: HashMap<usize, usize> = HashMap::new();
    for (li, layer) in layers.iter().enumerate() {
        for &idx in layer {
            layer_of_all.insert(idx, li);
        }
    }

    let mut name_to_layout: HashMap<String, NodeLayout> = HashMap::new();
    for (class_idx, node) in diagram.classes.iter().enumerate() {
        let li = layer_of_all.get(&class_idx).copied().unwrap_or(0);
        let col = columns.get(&class_idx).copied().unwrap_or(0);
        let x = SIDE_MARGIN + col as f64 * col_step;
        let member_sections = build_member_sections(node);
        name_to_layout.insert(
            node.name.clone(),
            NodeLayout {
                name: node.name.clone(),
                display_name: display_name(node),
                x,
                y: layer_y[li],
                width: uniform_width,
                height: uniform_height,
                kind: node.kind.clone(),
                stereotype: node.stereotype.clone(),
                header_h: HEADER_HEIGHT,
                member_sections,
            },
        );
    }

    // Canvas: span all occupied grid columns + side margins.
    let max_col = columns.values().copied().max().unwrap_or(0);
    let grid_width = uniform_width + max_col as f64 * col_step;
    let mut total_width = grid_width + SIDE_MARGIN * 2.0;
    let mut total_height = y + SIDE_MARGIN;

    // Centre the graph horizontally inside the canvas.
    let (min_x, max_right) = name_to_layout.values().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min_x, max_right), nl| (min_x.min(nl.x), max_right.max(nl.x + nl.width)),
    );
    let centre_shift = if min_x.is_finite() {
        let content_w = max_right - min_x;
        let target_left = (total_width - content_w) / 2.0;
        let shift = target_left - min_x;
        for nl in name_to_layout.values_mut() {
            nl.x += shift;
        }
        shift
    } else {
        0.0
    };

    // Per-virtual centre coordinates, in the same coordinate space as the
    // real nodes (so chain routing can mix them seamlessly).
    let mut virtual_centre: HashMap<usize, (f64, f64)> = HashMap::new();
    for v in real_n..next_virtual {
        let li = layer_of_all.get(&v).copied().unwrap_or(0);
        let col = columns.get(&v).copied().unwrap_or(0);
        let cx = SIDE_MARGIN + col as f64 * col_step + uniform_width / 2.0 + centre_shift;
        let cy = layer_y[li] + uniform_height / 2.0;
        virtual_centre.insert(v, (cx, cy));
    }

    // Build edges. Direct (≤1 layer) edges still go through the orthogonal
    // router for box-edge attachment. Multi-rank edges follow their
    // virtual chain so the routing threads through the columns chosen by
    // barycentric reordering.
    let mut edges: Vec<EdgeLayout> = diagram
        .relations
        .iter()
        .zip(chains.iter())
        .filter_map(|(rel, chain)| {
            if chain.is_empty() {
                return None;
            }
            // Resolve through the chain's first/last node-id back to the
            // canonical name — `rel.from`/`rel.to` may be aliases that
            // wouldn't hit `name_to_layout` directly.
            let from_idx = *chain.first()?;
            let to_idx = *chain.last()?;
            if from_idx >= real_n || to_idx >= real_n {
                return None;
            }
            let from_nl = name_to_layout.get(&diagram.classes[from_idx].name)?;
            let to_nl = name_to_layout.get(&diagram.classes[to_idx].name)?;

            let points = if chain.len() == 2 {
                // Port-aware routing: pick the side of each box that faces
                // the other node. Cross-rank edges (parent → child) force
                // top/bottom attachment so the visual hierarchy reads
                // correctly even when the diagonal between centres is
                // closer to 45° than to vertical; same-rank edges
                // (sibling associations) fall back to whichever axis
                // dominates so they exit out the side facing the partner.
                let a = chain[0];
                let b = chain[1];
                let cross_rank = layer_of_all.get(&a) != layer_of_all.get(&b);
                if source_to_target_dependencies && cross_rank && to_nl.y < from_nl.y {
                    route_upward_around(from_nl, to_nl)
                } else if !cross_rank && same_rank_obstacle(from_nl, to_nl, &name_to_layout) {
                    route_same_rank_around(from_nl, to_nl)
                } else {
                    let from_bbox = (from_nl.x, from_nl.y, from_nl.width, from_nl.height);
                    let to_bbox = (to_nl.x, to_nl.y, to_nl.width, to_nl.height);
                    let from_centre = (
                        from_nl.x + from_nl.width / 2.0,
                        from_nl.y + from_nl.height / 2.0,
                    );
                    let to_centre = (to_nl.x + to_nl.width / 2.0, to_nl.y + to_nl.height / 2.0);
                    let (src, src_side) = pick_port(from_bbox, to_centre, cross_rank);
                    let (dst, dst_side) = pick_port(to_bbox, from_centre, cross_rank);
                    orthogonal_through_ports(src, src_side, dst, dst_side)
                }
            } else {
                route_through_virtuals(from_nl, to_nl, &chain[1..chain.len() - 1], &virtual_centre)
            };

            Some(EdgeLayout {
                points,
                kind: rel.kind.clone(),
                label: rel.label.clone(),
                from_label: rel.from_label.clone(),
                to_label: rel.to_label.clone(),
            })
        })
        .collect();

    // Spread overlapping middle rails apart so parallel edges between the
    // same pair of layers (e.g. two children of one parent, or shared
    // grandparent links) don't draw on top of each other.
    let mut routes: Vec<Vec<(f64, f64)>> = edges.iter().map(|e| e.points.clone()).collect();
    nudge_overlapping_segments(&mut routes, EDGE_NUDGE_GAP);
    for (e, r) in edges.iter_mut().zip(routes) {
        e.points = r;
    }

    // Preserve declaration order from the diagram for deterministic SVG output
    let mut nodes: Vec<NodeLayout> = diagram
        .classes
        .iter()
        .filter_map(|c| name_to_layout.remove(&c.name))
        .collect();

    let mut notes = place_notes(&diagram.notes, &nodes, &mut total_width);
    let mut boundaries = compute_boundaries(&diagram.boundaries, &nodes);

    // Boundaries pad outward by SIDE_PAD + nest depth, so deeply nested
    // outer boundaries can extend left past x=0 (or above the title). Shift
    // the whole layout right/down to keep everything inside the canvas.
    let min_b_x = boundaries.iter().map(|b| b.x).fold(f64::INFINITY, f64::min);
    if min_b_x.is_finite() && min_b_x < SIDE_MARGIN {
        let dx = SIDE_MARGIN - min_b_x;
        for n in &mut nodes {
            n.x += dx;
        }
        for e in &mut edges {
            for p in &mut e.points {
                p.0 += dx;
            }
        }
        for b in &mut boundaries {
            b.x += dx;
        }
    }

    let (edge_min_x, _) = edge_horizontal_bounds(&edges);
    if edge_min_x.is_finite() && edge_min_x < SIDE_MARGIN {
        let dx = SIDE_MARGIN - edge_min_x;
        shift_layout_x(&mut nodes, &mut edges, &mut boundaries, &mut notes, dx);
    }

    // Expand the canvas if any boundary or shifted node extends past the
    // node grid.
    for b in &boundaries {
        total_width = total_width.max(b.x + b.width + SIDE_MARGIN);
        total_height = total_height.max(b.y + b.height + SIDE_MARGIN);
    }
    for n in &nodes {
        total_width = total_width.max(n.x + n.width + SIDE_MARGIN);
    }
    for note in &notes {
        total_width = total_width.max(note.x + note.width + SIDE_MARGIN);
    }
    let (_, edge_max_x) = edge_horizontal_bounds(&edges);
    if edge_max_x.is_finite() {
        total_width = total_width.max(edge_max_x + SIDE_MARGIN);
    }

    ClassLayout {
        nodes,
        edges,
        notes,
        boundaries,
        total_width,
        total_height,
        title: diagram.title.clone(),
    }
}

/// For each AST boundary, compute the bounding box of its member nodes
/// plus extra padding scaled by *nesting depth* so an outer boundary lands
/// strictly outside any inner one it contains.
///
/// Nesting is detected by subset: boundary `B` is inside `A` iff every
/// member of `B` is also a member of `A`. This works for the C4 shape
/// where nested `Deployment_Node` blocks accumulate ancestor membership in
/// the translator. Boundaries with no resolved members produce no box.
fn compute_boundaries(
    declared: &[crate::ast::class::Boundary],
    nodes: &[NodeLayout],
) -> Vec<BoundaryBox> {
    const TITLE_PAD: f64 = 28.0;
    const SIDE_PAD: f64 = 14.0;
    const BOTTOM_PAD: f64 = 14.0;
    const NEST_STEP: f64 = 22.0;

    // Resolve each boundary to its concrete member set first, dropping any
    // member name that doesn't match a real node.
    let resolved: Vec<Vec<&NodeLayout>> = declared
        .iter()
        .map(|b| {
            b.members
                .iter()
                .filter_map(|name| nodes.iter().find(|n| &n.name == name))
                .collect()
        })
        .collect();

    let mut out: Vec<BoundaryBox> = Vec::new();
    for (i, b) in declared.iter().enumerate() {
        let members = &resolved[i];
        if members.is_empty() {
            continue;
        }
        // Depth = number of other boundaries contained inside this one.
        // Strict-subset handles the obvious case (ec2 ⊂ aws). When two
        // boundaries share the same members (a Deployment_Node that wraps
        // exactly one inner Deployment_Node), declaration order breaks the
        // tie — the outer is declared first in the source, so anything
        // later with the same set is treated as nested under us.
        let mine: std::collections::HashSet<&str> =
            members.iter().map(|n| n.name.as_str()).collect();
        let depth = resolved
            .iter()
            .enumerate()
            .filter(|(j, other)| {
                if *j == i || other.is_empty() {
                    return false;
                }
                let theirs: std::collections::HashSet<&str> =
                    other.iter().map(|n| n.name.as_str()).collect();
                if !theirs.is_subset(&mine) {
                    return false;
                }
                theirs != mine || *j > i
            })
            .count();

        let min_x = members.iter().map(|n| n.x).fold(f64::INFINITY, f64::min);
        let min_y = members.iter().map(|n| n.y).fold(f64::INFINITY, f64::min);
        let max_x = members
            .iter()
            .map(|n| n.x + n.width)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = members
            .iter()
            .map(|n| n.y + n.height)
            .fold(f64::NEG_INFINITY, f64::max);

        let nest_pad = depth as f64 * NEST_STEP;
        // Make sure the boundary box is wide enough for its title. Without
        // this, deeply nested boundaries with long type names overflow the
        // dashed rectangle on the right side. Title char width is a rough
        // estimate matching the 12px boundary font.
        const TITLE_CHAR_W: f64 = 7.0;
        const TITLE_INSET: f64 = 12.0;
        let title_text_len = b.label.chars().count()
            + if b.kind.is_empty() {
                0
            } else {
                b.kind.chars().count() + " «boundary»".chars().count()
            };
        let title_min_w = title_text_len as f64 * TITLE_CHAR_W + TITLE_INSET * 2.0;

        let raw_width = (max_x - min_x) + (SIDE_PAD + nest_pad) * 2.0;
        let width = raw_width.max(title_min_w);
        out.push(BoundaryBox {
            label: b.label.clone(),
            kind: b.kind.clone(),
            x: min_x - SIDE_PAD - nest_pad,
            y: min_y - TITLE_PAD - nest_pad,
            width,
            height: (max_y - min_y) + TITLE_PAD + BOTTOM_PAD + nest_pad * 2.0,
        });
    }
    out
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

/// Route a multi-rank edge as a stair-step through its virtual nodes.
///
/// Source exits the bottom (or top) of its node; for each virtual centre
/// in the chain we drop to the midway y between its layer and the
/// previous, jog horizontally to the virtual's column, and continue.
/// Final segment enters the target's top (or bottom). This produces a
/// clean, column-aligned path that respects whatever order barycentric
/// reordering picked for the virtuals.
fn route_through_virtuals(
    from: &NodeLayout,
    to: &NodeLayout,
    virtuals: &[usize],
    virtual_centre: &HashMap<usize, (f64, f64)>,
) -> Vec<(f64, f64)> {
    if virtuals.is_empty() {
        // Caller should have used orthogonal_route directly; fall through.
        return vec![
            (from.x + from.width / 2.0, from.y + from.height),
            (to.x + to.width / 2.0, to.y),
        ];
    }

    let descending = to.y > from.y;
    let from_cx = from.x + from.width / 2.0;
    let to_cx = to.x + to.width / 2.0;
    let from_port_y = if descending {
        from.y + from.height
    } else {
        from.y
    };
    let to_port_y = if descending { to.y } else { to.y + to.height };

    // Collect every centre on the path: source port, each virtual, target
    // port. The y-coordinates already alternate in layer order.
    let mut anchors: Vec<(f64, f64)> = Vec::with_capacity(virtuals.len() + 2);
    anchors.push((from_cx, from_port_y));
    for &v in virtuals {
        if let Some(&(cx, cy)) = virtual_centre.get(&v) {
            anchors.push((cx, cy));
        }
    }
    anchors.push((to_cx, to_port_y));

    // Stair-step: between consecutive anchors a and b, drop to the midway
    // y, jog to b's x, continue. If a and b already share x, collapse to
    // a single straight segment.
    let mut points: Vec<(f64, f64)> = Vec::with_capacity(anchors.len() * 2);
    points.push(anchors[0]);
    for w in anchors.windows(2) {
        let (ax, ay) = w[0];
        let (bx, by) = w[1];
        if (ax - bx).abs() < 0.5 {
            points.push((bx, by));
            continue;
        }
        let mid_y = (ay + by) / 2.0;
        points.push((ax, mid_y));
        points.push((bx, mid_y));
        points.push((bx, by));
    }
    points
}

fn same_rank_obstacle(
    from: &NodeLayout,
    to: &NodeLayout,
    nodes: &HashMap<String, NodeLayout>,
) -> bool {
    if (from.y - to.y).abs() > 0.5 {
        return false;
    }

    let from_right = from.x + from.width;
    let to_right = to.x + to.width;
    let (span_left, span_right) = if from.x < to.x {
        (from_right, to.x)
    } else {
        (to_right, from.x)
    };
    if span_right <= span_left {
        return false;
    }

    nodes.values().any(|node| {
        node.name != from.name
            && node.name != to.name
            && (node.y - from.y).abs() <= 0.5
            && node.x < span_right
            && node.x + node.width > span_left
    })
}

fn route_same_rank_around(from: &NodeLayout, to: &NodeLayout) -> Vec<(f64, f64)> {
    let left_to_right = from.x < to.x;
    let y = from.y + from.height / 2.0;
    let (src_x, dst_x) = if left_to_right {
        (from.x + from.width, to.x)
    } else {
        (from.x, to.x + to.width)
    };
    let rail_y = if left_to_right {
        from.y - SAME_RANK_RAIL_GAP
    } else {
        from.y + from.height + SAME_RANK_RAIL_GAP
    };

    vec![(src_x, y), (src_x, rail_y), (dst_x, rail_y), (dst_x, y)]
}

fn route_upward_around(from: &NodeLayout, to: &NodeLayout) -> Vec<(f64, f64)> {
    let rail_x = (from.x + from.width).max(to.x + to.width) + NODE_H_GAP;
    let src = (from.x + from.width / 2.0, from.y);
    let dst = (to.x + to.width / 2.0, to.y + to.height);
    vec![src, (rail_x, src.1), (rail_x, dst.1), dst]
}

fn shift_layout_x(
    nodes: &mut [NodeLayout],
    edges: &mut [EdgeLayout],
    boundaries: &mut [BoundaryBox],
    notes: &mut [NoteBox],
    dx: f64,
) {
    for n in nodes {
        n.x += dx;
    }
    for e in edges {
        for p in &mut e.points {
            p.0 += dx;
        }
    }
    for b in boundaries {
        b.x += dx;
    }
    for note in notes {
        note.x += dx;
        note.target_x += dx;
    }
}

fn edge_horizontal_bounds(edges: &[EdgeLayout]) -> (f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    for edge in edges {
        for &(x, _) in &edge.points {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
        }
        if let Some(ref label) = edge.label {
            let (x, _, anchor) = edge_label_position(&edge.points, 8.0);
            let width = text_w(label);
            let (left, right) = match anchor {
                "end" => (x - width, x),
                "middle" => (x - width / 2.0, x + width / 2.0),
                _ => (x, x + width),
            };
            min_x = min_x.min(left);
            max_x = max_x.max(right);
        }
        if let (Some(label), Some(&(x, _))) = (&edge.from_label, edge.points.first()) {
            min_x = min_x.min(x + 6.0);
            max_x = max_x.max(x + 6.0 + text_w(label));
        }
        if let (Some(label), Some(&(x, _))) = (&edge.to_label, edge.points.last()) {
            min_x = min_x.min(x + 6.0);
            max_x = max_x.max(x + 6.0 + text_w(label));
        }
    }
    (min_x, max_x)
}

fn edge_label_position(points: &[(f64, f64)], gap: f64) -> (f64, f64, &'static str) {
    if points.len() < 2 {
        let (x, y) = points.first().copied().unwrap_or((0.0, 0.0));
        return (x + gap, y, "start");
    }
    let seg_lens: Vec<f64> = points
        .windows(2)
        .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
        .collect();
    let total: f64 = seg_lens.iter().sum();
    let half = total / 2.0;
    let mut travelled = 0.0;
    for (i, &len) in seg_lens.iter().enumerate() {
        if travelled + len < half {
            travelled += len;
            continue;
        }
        let t = if len > 0.0 {
            (half - travelled) / len
        } else {
            0.0
        };
        let (x0, y0) = points[i];
        let (x1, y1) = points[i + 1];
        let dx = x1 - x0;
        let dy = y1 - y0;
        let mag = (dx * dx + dy * dy).sqrt().max(0.0001);
        let (ux, uy) = (dx / mag, dy / mag);
        let (px, py) = (uy, -ux);
        let mx = x0 + dx * t;
        let my = y0 + dy * t;
        let lx = mx + px * gap;
        let dy_baseline = if py < -0.3 {
            0.0
        } else if py > 0.3 {
            11.0
        } else {
            4.0
        };
        let ly = my + py * gap + dy_baseline;
        let anchor = if px > 0.3 {
            "start"
        } else if px < -0.3 {
            "end"
        } else {
            "middle"
        };
        return (lx, ly, anchor);
    }
    let last = *points.last().unwrap();
    (last.0 + gap, last.1, "start")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(name: &str) -> ClassNode {
        ClassNode {
            name: name.to_string(),
            alias: None,
            generics: None,
            kind: ClassKind::Class,
            stereotype: None,
            color: None,
            members: Vec::new(),
            namespace: None,
        }
    }

    fn dependency(from: &str, to: &str) -> Relation {
        Relation {
            from: from.to_string(),
            to: to.to_string(),
            kind: RelationKind::Dependency,
            from_label: None,
            to_label: None,
            label: None,
            reversed: false,
        }
    }

    #[test]
    fn c4_dependency_cycles_keep_forward_ranks() {
        let diagram = ClassDiagram {
            classes: vec![
                class("customer"),
                class("spa"),
                class("sign_in"),
                class("security"),
                class("db"),
            ],
            relations: vec![
                dependency("customer", "spa"),
                dependency("spa", "sign_in"),
                dependency("sign_in", "security"),
                dependency("security", "db"),
                dependency("security", "sign_in"),
                dependency("sign_in", "spa"),
            ],
            c4_mode: true,
            ..ClassDiagram::default()
        };

        let ranks = assign_ranks(&diagram);
        assert_eq!(ranks["customer"], 0);
        assert_eq!(ranks["spa"], 1);
        assert_eq!(ranks["sign_in"], 2);
        assert_eq!(ranks["security"], 3);
        assert_eq!(ranks["db"], 4);
    }
}
