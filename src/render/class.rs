use svg::node::element::{Group, Line, Rectangle, Text};
use svg::Document;

use super::primitives::{style_block, text_node};
use super::theme::Theme;
use crate::ast::class::{ClassKind, RelationKind};
use crate::layout::class::{ClassLayout, EdgeLayout, NodeLayout, NoteBox};

const FONT_SIZE: f64 = 13.0;
const HEADER_TEXT_Y_OFF: f64 = 22.0;
const MEMBER_Y_OFF: f64 = 14.0;
const TOP_MARGIN: f64 = 30.0;
const TITLE_HEIGHT: f64 = 30.0;

pub fn render(layout: &ClassLayout, theme: &Theme) -> Document {
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

    doc = doc.add(class_defs());
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

    for edge in &layout.edges {
        doc = doc.add(render_edge(edge, title_offset));
    }
    for node in &layout.nodes {
        doc = doc.add(render_node(node, title_offset));
    }
    for note in &layout.notes {
        doc = doc.add(render_note(note, title_offset));
    }

    doc
}

fn render_note(note: &NoteBox, y_off: f64) -> Group {
    let y = note.y + y_off;
    let fold = 10.0;
    let pts = format!(
        "{},{} {},{} {},{} {},{} {},{}",
        note.x,
        y,
        note.x + note.width - fold,
        y,
        note.x + note.width,
        y + fold,
        note.x + note.width,
        y + note.height,
        note.x,
        y + note.height
    );
    let body = svg::node::element::Polygon::new()
        .set("points", pts)
        .set("class", "note-box");
    let fold_line = svg::node::element::Polyline::new()
        .set(
            "points",
            format!(
                "{},{} {},{} {},{}",
                note.x + note.width - fold,
                y,
                note.x + note.width - fold,
                y + fold,
                note.x + note.width,
                y + fold
            ),
        )
        .set("fill", "none")
        .set("stroke", "#bbbb00")
        .set("stroke-width", "1");
    let tether = Line::new()
        .set("x1", note.target_x)
        .set("y1", note.target_y + y_off)
        .set(
            "x2",
            if note.target_x < note.x {
                note.x
            } else {
                note.x + note.width
            },
        )
        .set("y2", y + note.height / 2.0)
        .set("stroke", "#bbbb00")
        .set("stroke-width", "1")
        .set("stroke-dasharray", "3,2");

    let mut g = Group::new().add(body).add(fold_line).add(tether);
    for (i, line) in note.lines.iter().enumerate() {
        let ty = y + 16.0 + i as f64 * 16.0;
        let t = Text::new()
            .set("x", note.x + 8.0)
            .set("y", ty)
            .set("font-size", FONT_SIZE - 1.0)
            .add(text_node(line.clone()));
        g = g.add(t);
    }
    g
}

fn class_defs() -> svg::node::element::Definitions {
    use svg::node::element::{Definitions, Marker, Path, Polygon as P};

    let filled = P::new()
        .set("points", "0 0, 10 5, 0 10")
        .set("fill", "#181818");
    let hollow = P::new()
        .set("points", "0 0, 10 5, 0 10")
        .set("fill", "#ffffff")
        .set("stroke", "#181818")
        .set("stroke-width", "1");
    let diamond_f = P::new()
        .set("points", "0 5, 5 0, 10 5, 5 10")
        .set("fill", "#181818");
    let diamond_h = P::new()
        .set("points", "0 5, 5 0, 10 5, 5 10")
        .set("fill", "#ffffff")
        .set("stroke", "#181818")
        .set("stroke-width", "1");

    let mk = |id: &str, child: P| {
        Marker::new()
            .set("id", id)
            .set("markerWidth", "12")
            .set("markerHeight", "12")
            .set("refX", "10")
            .set("refY", "5")
            .set("orient", "auto")
            .add(child)
    };

    // Inline the generic arrowhead markers so we don't nest <defs>.
    let arrow_filled = P::new()
        .set("points", "0 0, 10 5, 0 10")
        .set("fill", "#181818");
    let arrowhead = Marker::new()
        .set("id", "arrowhead")
        .set("markerWidth", "10")
        .set("markerHeight", "10")
        .set("refX", "9")
        .set("refY", "5")
        .set("orient", "auto")
        .add(arrow_filled);

    let open = Path::new()
        .set("d", "M0,0 L10,5 L0,10")
        .set("fill", "none")
        .set("stroke", "#181818")
        .set("stroke-width", "1.5");
    let arrowhead_open = Marker::new()
        .set("id", "arrowhead-open")
        .set("markerWidth", "10")
        .set("markerHeight", "10")
        .set("refX", "9")
        .set("refY", "5")
        .set("orient", "auto")
        .add(open);

    Definitions::new()
        .add(mk("arr-filled", filled))
        .add(mk("arr-hollow", hollow))
        .add(mk("arr-diamond-f", diamond_f))
        .add(mk("arr-diamond-h", diamond_h))
        .add(arrowhead)
        .add(arrowhead_open)
}

