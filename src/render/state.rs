use svg::node::element::{Circle, Definitions, Group, Marker, Path, Polygon, Rectangle, Text};
use svg::Document;

use super::primitives::{background_rect, label_perpendicular, style_block, text_node};
use super::theme::Theme;
use crate::ast::state::StateKind;
use crate::layout::ports::pick_port;
use crate::layout::state::{StateLayout, StateLayoutEdge, StateLayoutNode};
use crate::layout::sugiyama::{nudge_overlapping_segments, orthogonal_through_ports};

const EDGE_NUDGE_GAP: f64 = 6.0;

const FONT_SIZE: f64 = 13.0;
const TOP_MARGIN: f64 = 20.0;
const TITLE_HEIGHT: f64 = 30.0;

pub fn render(layout: &StateLayout, theme: &Theme) -> Document {
    let title_offset = if layout.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let height = layout.total_height + title_offset;

    let mut doc = Document::new()
        .set("xmlns", "http://www.w3.org/2000/svg")
        .set("width", layout.total_width)
        .set("height", height)
        .set("viewBox", format!("0 0 {} {}", layout.total_width, height));

    doc = doc.add(state_defs());
    doc = doc.add(style_block(theme));

    doc = doc.add(background_rect(theme));

    if let Some(ref t) = layout.title {
        let title_el = Text::new()
            .set("x", layout.total_width / 2.0)
            .set("y", TOP_MARGIN)
            .set("text-anchor", "middle")
            .set("class", "title")
            .add(text_node(t.clone()));
        doc = doc.add(title_el);
    }

    let node_map: std::collections::HashMap<&str, &StateLayoutNode> =
        layout.nodes.iter().map(|n| (n.name.as_str(), n)).collect();

    // Pre-compute every route so the nudge pass sees the full edge set and
    // can spread overlapping rails apart before any segment is drawn. Port
    // selection picks the side facing the other node — a fork/join bar is
    // forced top/bottom because its left/right edges have no useful
    // surface area.
    let mut routes: Vec<(usize, Vec<(f64, f64)>)> = Vec::new();
    for (i, edge) in layout.edges.iter().enumerate() {
        if let (Some(from), Some(to)) = (
            node_map.get(edge.from.as_str()),
            node_map.get(edge.to.as_str()),
        ) {
            let from_bbox = (from.x, from.y, from.w, from.h);
            let to_bbox = (to.x, to.y, to.w, to.h);
            let from_centre = (from.x + from.w / 2.0, from.y + from.h / 2.0);
            let to_centre = (to.x + to.w / 2.0, to.y + to.h / 2.0);
            let vertical_only_from = matches!(from.kind, StateKind::Fork | StateKind::Join);
            let vertical_only_to = matches!(to.kind, StateKind::Fork | StateKind::Join);
            let (src, src_side) = pick_port(from_bbox, to_centre, vertical_only_from);
            let (dst, dst_side) = pick_port(to_bbox, from_centre, vertical_only_to);
            let points = orthogonal_through_ports(src, src_side, dst, dst_side);
            routes.push((i, points));
        }
    }
    let mut just_points: Vec<Vec<(f64, f64)>> = routes.iter().map(|(_, p)| p.clone()).collect();
    nudge_overlapping_segments(&mut just_points, EDGE_NUDGE_GAP);
    for ((_, slot), nudged) in routes.iter_mut().zip(just_points) {
        *slot = nudged;
    }

    for (edge_idx, points) in routes {
        let edge = &layout.edges[edge_idx];
        doc = doc.add(render_edge(edge, &points));
    }

    for node in &layout.nodes {
        doc = doc.add(render_node(node));
    }

    doc
}

fn state_defs() -> Definitions {
    let arrow = Polygon::new()
        .set("points", "0 0, 8 4, 0 8")
        .set("fill", "#181818");
    let marker = Marker::new()
        .set("id", "state-arrow")
        .set("markerWidth", "8")
        .set("markerHeight", "8")
        .set("refX", "7")
        .set("refY", "4")
        .set("orient", "auto")
        .add(arrow);

    Definitions::new().add(marker)
}

fn node_cx(n: &StateLayoutNode) -> f64 {
    n.x + n.w / 2.0
}

fn node_cy(n: &StateLayoutNode) -> f64 {
    n.y + n.h / 2.0
}

