use svg::node::element::{
    Circle, Definitions, Group, Line, Marker, Path, Polygon, Rectangle, Text,
};
use svg::Document;

use super::primitives::{style_block, text_node};
use crate::layout::activity::{ActivityLayout, LayoutEdge, LayoutNode, Shape};

const FONT_SIZE: f64 = 13.0;
const TOP_MARGIN: f64 = 20.0;
const TITLE_HEIGHT: f64 = 30.0;

pub fn render(layout: &ActivityLayout) -> Document {
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
    doc = doc.add(style_block());

    let bg = Rectangle::new()
        .set("width", "100%")
        .set("height", "100%")
        .set("fill", "#ffffff");
    doc = doc.add(bg);

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

    // Draw edges first (below nodes)
    for edge in &layout.edges {
        if let (Some(from), Some(to)) = (node_map.get(&edge.from), node_map.get(&edge.to)) {
            doc = doc.add(render_edge(edge, from, to));
        }
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
    let fill = node.color.as_deref().unwrap_or("#dae8fc");
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
        .set("fill", "#fff2cc")
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

/// Attachment point on a node for a given direction (top/bottom/center).
fn attach_bottom(n: &LayoutNode) -> (f64, f64) {
    match n.shape {
        Shape::StartEnd | Shape::Decision => (node_cx(n), n.y + n.h),
        Shape::MergeBar => (node_cx(n), n.y + n.h),
        _ => (node_cx(n), n.y + n.h),
    }
}

fn attach_top(n: &LayoutNode) -> (f64, f64) {
    match n.shape {
        Shape::StartEnd | Shape::Decision => (node_cx(n), n.y),
        Shape::MergeBar => (node_cx(n), n.y),
        _ => (node_cx(n), n.y),
    }
}

fn render_edge(edge: &LayoutEdge, from: &LayoutNode, to: &LayoutNode) -> Group {
    let (x1, y1) = attach_bottom(from);
    let (x2, y2) = attach_top(to);

    let is_back = y2 < y1; // back-edge for loops

    let class = if edge.dashed { "arrow-dashed" } else { "arrow" };

    let mut g = Group::new();

    if is_back {
        // Draw as a curved path going left then up then right
        let offset_x = 40.0;
        let d = format!(
            "M{},{} C{},{} {},{} {},{}",
            x1,
            y1,
            x1 - offset_x,
            y1 + 20.0,
            x2 - offset_x,
            y2 - 20.0,
            x2,
            y2
        );
        let path = Path::new()
            .set("d", d)
            .set("class", class)
            .set("marker-end", "url(#act-arrow)");
        g = g.add(path);
    } else {
        let line = Line::new()
            .set("x1", x1)
            .set("y1", y1)
            .set("x2", x2)
            .set("y2", y2)
            .set("class", class)
            .set("marker-end", "url(#act-arrow)");
        g = g.add(line);
    }

    if let Some(ref lbl) = edge.label {
        if !lbl.is_empty() {
            let mx = (x1 + x2) / 2.0 + 6.0;
            let my = (y1 + y2) / 2.0;
            let text = Text::new()
                .set("x", mx)
                .set("y", my)
                .set("font-size", 11.0)
                .add(text_node(lbl.clone()));
            g = g.add(text);
        }
    }

    g
}
