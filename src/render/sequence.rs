use svg::node::element::{Circle, Group, Line, Rectangle, Text};
use svg::Document;

use crate::ast::sequence::ParticipantKind;
use crate::layout::sequence::{
    ActivationLayout, DividerLayout, GroupLayout, LayoutElement, MessageLayout, NoteLayout,
    ParticipantLayout, SequenceLayout,
};

use super::primitives::{arrowhead_defs, background_rect, style_block, text_node};
use super::theme::Theme;

const PARTICIPANT_HEIGHT: f64 = 40.0;
const PARTICIPANT_WIDTH: f64 = 120.0;
const ACTIVATION_WIDTH: f64 = 10.0;
const FONT_SIZE: f64 = 13.0;
const TOP_MARGIN: f64 = 20.0;
const TITLE_HEIGHT: f64 = 30.0;

pub fn render(layout: &SequenceLayout, theme: &Theme) -> Document {
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

    doc = doc.add(arrowhead_defs());
    doc = doc.add(style_block(theme));

    doc = doc.add(background_rect(theme));

    // Title
    if let Some(ref t) = layout.title {
        let title_el = Text::new()
            .set("x", layout.total_width / 2.0)
            .set("y", TOP_MARGIN + 5.0)
            .set("text-anchor", "middle")
            .set("class", "title")
            .add(text_node(t.clone()));
        doc = doc.add(title_el);
    }

    let y_off = title_offset;

    // Header participant boxes
    for p in &layout.participants {
        doc = doc.add(participant_box(p, y_off + TOP_MARGIN, false));
    }

    // Lifelines
    let lifeline_start = y_off + TOP_MARGIN + PARTICIPANT_HEIGHT;
    for p in &layout.participants {
        let line = Line::new()
            .set("x1", p.x)
            .set("y1", lifeline_start)
            .set("x2", p.x)
            .set("y2", y_off + layout.lifeline_height)
            .set("class", "lifeline");
        doc = doc.add(line);
    }

    // Draw group containers first so they sit below messages/notes.
    for elem in &layout.elements {
        if let LayoutElement::Group(g) = elem {
            doc = doc.add(render_group(g, y_off));
        }
    }

    for elem in &layout.elements {
        match elem {
            LayoutElement::Message(m) => {
                doc = doc.add(render_message(m, y_off));
            }
            LayoutElement::Note(n) => {
                doc = doc.add(render_note(n, y_off));
            }
            LayoutElement::Activation(a) => {
                doc = doc.add(render_activation(a, y_off));
            }
            LayoutElement::Divider(d) => {
                doc = doc.add(render_divider(d, y_off));
            }
            LayoutElement::Group(_) => {}
        }
    }

    // Footer participant boxes (unless hidden)
    if !layout.hide_footbox {
        let footer_y = y_off + layout.lifeline_height;
        for p in &layout.participants {
            doc = doc.add(participant_box(p, footer_y, true));
        }
    }

    doc
}

fn participant_box(p: &ParticipantLayout, y: f64, _is_footer: bool) -> Group {
    let x = p.x - PARTICIPANT_WIDTH / 2.0;
    let color = p.color.as_deref().unwrap_or("#dae8fc");
    let border = "#6c8ebf";

    match p.kind {
        ParticipantKind::Actor => render_actor(p, y),
        _ => {
            let rect = Rectangle::new()
                .set("x", x)
                .set("y", y)
                .set("width", PARTICIPANT_WIDTH)
                .set("height", PARTICIPANT_HEIGHT)
                .set("rx", "4")
                .set("fill", color)
                .set("stroke", border)
                .set("stroke-width", "1.5");
            let text = Text::new()
                .set("x", p.x)
                .set("y", y + PARTICIPANT_HEIGHT / 2.0 + FONT_SIZE / 2.0 - 2.0)
                .set("text-anchor", "middle")
                .add(text_node(p.display.clone()));
            Group::new().add(rect).add(text)
        }
    }
}