fn render_node(node: &StateLayoutNode) -> Group {
    if node.name == "[H]" || node.name == "[H*]" {
        return render_history(node);
    }
    match node.kind {
        StateKind::Choice => render_choice(node),
        StateKind::Fork | StateKind::Join => render_bar(node),
        StateKind::Start => render_pseudo(node),
        _ if node.name == "[*]" => render_pseudo(node),
        _ => render_state_box(node),
    }
}

fn render_pseudo(node: &StateLayoutNode) -> Group {
    let cx = node_cx(node);
    let cy = node_cy(node);
    let r = node.w / 2.0;

    let circle = Circle::new()
        .set("cx", cx)
        .set("cy", cy)
        .set("r", r)
        .set("fill", "#181818");

    Group::new().add(circle)
}

fn render_history(node: &StateLayoutNode) -> Group {
    let cx = node_cx(node);
    let cy = node_cy(node);
    let r = node.w.max(node.h) / 2.0;
    let label = if node.name == "[H*]" { "H*" } else { "H" };

    // History marker: outline-only circle with the H/H* glyph inside. The
    // stroke uses the theme-driven arrow colour via the `.arrow` class so
    // it adapts on dark. Text colour comes from the root `text` rule.
    let circle = Circle::new()
        .set("cx", cx)
        .set("cy", cy)
        .set("r", r)
        .set("fill", "none")
        .set("class", "arrow");
    let text = Text::new()
        .set("x", cx)
        .set("y", cy + FONT_SIZE / 3.0)
        .set("text-anchor", "middle")
        .set("font-size", FONT_SIZE - 2.0)
        .set("font-weight", "bold")
        .add(text_node(label));

    Group::new().add(circle).add(text)
}

fn render_choice(node: &StateLayoutNode) -> Group {
    let cx = node_cx(node);
    let cy = node_cy(node);
    let hw = node.w / 2.0;
    let hh = node.h / 2.0;
    let points = format!(
        "{},{} {},{} {},{} {},{}",
        cx,
        cy - hh,
        cx + hw,
        cy,
        cx,
        cy + hh,
        cx - hw,
        cy
    );
    let diamond = Polygon::new()
        .set("points", points)
        .set("fill", "none")
        .set("stroke", "#d6b656")
        .set("stroke-width", 1.5);
    Group::new().add(diamond)
}

fn render_bar(node: &StateLayoutNode) -> Group {
    // Fork/join drawn as a thick horizontal bar spanning the node width.
    let bar = Rectangle::new()
        .set("x", node.x)
        .set("y", node.y + node.h / 2.0 - 3.0)
        .set("width", node.w)
        .set("height", 6.0)
        .set("fill", "#181818");
    Group::new().add(bar)
}

fn render_state_box(node: &StateLayoutNode) -> Group {
    let rect = Rectangle::new()
        .set("x", node.x)
        .set("y", node.y)
        .set("width", node.w)
        .set("height", node.h)
        .set("rx", 10.0)
        .set("ry", 10.0)
        .set("fill", "none")
        .set("stroke", "#6c8ebf")
        .set("stroke-width", 1.5);

    let display = node.label.as_deref().unwrap_or(&node.name);
    let text = Text::new()
        .set("x", node_cx(node))
        .set("y", node.y + node.h / 2.0 + FONT_SIZE / 3.0)
        .set("text-anchor", "middle")
        .set("font-size", FONT_SIZE)
        .add(text_node(display));

    Group::new().add(rect).add(text)
}

fn render_edge(edge: &StateLayoutEdge, points: &[(f64, f64)]) -> Group {
    let mut g = Group::new();
    if points.len() < 2 {
        return g;
    }

    let mut d = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        let cmd = if i == 0 { "M" } else { " L" };
        d.push_str(&format!("{}{},{}", cmd, x, y));
    }
    let path = Path::new()
        .set("d", d)
        .set("class", "arrow")
        .set("marker-end", "url(#state-arrow)");
    g = g.add(path);

    if let Some(ref lbl) = edge.label {
        if !lbl.is_empty() {
            let (lx, ly, anchor) = label_perpendicular(points, 8.0);
            let text = Text::new()
                .set("x", lx)
                .set("y", ly)
                .set("text-anchor", anchor)
                .set("font-size", 11.0)
                .add(text_node(lbl.clone()));
            g = g.add(text);
        }
    }

    g
}
