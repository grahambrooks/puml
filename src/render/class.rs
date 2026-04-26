use svg::node::element::{Ellipse, Group, Line, Path, Polygon, Rectangle, Text};
use svg::Document;

use super::primitives::{background_rect, label_perpendicular, style_block, text_node};
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

    // Boundaries render first so they sit underneath nodes and edges.
    for boundary in &layout.boundaries {
        doc = doc.add(render_boundary(boundary, title_offset));
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

fn render_boundary(b: &crate::layout::class::BoundaryBox, y_off: f64) -> Group {
    let y = b.y + y_off;
    let rect = Rectangle::new()
        .set("x", b.x)
        .set("y", y)
        .set("width", b.width)
        .set("height", b.height)
        .set("rx", 8.0)
        .set("ry", 8.0)
        .set("fill", "none")
        .set("stroke", "#888888")
        .set("stroke-width", 1.5)
        .set("stroke-dasharray", "8,4");

    let title_label = if b.kind.is_empty() {
        b.label.clone()
    } else {
        format!("{} «{} boundary»", b.label, b.kind)
    };
    // Title sits in the top stripe of the boundary rectangle, left-aligned
    // with a small inset so the dashed stroke is visible behind it.
    let title = Text::new()
        .set("x", b.x + 12.0)
        .set("y", y + 18.0)
        .set("font-size", 12.0)
        .set("font-weight", "bold")
        .set("fill", "#666666")
        .add(text_node(title_label));

    Group::new().add(rect).add(title)
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

/// Build the shape geometry for one node. Every shape is outline-only —
/// `fill="none"` — so the canvas background shows through. That's what makes
/// the rendering adapt naturally to light/dark viewers in `auto` mode
/// without the renderer needing to know which one it's in.
///
/// Differentiation comes from stroke colour + stereotype label + (for
/// deployment kinds) shape geometry. The `fill`/`header_fill` args are still
/// threaded through so a future palette that does want accents can reclaim
/// them without re-plumbing every call site.
fn draw_shape(node: &NodeLayout, y: f64, _fill: &str, stroke: &str, _header_fill: &str) -> Group {
    let x = node.x;
    let w = node.width;
    let h = node.height;
    // Outlines read against both light and dark canvases; shape bodies stay
    // transparent so the rendering follows the viewer's background.
    let fill = "none";

    match node.kind {
        ClassKind::Database => cylinder_shape(x, y, w, h, fill, stroke),
        ClassKind::Queue => queue_shape(x, y, w, h, fill, stroke),
        ClassKind::Cloud => cloud_shape(x, y, w, h, fill, stroke),
        ClassKind::Folder => folder_shape(x, y, w, h, fill, stroke),
        ClassKind::Frame => frame_shape(x, y, w, h, fill, stroke),
        ClassKind::Artifact => artifact_shape(x, y, w, h, fill, stroke),
        ClassKind::Node => node3d_shape(x, y, w, h, fill, stroke),
        _ => {
            // Class family: a single outlined rectangle. The separator line
            // below the header (drawn elsewhere in render_node) gives the
            // visual break between the name block and members — no coloured
            // header band needed.
            let border = Rectangle::new()
                .set("x", x)
                .set("y", y)
                .set("width", w)
                .set("height", h)
                .set("fill", fill)
                .set("stroke", stroke)
                .set("stroke-width", "1.5")
                .set("rx", "3");

            Group::new().add(border)
        }
    }
}

/// Vertical cylinder: elliptical cap on top, front/side outlines, bottom
/// arc. Body is transparent so the canvas shows through.
fn cylinder_shape(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) -> Group {
    let cap_ry = 8.0_f64.min(h * 0.18);
    let body_top = y + cap_ry;
    let body_bot = y + h - cap_ry;

    let left = Line::new()
        .set("x1", x)
        .set("y1", body_top)
        .set("x2", x)
        .set("y2", body_bot)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    let right = Line::new()
        .set("x1", x + w)
        .set("y1", body_top)
        .set("x2", x + w)
        .set("y2", body_bot)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    let top = Ellipse::new()
        .set("cx", x + w / 2.0)
        .set("cy", body_top)
        .set("rx", w / 2.0)
        .set("ry", cap_ry)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    // Bottom: just the visible front arc.
    let bottom_arc = Path::new()
        .set(
            "d",
            format!(
                "M{},{} A{},{} 0 0 0 {},{}",
                x,
                body_bot,
                w / 2.0,
                cap_ry,
                x + w,
                body_bot
            ),
        )
        .set("fill", "none")
        .set("stroke", stroke)
        .set("stroke-width", "1.5");

    Group::new().add(left).add(right).add(bottom_arc).add(top)
}

/// Horizontal cylinder / queue: cap on the left, body, arc on the right.
fn queue_shape(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) -> Group {
    let cap_rx = 10.0_f64.min(w * 0.15);
    let body_left = x + cap_rx;
    let body_right = x + w - cap_rx;

    let top = Line::new()
        .set("x1", body_left)
        .set("y1", y)
        .set("x2", body_right)
        .set("y2", y)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    let bot = Line::new()
        .set("x1", body_left)
        .set("y1", y + h)
        .set("x2", body_right)
        .set("y2", y + h)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    let left_cap = Ellipse::new()
        .set("cx", body_left)
        .set("cy", y + h / 2.0)
        .set("rx", cap_rx)
        .set("ry", h / 2.0)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    // Right-side arc indicating the far end of the cylinder.
    let right_arc = Path::new()
        .set(
            "d",
            format!(
                "M{},{} A{},{} 0 0 1 {},{}",
                body_right,
                y,
                cap_rx,
                h / 2.0,
                body_right,
                y + h,
            ),
        )
        .set("fill", "none")
        .set("stroke", stroke)
        .set("stroke-width", "1.5");

    Group::new().add(top).add(bot).add(left_cap).add(right_arc)
}

/// Cloud outline: a chain of circular arcs sketching a lobed cloud. The path
/// is parameterised by the node's bounding box so labels still centre inside.
fn cloud_shape(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) -> Group {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let rx = w / 2.0;
    let ry = h / 2.0;
    // Six lobes evenly distributed around the ellipse; each lobe is an arc
    // that bulges outward from the ellipse surface.
    let mut d = String::new();
    let lobes = 8;
    let bulge = (w.min(h) * 0.15).max(6.0);
    for i in 0..lobes {
        let theta0 = (i as f64 / lobes as f64) * std::f64::consts::TAU;
        let theta1 = ((i + 1) as f64 / lobes as f64) * std::f64::consts::TAU;
        let x0 = cx + rx * theta0.cos();
        let y0 = cy + ry * theta0.sin();
        let x1 = cx + rx * theta1.cos();
        let y1 = cy + ry * theta1.sin();
        if i == 0 {
            d.push_str(&format!("M{:.2},{:.2}", x0, y0));
        }
        // A clockwise arc with a radius slightly bigger than the lobe chord
        // gives the puffed-out cloud look.
        let chord = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let r = chord / 1.6 + bulge;
        d.push_str(&format!(" A{:.2},{:.2} 0 0 1 {:.2},{:.2}", r, r, x1, y1));
    }
    d.push_str(" Z");
    let path = Path::new()
        .set("d", d)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    Group::new().add(path)
}

/// Folder: outline with a small tab on the upper-left corner, transparent
/// body so the canvas shows through.
fn folder_shape(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) -> Group {
    let tab_w = (w * 0.35).min(60.0);
    let tab_h = 10.0;
    // Tab trapezoid flowing into the body rectangle.
    let d = format!(
        "M{x0},{ty} L{tx1},{ty} L{tx2},{by} L{x1},{by} L{x1},{yb} L{x0},{yb} Z",
        x0 = x,
        ty = y,
        tx1 = x + tab_w,
        tx2 = x + tab_w + tab_h,
        by = y + tab_h,
        x1 = x + w,
        yb = y + h,
    );
    let outline = Path::new()
        .set("d", d)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    // Divider line under the tab so it reads as a distinct region even
    // without a separate fill colour.
    let tab_divider = Line::new()
        .set("x1", x)
        .set("y1", y + tab_h)
        .set("x2", x + tab_w + tab_h)
        .set("y2", y + tab_h)
        .set("stroke", stroke)
        .set("stroke-width", "1");
    Group::new().add(outline).add(tab_divider)
}

/// Frame: outlined rectangle with a small labelled tab in the top-left
/// corner. Both the body and the tab interior stay transparent.
fn frame_shape(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) -> Group {
    let tab_w = 42.0;
    let tab_h = 14.0;
    let border = Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5")
        .set("rx", "2");
    let tab = Path::new()
        .set(
            "d",
            format!(
                "M{x0},{y0} L{x1},{y0} L{x2},{y1} L{x0},{y1} Z",
                x0 = x,
                y0 = y,
                x1 = x + tab_w,
                x2 = x + tab_w + tab_h / 2.0,
                y1 = y + tab_h,
            ),
        )
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    Group::new().add(border).add(tab)
}

/// Artifact: a document outline with a folded top-right corner.
fn artifact_shape(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) -> Group {
    let fold = 14.0;
    let body = Path::new()
        .set(
            "d",
            format!(
                "M{},{} L{},{} L{},{} L{},{} L{},{} Z",
                x,
                y,
                x + w - fold,
                y,
                x + w,
                y + fold,
                x + w,
                y + h,
                x,
                y + h,
            ),
        )
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    // The fold is communicated entirely through the outline — two extra
    // stroke segments inside the bounding box tracing the diagonal crease.
    let fold_outline = Path::new()
        .set(
            "d",
            format!(
                "M{},{} L{},{} L{},{}",
                x + w - fold,
                y,
                x + w - fold,
                y + fold,
                x + w,
                y + fold,
            ),
        )
        .set("fill", "none")
        .set("stroke", stroke)
        .set("stroke-width", "1.2");
    Group::new().add(body).add(fold_outline)
}

/// Deployment node: 3D perspective box — front face plus two skewed
/// parallelograms suggesting the top and right sides.
fn node3d_shape(x: f64, y: f64, w: f64, h: f64, fill: &str, stroke: &str) -> Group {
    // Depth offset shrinks with smaller nodes to stop the perspective from
    // crowding the text area.
    let depth = (w.min(h) * 0.12).clamp(6.0, 14.0);
    let front = Rectangle::new()
        .set("x", x)
        .set("y", y + depth)
        .set("width", w - depth)
        .set("height", h - depth)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    let top = Polygon::new()
        .set(
            "points",
            format!(
                "{},{} {},{} {},{} {},{}",
                x,
                y + depth,
                x + depth,
                y,
                x + w,
                y,
                x + w - depth,
                y + depth,
            ),
        )
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    let right = Polygon::new()
        .set(
            "points",
            format!(
                "{},{} {},{} {},{} {},{}",
                x + w - depth,
                y + depth,
                x + w,
                y,
                x + w,
                y + h - depth,
                x + w - depth,
                y + h,
            ),
        )
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", "1.5");
    Group::new().add(top).add(right).add(front)
}

fn render_node(node: &NodeLayout, y_off: f64) -> Group {
    let x = node.x;
    let y = node.y + y_off;
    let w = node.width;

    // Strokes only — shapes are outline-only now. Picked so each kind stays
    // distinguishable and the mid-tone colours read on both light and dark
    // canvases. Too-dark greys (#444, #555, #666) bumped to #888-ish so
    // they stay visible against the dark-mode background.
    let stroke = match node.kind {
        ClassKind::Interface => "#6666bb",
        ClassKind::Abstract => "#bb6666",
        ClassKind::Enum => "#66bb66",
        ClassKind::Object => "#b8a85a",
        ClassKind::Component => "#6c8ebf",
        ClassKind::Node => "#888888",
        ClassKind::Cloud => "#8da0b0",
        ClassKind::Database => "#a06ba0",
        ClassKind::Folder => "#c89b2e",
        ClassKind::Frame => "#888888",
        ClassKind::Rectangle => "#888888",
        ClassKind::Artifact => "#a68862",
        ClassKind::Queue => "#5a92c4",
        _ => "#6c8ebf",
    };
    // Legacy fill/header_fill bindings — kept as placeholders because
    // draw_shape still accepts them. All shapes currently ignore them and
    // render as outline-only.
    let fill = "none";
    let header_fill = "none";

    let mut g = draw_shape(node, y, fill, stroke, header_fill);

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
    if edge.points.is_empty() {
        return Group::new();
    }

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

    let d = path_from_points(&edge.points, y_off);
    let path = Path::new()
        .set("d", d)
        .set("class", stroke_class)
        .set("marker-end", marker_end);

    let mut g = Group::new().add(path);

    // Labels: the main edge label sits at the path's midpoint; from/to
    // labels sit near the respective endpoints. Orthogonal paths may have
    // multiple bends so we compute the geometric midpoint along the
    // polyline, which lands on (or near) the middle waypoint for typical
    // 3/4-point routes.
    let (first_x, first_y) = edge.points.first().copied().unwrap_or((0.0, 0.0));
    let (last_x, last_y) = edge.points.last().copied().unwrap_or((0.0, 0.0));

    if let Some(ref lbl) = edge.label {
        let (lx, ly, anchor) = label_perpendicular(&edge.points, 8.0);
        let t = Text::new()
            .set("x", lx)
            .set("y", ly + y_off)
            .set("text-anchor", anchor)
            .set("font-size", "11")
            .add(text_node(lbl.clone()));
        g = g.add(t);
    }

    if let Some(ref lbl) = edge.from_label {
        let t = Text::new()
            .set("x", first_x + 6.0)
            .set("y", first_y + y_off - 4.0)
            .set("font-size", "11")
            .add(text_node(lbl.clone()));
        g = g.add(t);
    }

    if let Some(ref lbl) = edge.to_label {
        let t = Text::new()
            .set("x", last_x + 6.0)
            .set("y", last_y + y_off - 4.0)
            .set("font-size", "11")
            .add(text_node(lbl.clone()));
        g = g.add(t);
    }

    g
}

/// Build an SVG `d` attribute from a polyline. First point is `M`, the rest
/// are `L` segments; `y_off` lifts the whole thing to account for the
/// diagram title offset.
fn path_from_points(points: &[(f64, f64)], y_off: f64) -> String {
    let mut d = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        let cmd = if i == 0 { "M" } else { " L" };
        d.push_str(&format!("{}{},{}", cmd, x, y + y_off));
    }
    d
}
