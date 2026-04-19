use svg::node::element::{Circle, Definitions, Group, Marker, Path, Polygon, Rectangle, Text};
use svg::Document;

use super::primitives::{background_rect, style_block, text_node};
use super::theme::Theme;
use crate::ast::state::StateKind;
use crate::layout::state::{StateLayout, StateLayoutEdge, StateLayoutNode};
use crate::layout::sugiyama::orthogonal_route;

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

    for edge in &layout.edges {
        if let (Some(from), Some(to)) = (
            node_map.get(edge.from.as_str()),
            node_map.get(edge.to.as_str()),
        ) {
            doc = doc.add(render_edge(edge, from, to));
        }
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

fn render_edge(edge: &StateLayoutEdge, from: &StateLayoutNode, to: &StateLayoutNode) -> Group {
    // Orthogonal route handles forward, backward (loop-back), and sibling
    // transitions with the same call — it picks source/target ports and
    // bend points from the two bounding boxes.
    let points = orthogonal_route((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h));

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
            // Label at the path midpoint, offset slightly right of the line
            // so it doesn't sit on the stroke itself.
            let (mx, my) = polyline_midpoint(&points);
            let text = Text::new()
                .set("x", mx + 6.0)
                .set("y", my)
                .set("font-size", 11.0)
                .add(text_node(lbl.clone()));
            g = g.add(text);
        }
    }

    g
}

fn polyline_midpoint(points: &[(f64, f64)]) -> (f64, f64) {
    if points.len() < 2 {
        return points.first().copied().unwrap_or((0.0, 0.0));
    }
    let seg_lens: Vec<f64> = points
        .windows(2)
        .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
        .collect();
    let total: f64 = seg_lens.iter().sum();
    let half = total / 2.0;
    let mut travelled = 0.0;
    for (i, &len) in seg_lens.iter().enumerate() {
        if travelled + len >= half {
            let t = if len > 0.0 {
                (half - travelled) / len
            } else {
                0.0
            };
            let (x0, y0) = points[i];
            let (x1, y1) = points[i + 1];
            return (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
        }
        travelled += len;
    }
    *points.last().unwrap()
}
