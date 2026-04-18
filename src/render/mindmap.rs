use svg::node::element::{Group, Rectangle, Text};
use svg::Document;

use super::primitives::{background_rect, style_block, text_node};
use super::theme::Theme;
use crate::layout::mindmap::{MindLayoutEdge, MindLayoutNode, MindMapLayout};

const FONT_SIZE: f64 = 13.0;
const TOP_MARGIN: f64 = 20.0;
const TITLE_HEIGHT: f64 = 30.0;

pub fn render(layout: &MindMapLayout, theme: &Theme) -> Document {
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

    // Edges behind nodes.
    for edge in &layout.edges {
        doc = doc.add(render_edge(edge));
    }
    for node in &layout.nodes {
        doc = doc.add(render_node(node));
    }
    doc
}

fn render_edge(edge: &MindLayoutEdge) -> Group {
    // Curved orthogonal connector: from the sided attach point of the parent
    // horizontally to a midpoint, then vertically to the child's y, then
    // horizontally to the child's attach point.
    let mx = (edge.from_x + edge.to_x) / 2.0;
    use svg::node::element::Path;
    let d = format!(
        "M{},{} C{},{} {},{} {},{}",
        edge.from_x, edge.from_y, mx, edge.from_y, mx, edge.to_y, edge.to_x, edge.to_y
    );
    let path = Path::new()
        .set("d", d)
        .set("fill", "none")
        .set("stroke", "#888")
        .set("stroke-width", "1.2");
    Group::new().add(path)
}

fn render_node(node: &MindLayoutNode) -> Group {
    // Outline-only nodes. Hierarchy is communicated by position (parent →
    // children fan out left/right) and by the root's bolder label, not by
    // filling colour. A user who wants a filled node can supply `[#color]`
    // in the source — we still honour that.
    let fill = node.color.as_deref().unwrap_or("none");
    let stroke_width = if node.depth == 1 { "1.8" } else { "1.2" };
    let rect = Rectangle::new()
        .set("x", node.x)
        .set("y", node.y)
        .set("width", node.w)
        .set("height", node.h)
        .set("rx", "14")
        .set("ry", "14")
        .set("fill", fill)
        .set("stroke", "#3d6aa0")
        .set("stroke-width", stroke_width);
    let label = Text::new()
        .set("x", node.x + node.w / 2.0)
        .set("y", node.y + node.h / 2.0 + FONT_SIZE / 3.0)
        .set("text-anchor", "middle")
        .set("font-size", FONT_SIZE)
        .set(
            "font-weight",
            if node.depth == 1 { "bold" } else { "normal" },
        )
        .add(text_node(node.label.clone()));
    Group::new().add(rect).add(label)
}
