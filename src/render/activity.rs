use svg::node::element::{Circle, Definitions, Group, Marker, Path, Polygon, Rectangle, Text};
use svg::Document;

use super::primitives::{background_rect, label_perpendicular, style_block, text_node};
use super::theme::Theme;
use crate::layout::activity::{ActivityLayout, LayoutEdge, LayoutNode, Shape};
use crate::layout::ports::pick_port as pick_port_bbox;
use crate::layout::sugiyama::{nudge_overlapping_segments, orthogonal_through_ports, Side};

const EDGE_NUDGE_GAP: f64 = 6.0;

const FONT_SIZE: f64 = 13.0;
const TOP_MARGIN: f64 = 20.0;
const TITLE_HEIGHT: f64 = 30.0;

pub fn render(layout: &ActivityLayout, theme: &Theme) -> Document {
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

    doc = doc.add(activity_defs());
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

    // Build id→node map for edge routing
    let node_map: std::collections::HashMap<usize, &LayoutNode> =
        layout.nodes.iter().map(|n| (n.id, n)).collect();

    // Pre-route every edge so the nudge pass can spread overlapping rails
    // before any segment is drawn.
    let mut routes: Vec<(usize, Vec<(f64, f64)>)> = Vec::new();
    for (i, edge) in layout.edges.iter().enumerate() {
        if let (Some(from), Some(to)) = (node_map.get(&edge.from), node_map.get(&edge.to)) {
            let from_center = (node_cx(from), node_cy(from));
            let to_center = (node_cx(to), node_cy(to));
            let (src, src_side) = pick_port(from, to_center);
            let (dst, dst_side) = pick_port(to, from_center);
            let points = orthogonal_through_ports(src, src_side, dst, dst_side);
            routes.push((i, points));
        }
    }
    let mut just_points: Vec<Vec<(f64, f64)>> = routes.iter().map(|(_, p)| p.clone()).collect();
    nudge_overlapping_segments(&mut just_points, EDGE_NUDGE_GAP);
    for ((_, slot), nudged) in routes.iter_mut().zip(just_points) {
        *slot = nudged;
    }

    // Draw edges first (below nodes).
    for (edge_idx, points) in routes {
        let edge = &layout.edges[edge_idx];
        doc = doc.add(render_edge(edge, &points));
    }

    // Draw nodes on top
    for node in &layout.nodes {
        doc = doc.add(render_node(node));
    }

    doc
}

fn activity_defs() -> Definitions {
    let arrow = Polygon::new()
        .set("points", "0 0, 8 4, 0 8")
        .set("fill", "#181818");
    let marker = Marker::new()
        .set("id", "act-arrow")
        .set("markerWidth", "8")
        .set("markerHeight", "8")
        .set("refX", "7")
        .set("refY", "4")
        .set("orient", "auto")
        .add(arrow);

    Definitions::new().add(marker)
}

fn node_cx(n: &LayoutNode) -> f64 {
    n.x + n.w / 2.0
}

fn node_cy(n: &LayoutNode) -> f64 {
    n.y + n.h / 2.0
}

fn render_node(node: &LayoutNode) -> Group {
    let g = Group::new();
    match node.shape {
        Shape::StartEnd => render_start_end(g, node),
        Shape::Action => render_action(g, node),
        Shape::Decision => render_decision(g, node),
        Shape::MergeBar => render_merge_bar(g, node),
        Shape::Note => render_note_shape(g, node),
        Shape::Arrow => g, // invisible routing point
    }
}

fn render_start_end(g: Group, node: &LayoutNode) -> Group {
    let r = node.w / 2.0;
    let cx = node_cx(node);
    let cy = node_cy(node);
    let is_stop = node.label == "stop";

    let outer = Circle::new()
        .set("cx", cx)
        .set("cy", cy)
        .set("r", r)
        .set("fill", "#181818");

    if is_stop {
        // Stop = filled circle inside a ring
        let ring = Circle::new()
            .set("cx", cx)
            .set("cy", cy)
            .set("r", r + 3.0)
            .set("fill", "none")
            .set("stroke", "#181818")
            .set("stroke-width", 2.0);
        g.add(ring).add(outer)
    } else {
        g.add(outer)
    }
}

