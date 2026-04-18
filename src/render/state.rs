use svg::node::element::{Circle, Definitions, Group, Line, Marker, Polygon, Rectangle, Text};
use svg::Document;

use super::primitives::{style_block, text_node};
use super::theme::Theme;
use crate::ast::state::StateKind;
use crate::layout::state::{StateLayout, StateLayoutEdge, StateLayoutNode};

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
    doc = doc.add(style_block());

    let bg = Rectangle::new()
        .set("width", "100%")
        .set("height", "100%")
        .set("fill", theme.background_color.as_str());
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

    let circle = Circle::new()
        .set("cx", cx)
        .set("cy", cy)
        .set("r", r)
        .set("fill", "#ffffff")
        .set("stroke", "#181818")
        .set("stroke-width", "1.5");
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
        .set("fill", "#fff2cc")
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
        .set("fill", "#dae8fc")
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

fn attach_bottom(n: &StateLayoutNode) -> (f64, f64) {
    (node_cx(n), n.y + n.h)
}

fn attach_top(n: &StateLayoutNode) -> (f64, f64) {
    (node_cx(n), n.y)
}

fn render_edge(edge: &StateLayoutEdge, from: &StateLayoutNode, to: &StateLayoutNode) -> Group {
    let (x1, y1) = attach_bottom(from);
    let (x2, y2) = attach_top(to);

    let mut g = Group::new();

    let is_back = y2 < y1 - 5.0;

    if is_back {
        let offset_x = 40.0;
        use svg::node::element::Path;
        let d = format!(
            "M{},{} C{},{} {},{} {},{}",
            x1,
            y1,
            x1 + offset_x,
            y1 + 20.0,
            x2 + offset_x,
            y2 - 20.0,
            x2,
            y2
        );
        let path = Path::new()
            .set("d", d)
            .set("class", "arrow")
            .set("marker-end", "url(#state-arrow)");
        g = g.add(path);
    } else {
        let line = Line::new()
            .set("x1", x1)
            .set("y1", y1)
            .set("x2", x2)
            .set("y2", y2)
            .set("class", "arrow")
            .set("marker-end", "url(#state-arrow)");
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