fn render_node(node: &NodeLayout, y_off: f64) -> Group {
    let x = node.x;
    let y = node.y + y_off;
    let w = node.width;
    let h = node.height;

    let (fill, stroke, header_fill) = match node.kind {
        ClassKind::Interface => ("#f5f5ff", "#6666bb", "#dde"),
        ClassKind::Abstract => ("#fff5f5", "#bb6666", "#edd"),
        ClassKind::Enum => ("#f5fff5", "#66bb66", "#ded"),
        ClassKind::Object => ("#fff9e6", "#b8a85a", "#f0e4b0"),
        ClassKind::Component => ("#e6f2ff", "#3d6aa0", "#b8d4f0"),
        // Deployment kinds — distinct but muted palettes keep them visually
        // grouped without competing with class content.
        ClassKind::Node => ("#eeeeee", "#444444", "#cccccc"),
        ClassKind::Cloud => ("#f0f8ff", "#708090", "#d0e4f5"),
        ClassKind::Database => ("#fff0f5", "#8b5a8b", "#f0cddf"),
        ClassKind::Folder => ("#fffacd", "#b8860b", "#ffe4b5"),
        ClassKind::Frame => ("#f5f5f5", "#555555", "#e0e0e0"),
        ClassKind::Rectangle => ("#f8f8f8", "#666666", "#e8e8e8"),
        ClassKind::Artifact => ("#fffaf0", "#8b7355", "#f5deb3"),
        ClassKind::Queue => ("#f0f5ff", "#4682b4", "#d0e0f0"),
        _ => ("#dae8fc", "#6c8ebf", "#c5d8f0"),
    };

    let border = Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5")
        .set("rx", "3");

    let header_rect = Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", node.header_h)
        .set("fill", header_fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5")
        .set("rx", "3");

    let mut g = Group::new().add(border).add(header_rect);

    // Stereotype label
    if let Some(ref stereo) = node.stereotype {
        let st = Text::new()
            .set("x", x + w / 2.0)
            .set("y", y + 14.0)
            .set("text-anchor", "middle")
            .set("font-size", "10")
            .set("fill", "#555")
            .add(text_node(format!("«{}»", stereo)));
        g = g.add(st);
    }

    // Kind label for interface/abstract/enum/component and deployment kinds.
    // Objects intentionally don't get one; the underlined name signals
    // instance-hood. Plain Class and Rectangle are intentionally unlabelled.
    let kind_label = match node.kind {
        ClassKind::Interface => Some("«interface»"),
        ClassKind::Abstract => Some("«abstract»"),
        ClassKind::Enum => Some("«enum»"),
        ClassKind::Annotation => Some("«annotation»"),
        ClassKind::Component => Some("«component»"),
        ClassKind::Node => Some("«node»"),
        ClassKind::Cloud => Some("«cloud»"),
        ClassKind::Database => Some("«database»"),
        ClassKind::Folder => Some("«folder»"),
        ClassKind::Frame => Some("«frame»"),
        ClassKind::Artifact => Some("«artifact»"),
        ClassKind::Queue => Some("«queue»"),
        ClassKind::Object | ClassKind::Class | ClassKind::Rectangle => None,
    };
    let name_y_adjust = if kind_label.is_some() || node.stereotype.is_some() {
        4.0
    } else {
        0.0
    };

    if let Some(kl) = kind_label {
        if node.stereotype.is_none() {
            let kl_el = Text::new()
                .set("x", x + w / 2.0)
                .set("y", y + 12.0)
                .set("text-anchor", "middle")
                .set("font-size", "10")
                .set("fill", "#555")
                .add(text_node(kl));
            g = g.add(kl_el);
        }
    }

    let mut name_el = Text::new()
        .set("x", x + w / 2.0)
        .set("y", y + HEADER_TEXT_Y_OFF + name_y_adjust)
        .set("text-anchor", "middle")
        .set(
            "font-weight",
            if matches!(node.kind, ClassKind::Abstract) {
                "italic"
            } else {
                "bold"
            },
        )
        .add(text_node(node.display_name.clone()));
    if matches!(node.kind, ClassKind::Object) {
        // UML convention: object instance names are underlined.
        name_el = name_el.set("text-decoration", "underline");
    }
    g = g.add(name_el);

    // Component adornment: two small port tabs on the left edge.
    if matches!(node.kind, ClassKind::Component) {
        let port_w = 12.0;
        let port_h = 8.0;
        let port_y1 = y + node.header_h / 2.0 - port_h - 2.0;
        let port_y2 = y + node.header_h / 2.0 + 2.0;
        for py in [port_y1, port_y2] {
            let port = Rectangle::new()
                .set("x", x - port_w / 2.0)
                .set("y", py)
                .set("width", port_w)
                .set("height", port_h)
                .set("fill", header_fill)
                .set("stroke", stroke)
                .set("stroke-width", "1.5");
            g = g.add(port);
        }
    }

    // Separator line below header
    if !node.member_sections.iter().all(|s| s.members.is_empty()) {
        let sep = Line::new()
            .set("x1", x)
            .set("y1", y + node.header_h)
            .set("x2", x + w)
            .set("y2", y + node.header_h)
            .set("stroke", stroke)
            .set("stroke-width", "1");
        g = g.add(sep);
    }

    // Members
    let mut my = y + node.header_h + MEMBER_Y_OFF;
    for section in &node.member_sections {
        if section.separator {
            let sep = Line::new()
                .set("x1", x)
                .set("y1", my - 4.0)
                .set("x2", x + w)
                .set("y2", my - 4.0)
                .set("stroke", stroke)
                .set("stroke-width", "0.5")
                .set("stroke-dasharray", "3,2");
            g = g.add(sep);
        }
        for member in &section.members {
            let txt = Text::new()
                .set("x", x + 8.0)
                .set("y", my)
                .set(
                    "font-style",
                    if member.is_abstract {
                        "italic"
                    } else {
                        "normal"
                    },
                )
                .set(
                    "text-decoration",
                    if member.is_static {
                        "underline"
                    } else {
                        "none"
                    },
                )
                .add(text_node(member.text.clone()));
            g = g.add(txt);
            my += FONT_SIZE + 7.0;
        }
    }

    g
}

fn render_edge(edge: &EdgeLayout, y_off: f64) -> Group {
    let (x1, y1) = (edge.from_x, edge.from_y + y_off);
    let (x2, y2) = (edge.to_x, edge.to_y + y_off);

    let (stroke_class, marker_end) = match edge.kind {
        RelationKind::Extension => ("class-line", "url(#arr-hollow)"),
        RelationKind::Implementation => ("class-line-dashed", "url(#arr-hollow)"),
        RelationKind::Composition => ("class-line", "url(#arr-diamond-f)"),
        RelationKind::Aggregation => ("class-line", "url(#arr-diamond-h)"),
        RelationKind::Dependency => ("class-line", "url(#arrowhead)"),
        RelationKind::DashedLink => ("class-line-dashed", "url(#arrowhead)"),
        RelationKind::Realization => ("class-line-dashed", "url(#arr-hollow)"),
        RelationKind::Association => ("class-line", "none"),
    };

    let line = Line::new()
        .set("x1", x1)
        .set("y1", y1)
        .set("x2", x2)
        .set("y2", y2)
        .set("class", stroke_class)
        .set("marker-end", marker_end);

    let mut g = Group::new().add(line);

    if let Some(ref lbl) = edge.label {
        let mx = (x1 + x2) / 2.0;
        let my = (y1 + y2) / 2.0 - 4.0;
        let t = Text::new()
            .set("x", mx)
            .set("y", my)
            .set("text-anchor", "middle")
            .set("font-size", "11")
            .add(text_node(lbl.clone()));
        g = g.add(t);
    }

    if let Some(ref lbl) = edge.from_label {
        let t = Text::new()
            .set("x", x1 + 6.0)
            .set("y", y1 - 4.0)
            .set("font-size", "11")
            .add(text_node(lbl.clone()));
        g = g.add(t);
    }

    if let Some(ref lbl) = edge.to_label {
        let t = Text::new()
            .set("x", x2 + 6.0)
            .set("y", y2 - 4.0)
            .set("font-size", "11")
            .add(text_node(lbl.clone()));
        g = g.add(t);
    }

    g
}