fn render_actor(p: &ParticipantLayout, y: f64) -> Group {
    // Stick figure: head circle + body lines
    let cx = p.x;
    let head_r = 8.0;
    let head_cy = y + head_r + 2.0;
    let body_top = head_cy + head_r;
    let body_bot = y + PARTICIPANT_HEIGHT - 10.0;
    let arm_y = body_top + (body_bot - body_top) * 0.4;

    let head = Circle::new()
        .set("cx", cx)
        .set("cy", head_cy)
        .set("r", head_r)
        .set("fill", "#dae8fc")
        .set("stroke", "#6c8ebf")
        .set("stroke-width", "1.5");
    let body = Line::new()
        .set("x1", cx)
        .set("y1", body_top)
        .set("x2", cx)
        .set("y2", body_bot)
        .set("stroke", "#6c8ebf")
        .set("stroke-width", "1.5");
    let arms = Line::new()
        .set("x1", cx - 12.0)
        .set("y1", arm_y)
        .set("x2", cx + 12.0)
        .set("y2", arm_y)
        .set("stroke", "#6c8ebf")
        .set("stroke-width", "1.5");
    let leg_l = Line::new()
        .set("x1", cx)
        .set("y1", body_bot)
        .set("x2", cx - 10.0)
        .set("y2", y + PARTICIPANT_HEIGHT)
        .set("stroke", "#6c8ebf")
        .set("stroke-width", "1.5");
    let leg_r = Line::new()
        .set("x1", cx)
        .set("y1", body_bot)
        .set("x2", cx + 10.0)
        .set("y2", y + PARTICIPANT_HEIGHT)
        .set("stroke", "#6c8ebf")
        .set("stroke-width", "1.5");
    let label = Text::new()
        .set("x", cx)
        .set("y", y + PARTICIPANT_HEIGHT + FONT_SIZE)
        .set("text-anchor", "middle")
        .add(text_node(p.display.clone()));

    Group::new()
        .add(head)
        .add(body)
        .add(arms)
        .add(leg_l)
        .add(leg_r)
        .add(label)
}

fn render_message(m: &MessageLayout, y_off: f64) -> Group {
    let y = m.y + y_off;
    let label_y = y - 5.0;

    let (class, marker) = if m.dashed {
        ("arrow-dashed", "url(#arrowhead)")
    } else {
        ("arrow", "url(#arrowhead)")
    };

    if m.self_msg {
        // Self-message: right-angle path looping right
        let x = m.from_x;
        let loop_w = 30.0;
        let path = svg::node::element::Path::new()
            .set(
                "d",
                format!(
                    "M {x} {y} L {} {y} L {} {} L {x} {}",
                    x + loop_w,
                    x + loop_w,
                    y + 20.0,
                    y + 20.0
                ),
            )
            .set("class", class)
            .set("marker-end", marker);
        let label = Text::new()
            .set("x", x + loop_w + 4.0)
            .set("y", y + 10.0)
            .add(text_node(m.label.clone()));
        return Group::new().add(path).add(label);
    }

    let line = Line::new()
        .set("x1", m.from_x)
        .set("y1", y)
        .set("x2", m.to_x)
        .set("y2", y)
        .set("class", class)
        .set("marker-end", marker);

    let text_x = (m.from_x + m.to_x) / 2.0;
    let label = Text::new()
        .set("x", text_x)
        .set("y", label_y)
        .set("text-anchor", "middle")
        .add(text_node(m.label.clone()));

    Group::new().add(line).add(label)
}

fn render_note(n: &NoteLayout, y_off: f64) -> Group {
    let y = n.y + y_off;
    let fold = 10.0;
    // Folded-corner rectangle
    let pts = format!(
        "{},{} {},{} {},{} {},{} {},{}",
        n.x,
        y,
        n.x + n.width - fold,
        y,
        n.x + n.width,
        y + fold,
        n.x + n.width,
        y + n.height,
        n.x,
        y + n.height
    );
    let body = svg::node::element::Polygon::new()
        .set("points", pts)
        .set("class", "note-box");
    let fold_line = svg::node::element::Polyline::new()
        .set(
            "points",
            format!(
                "{},{} {},{} {},{}",
                n.x + n.width - fold,
                y,
                n.x + n.width - fold,
                y + fold,
                n.x + n.width,
                y + fold
            ),
        )
        .set("fill", "none")
        .set("stroke", "#bbbb00")
        .set("stroke-width", "1");

    let mut g = Group::new().add(body).add(fold_line);
    for (i, line) in n.lines.iter().enumerate() {
        let ty = y + 16.0 + i as f64 * 17.0;
        let t = Text::new()
            .set("x", n.x + 6.0)
            .set("y", ty)
            .add(text_node(line.clone()));
        g = g.add(t);
    }
    g
}