fn render_action(g: Group, node: &LayoutNode) -> Group {
    // Per-action colour (e.g. `:task; #aabbcc`) still lands if supplied;
    // default is transparent so the rounded box reads on either canvas.
    let fill = node.color.as_deref().unwrap_or("none");
    let rect = Rectangle::new()
        .set("x", node.x)
        .set("y", node.y)
        .set("width", node.w)
        .set("height", node.h)
        .set("rx", 8.0)
        .set("ry", 8.0)
        .set("fill", fill)
        .set("stroke", "#6c8ebf")
        .set("stroke-width", 1.5);

    let text = Text::new()
        .set("x", node_cx(node))
        .set("y", node.y + node.h / 2.0 + FONT_SIZE / 3.0)
        .set("text-anchor", "middle")
        .add(text_node(node.label.clone()));

    g.add(rect).add(text)
}

fn render_decision(g: Group, node: &LayoutNode) -> Group {
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

    let mut g = g.add(diamond);

    if !node.label.is_empty() {
        let text = Text::new()
            .set("x", cx)
            .set("y", cy + FONT_SIZE / 3.0)
            .set("text-anchor", "middle")
            .set("font-size", FONT_SIZE)
            .add(text_node(node.label.clone()));
        g = g.add(text);
    }
    g
}

fn render_merge_bar(g: Group, node: &LayoutNode) -> Group {
    let bar = Rectangle::new()
        .set("x", node.x)
        .set("y", node.y)
        .set("width", node.w)
        .set("height", node.h)
        .set("fill", "#181818");
    g.add(bar)
}

fn render_note_shape(g: Group, node: &LayoutNode) -> Group {
    let fold = 10.0;
    let x = node.x;
    let y = node.y;
    let w = node.w;
    let h = node.h;

    let body = Path::new()
        .set(
            "d",
            format!(
                "M{},{} L{},{} L{},{} L{},{} Z",
                x,
                y,
                x + w - fold,
                y,
                x + w,
                y + fold,
                x + w,
                y + h,
            ),
        )
        .set("fill", "#ffffc0")
        .set("stroke", "#bbbb00")
        .set("stroke-width", 1.0);

    // Left+bottom sides via separate path to close properly
    let border = Path::new()
        .set(
            "d",
            format!("M{},{} L{},{} L{},{}", x, y, x, y + h, x + w, y + h),
        )
        .set("fill", "none")
        .set("stroke", "#bbbb00")
        .set("stroke-width", 1.0);

    let fold_line = Path::new()
        .set(
            "d",
            format!(
                "M{},{} L{},{} L{},{}",
                x + w - fold,
                y,
                x + w - fold,
                y + fold,
                x + w,
                y + fold
            ),
        )
        .set("fill", "none")
        .set("stroke", "#bbbb00")
        .set("stroke-width", 1.0);

    let text = Text::new()
        .set("x", x + 6.0)
        .set("y", y + FONT_SIZE + 4.0)
        .set("font-size", FONT_SIZE)
        .add(text_node(node.label.clone()));

    g.add(body).add(border).add(fold_line).add(text)
}

/// Pick the edge attachment port on `node` facing a position `toward`.
///
/// Wraps `layout::ports::pick_port` with activity-specific shape policy.
/// In an activity diagram the conventional reading flow is **top → bottom**
/// — every action box should have its arrows enter at the top and exit
/// at the bottom. Decisions are the only shape that fans out sideways
/// (left/right corners of the diamond carry the branch exits). Bars are
/// always horizontal too. So:
///
/// - `MergeBar`, `Action`, `StartEnd`, `Note`, `Arrow` → vertical only.
/// - `Decision` → free choice; the diamond's geometry naturally selects
///   top for the incoming edge and left/right corners for the branches.
fn pick_port(node: &LayoutNode, toward: (f64, f64)) -> ((f64, f64), Side) {
    let bbox = (node.x, node.y, node.w, node.h);
    let vertical_only = !matches!(node.shape, Shape::Decision);
    pick_port_bbox(bbox, toward, vertical_only)
}

fn render_edge(edge: &LayoutEdge, points: &[(f64, f64)]) -> Group {
    let class = if edge.dashed { "arrow-dashed" } else { "arrow" };

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
        .set("class", class)
        .set("marker-end", "url(#act-arrow)");
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
