use svg::node::element::{Circle, Definitions, Ellipse, Group, Line, Marker, Polygon, Text};
use svg::Document;

use super::primitives::{background_rect, style_block, text_node};
use super::theme::Theme;
use crate::ast::usecase::NodeKind;
use crate::layout::usecase::{UseCaseLayout, UseCaseLayoutEdge, UseCaseLayoutNode};

const FONT_SIZE: f64 = 13.0;
const TOP_MARGIN: f64 = 20.0;
const TITLE_HEIGHT: f64 = 30.0;

pub fn render(layout: &UseCaseLayout, theme: &Theme) -> Document {
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

    doc = doc.add(usecase_defs());
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

    // Index for edge attachment lookups.
    let index: std::collections::HashMap<&str, &UseCaseLayoutNode> =
        layout.nodes.iter().map(|n| (n.name.as_str(), n)).collect();

    for edge in &layout.edges {
        if let (Some(from), Some(to)) = (index.get(edge.from.as_str()), index.get(edge.to.as_str()))
        {
            doc = doc.add(render_edge(edge, from, to));
        }
    }

    for node in &layout.nodes {
        doc = doc.add(render_node(node));
    }

    doc
}

fn usecase_defs() -> Definitions {
    let arrow = Polygon::new()
        .set("points", "0 0, 8 4, 0 8")
        .set("fill", "#181818");
    let marker = Marker::new()
        .set("id", "uc-arrow")
        .set("markerWidth", "8")
        .set("markerHeight", "8")
        .set("refX", "7")
        .set("refY", "4")
        .set("orient", "auto")
        .add(arrow);
    Definitions::new().add(marker)
}

fn render_node(node: &UseCaseLayoutNode) -> Group {
    match node.kind {
        NodeKind::Actor => render_actor(node),
        NodeKind::UseCase => render_usecase(node),
    }
}

fn render_actor(node: &UseCaseLayoutNode) -> Group {
    // Stick figure centred within the node's bounding box, name below.
    let cx = node.x + node.w / 2.0;
    let top = node.y;
    let head_r = 8.0;
    let head_cy = top + head_r + 2.0;
    let body_top = head_cy + head_r;
    let body_bot = top + node.h * 0.65;
    let arm_y = body_top + (body_bot - body_top) * 0.4;

    // Stroke colour flows from the theme-aware `.arrow` CSS class so the
    // actor stays visible on both light and dark backgrounds.
    let head = Circle::new()
        .set("cx", cx)
        .set("cy", head_cy)
        .set("r", head_r)
        .set("class", "arrow");
    let body = Line::new()
        .set("x1", cx)
        .set("y1", body_top)
        .set("x2", cx)
        .set("y2", body_bot)
        .set("class", "arrow");
    let arms = Line::new()
        .set("x1", cx - 14.0)
        .set("y1", arm_y)
        .set("x2", cx + 14.0)
        .set("y2", arm_y)
        .set("class", "arrow");
    let leg_l = Line::new()
        .set("x1", cx)
        .set("y1", body_bot)
        .set("x2", cx - 10.0)
        .set("y2", body_bot + 14.0)
        .set("class", "arrow");
    let leg_r = Line::new()
        .set("x1", cx)
        .set("y1", body_bot)
        .set("x2", cx + 10.0)
        .set("y2", body_bot + 14.0)
        .set("class", "arrow");
    let label = Text::new()
        .set("x", cx)
        .set("y", top + node.h - 2.0)
        .set("text-anchor", "middle")
        .set("font-size", FONT_SIZE)
        .add(text_node(node.display.clone()));

    Group::new()
        .add(head)
        .add(body)
        .add(arms)
        .add(leg_l)
        .add(leg_r)
        .add(label)
}

fn render_usecase(node: &UseCaseLayoutNode) -> Group {
    let cx = node.x + node.w / 2.0;
    let cy = node.y + node.h / 2.0;
    let rx = node.w / 2.0;
    let ry = node.h / 2.0;

    let ellipse = Ellipse::new()
        .set("cx", cx)
        .set("cy", cy)
        .set("rx", rx)
        .set("ry", ry)
        .set("fill", "none")
        .set("stroke", "#d6b656")
        .set("stroke-width", "1.5");
    let label = Text::new()
        .set("x", cx)
        .set("y", cy + FONT_SIZE / 3.0)
        .set("text-anchor", "middle")
        .set("font-size", FONT_SIZE)
        .add(text_node(node.display.clone()));

    let mut g = Group::new().add(ellipse).add(label);
    if let Some(ref s) = node.stereotype {
        let st = Text::new()
            .set("x", cx)
            .set("y", node.y + node.h + FONT_SIZE)
            .set("text-anchor", "middle")
            .set("font-size", "10")
            .set("fill", "#555")
            .add(text_node(format!("«{}»", s)));
        g = g.add(st);
    }
    g
}

fn render_edge(
    edge: &UseCaseLayoutEdge,
    from: &UseCaseLayoutNode,
    to: &UseCaseLayoutNode,
) -> Group {
    let (x1, y1) = attachment(from, to);
    let (x2, y2) = attachment(to, from);
    let class = if edge.dashed { "arrow-dashed" } else { "arrow" };
    let line = Line::new()
        .set("x1", x1)
        .set("y1", y1)
        .set("x2", x2)
        .set("y2", y2)
        .set("class", class)
        .set("marker-end", "url(#uc-arrow)");

    let mut g = Group::new().add(line);

    if let Some(ref lbl) = edge.label {
        if !lbl.is_empty() {
            let t = Text::new()
                .set("x", (x1 + x2) / 2.0)
                .set("y", (y1 + y2) / 2.0 - 4.0)
                .set("text-anchor", "middle")
                .set("font-size", 11.0)
                .add(text_node(lbl.clone()));
            g = g.add(t);
        }
    }
    g
}

/// Return the attachment point on `node` facing `toward`.
fn attachment(node: &UseCaseLayoutNode, toward: &UseCaseLayoutNode) -> (f64, f64) {
    let cx = node.x + node.w / 2.0;
    let cy = node.y + node.h / 2.0;
    let tcx = toward.x + toward.w / 2.0;
    if tcx > cx {
        (node.x + node.w, cy) // right side
    } else {
        (node.x, cy) // left side
    }
}