fn render_activation(a: &ActivationLayout, y_off: f64) -> Group {
    let x = a.participant_x - ACTIVATION_WIDTH / 2.0 + a.depth as f64 * 3.0;
    let rect = Rectangle::new()
        .set("x", x)
        .set("y", a.y_start + y_off)
        .set("width", ACTIVATION_WIDTH)
        .set("height", (a.y_end - a.y_start).max(4.0))
        .set("class", "activation");
    Group::new().add(rect)
}

fn render_group(g: &GroupLayout, y_off: f64) -> Group {
    let y = g.y + y_off;
    let kind_display = g.kind.to_lowercase();
    let tab_w = (kind_display.len() as f64 * 8.0 + 16.0).max(40.0);
    let tab_h = 16.0;

    let frame = Rectangle::new()
        .set("x", g.x)
        .set("y", y)
        .set("width", g.width)
        .set("height", g.height)
        .set("fill", "none")
        .set("stroke", "#888")
        .set("stroke-width", "1.2");

    let tab = svg::node::element::Polygon::new()
        .set(
            "points",
            format!(
                "{},{} {},{} {},{} {},{} {},{}",
                g.x,
                y,
                g.x + tab_w,
                y,
                g.x + tab_w + tab_h * 0.5,
                y + tab_h * 0.5,
                g.x + tab_w,
                y + tab_h,
                g.x,
                y + tab_h
            ),
        )
        .set("fill", "#eeeeee")
        .set("stroke", "#888")
        .set("stroke-width", "1.2");

    let tab_label = Text::new()
        .set("x", g.x + 8.0)
        .set("y", y + tab_h - 4.0)
        .set("font-weight", "bold")
        .set("font-size", FONT_SIZE - 2.0)
        .add(text_node(kind_display));

    let mut grp = Group::new().add(frame).add(tab).add(tab_label);

    if !g.label.is_empty() {
        let label = Text::new()
            .set("x", g.x + tab_w + tab_h * 0.5 + 8.0)
            .set("y", y + tab_h - 4.0)
            .set("font-size", FONT_SIZE - 1.0)
            .add(text_node(format!("[{}]", g.label)));
        grp = grp.add(label);
    }

    for (break_y, section_label) in &g.section_breaks {
        let by = break_y + y_off;
        let line = Line::new()
            .set("x1", g.x)
            .set("y1", by)
            .set("x2", g.x + g.width)
            .set("y2", by)
            .set("stroke", "#888")
            .set("stroke-width", "1")
            .set("stroke-dasharray", "4,3");
        grp = grp.add(line);
        if let Some(lbl) = section_label {
            let t = Text::new()
                .set("x", g.x + 8.0)
                .set("y", by + 13.0)
                .set("font-size", FONT_SIZE - 1.0)
                .set("font-weight", "bold")
                .add(text_node(format!("[{}]", lbl)));
            grp = grp.add(t);
        }
    }

    grp
}

fn render_divider(d: &DividerLayout, y_off: f64) -> Group {
    let y = d.y + y_off;
    let cx = d.total_width / 2.0;
    let line = Line::new()
        .set("x1", 0)
        .set("y1", y)
        .set("x2", d.total_width)
        .set("y2", y)
        .set("class", "divider-line");
    let label = Text::new()
        .set("x", cx)
        .set("y", y - 4.0)
        .set("text-anchor", "middle")
        .set("class", "divider-label")
        .add(text_node(d.label.clone()));
    Group::new().add(line).add(label)
}
