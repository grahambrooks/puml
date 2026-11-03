use svg::Document;
use svg::node::element::{Circle, Group, Line, Rectangle, Text};

use crate::ast::sequence::ParticipantKind;
use crate::layout::sequence::{
    ActivationLayout, BoxLayout, DividerLayout, GroupLayout, LABEL_BASELINE, LINE_HEIGHT,
    LayoutElement, MessageLayout, NoteLayout, ParticipantLayout, SequenceLayout, lines_width,
};

use super::primitives::{arrowhead_defs, background_rect, style_block};
use super::theme::Theme;

const PARTICIPANT_WIDTH: f64 = 120.0;
const ACTIVATION_WIDTH: f64 = 10.0;
const FONT_SIZE: f64 = 13.0;
const TOP_MARGIN: f64 = 20.0;
const TITLE_HEIGHT: f64 = 30.0;
const SELF_LOOP_MIN_W: f64 = 30.0;
const SELF_LOOP_H: f64 = 20.0;
/// The stick figure's height; an actor's name goes underneath it.
const ACTOR_HEIGHT: f64 = 40.0;

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
        let title_el = Text::new(t.clone())
            .set("x", layout.total_width / 2.0)
            .set("y", TOP_MARGIN + 5.0)
            .set("text-anchor", "middle")
            .set("class", "title");
        doc = doc.add(title_el);
    }

    let y_off = title_offset;

    // Participant boxes (`box` … `end box`) sit behind everything else.
    for b in &layout.boxes {
        doc = doc.add(render_box(b, y_off));
    }

    // Header participant boxes
    let header_top = y_off + TOP_MARGIN + layout.header_offset;
    for p in &layout.participants {
        doc = doc.add(participant_box(p, header_top, layout.participant_height));
    }

    // Lifelines
    let lifeline_start = header_top + layout.participant_height;
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
            doc = doc.add(participant_box(p, footer_y, layout.participant_height));
        }
    }

    doc
}

/// An SVG paint for a PlantUML colour: `#rgb`/`#rrggbb` stay as they are,
/// `#Name` becomes the CSS colour keyword `Name`.
fn svg_color(c: &str) -> String {
    let body = c.trim_start_matches('#');
    let hex = matches!(body.len(), 3 | 6) && body.chars().all(|ch| ch.is_ascii_hexdigit());
    if hex {
        format!("#{body}")
    } else {
        body.to_string()
    }
}

fn render_box(b: &BoxLayout, y_off: f64) -> Group {
    let y = b.y + y_off;
    let mut rect = Rectangle::new()
        .set("x", b.x)
        .set("y", y)
        .set("width", b.width)
        .set("height", b.height)
        .set("rx", "3")
        .set("stroke", "#888888")
        .set("stroke-width", "1");
    // An explicit colour is drawn as given; the default is a translucent
    // grey that reads on light and dark backgrounds alike.
    rect = match &b.color {
        Some(c) => rect.set("fill", svg_color(c)),
        None => rect.set("fill", "#888888").set("fill-opacity", "0.15"),
    };
    let mut g = Group::new().set("class", "participant-group").add(rect);
    if let Some(t) = &b.title {
        g = g.add(
            Text::new(t.clone())
                .set("x", b.x + b.width / 2.0)
                .set("y", y + 15.0)
                .set("text-anchor", "middle")
                .set("font-weight", "bold"),
        );
    }
    g
}

/// `lines` as stacked `<text>` elements at `x`, the first baseline at
/// `first_baseline` and each next one `LINE_HEIGHT` below it.
fn text_lines(lines: &[String], x: f64, first_baseline: f64, anchor: &str) -> Vec<Text> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            Text::new(line.clone())
                .set("x", x)
                .set("y", first_baseline + i as f64 * LINE_HEIGHT)
                .set("text-anchor", anchor)
        })
        .collect()
}

fn participant_box(p: &ParticipantLayout, y: f64, height: f64) -> Group {
    let x = p.x - PARTICIPANT_WIDTH / 2.0;
    // Per-participant colour (from `skinparam`/`color` in the source) still
    // overrides the default — users who ask for a coloured box get one. The
    // default is transparent so the box reads on any background.
    let fill = p.color.as_deref().unwrap_or("none");
    let border = "#6c8ebf";

    match p.kind {
        ParticipantKind::Actor => render_actor(p, y),
        _ => {
            let rect = Rectangle::new()
                .set("x", x)
                .set("y", y)
                .set("width", PARTICIPANT_WIDTH)
                .set("height", height)
                .set("rx", "4")
                .set("fill", fill)
                .set("stroke", border)
                .set("stroke-width", "1.5");
            // The block of lines is centred vertically in the box; one
            // line sits where it always has.
            let n = p.display.len().max(1) as f64;
            let first = y + height / 2.0 + FONT_SIZE / 2.0 - 2.0 - (n - 1.0) * LINE_HEIGHT / 2.0;
            text_lines(&p.display, p.x, first, "middle")
                .into_iter()
                .fold(Group::new().add(rect), |g, t| g.add(t))
        }
    }
}

fn render_actor(p: &ParticipantLayout, y: f64) -> Group {
    // Stick figure: head circle + body lines
    let cx = p.x;
    let head_r = 8.0;
    let head_cy = y + head_r + 2.0;
    let body_top = head_cy + head_r;
    let body_bot = y + ACTOR_HEIGHT - 10.0;
    let arm_y = body_top + (body_bot - body_top) * 0.4;

    let head = Circle::new()
        .set("cx", cx)
        .set("cy", head_cy)
        .set("r", head_r)
        .set("fill", "none")
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
        .set("y2", y + ACTOR_HEIGHT)
        .set("stroke", "#6c8ebf")
        .set("stroke-width", "1.5");
    let leg_r = Line::new()
        .set("x1", cx)
        .set("y1", body_bot)
        .set("x2", cx + 10.0)
        .set("y2", y + ACTOR_HEIGHT)
        .set("stroke", "#6c8ebf")
        .set("stroke-width", "1.5");
    let figure = Group::new()
        .add(head)
        .add(body)
        .add(arms)
        .add(leg_l)
        .add(leg_r);
    text_lines(&p.display, cx, y + ACTOR_HEIGHT + FONT_SIZE, "middle")
        .into_iter()
        .fold(figure, |g, t| g.add(t))
}

/// Width of the right-angle loop arc for a self-message — sized so the
/// label fits comfortably inside the loop's horizontal extent. The same
/// formula is used in `layout::sequence::widen_for_messages` so the
/// participant column is always wide enough to hold the loop without
/// bleeding into the next column.
pub fn self_loop_width(lines: &[String]) -> f64 {
    (lines_width(lines) + 16.0).max(SELF_LOOP_MIN_W)
}

fn render_message(m: &MessageLayout, y_off: f64) -> Group {
    let y = m.y + y_off;
    // The last line's baseline sits clear above the arrow; earlier lines
    // stack upwards from it.
    let label_y = y - LABEL_BASELINE - m.lines.len().saturating_sub(1) as f64 * LINE_HEIGHT;
    // An unlabelled message keeps its (empty) text element.
    let no_label = [String::new()];
    let lines: &[String] = if m.lines.is_empty() {
        &no_label
    } else {
        &m.lines
    };

    let (class, marker) = if m.dashed {
        ("arrow-dashed", "url(#arrowhead)")
    } else {
        ("arrow", "url(#arrowhead)")
    };

    if m.self_msg {
        // Self-message: right-angle path looping right. Loop width grows
        // with the label so a long message still has somewhere to put its
        // text without crashing into the next participant's lifeline.
        // Layout's `widen_for_messages` widens the source participant's
        // column with the same formula, so the loop always fits inside
        // its column with room to spare.
        let x = m.from_x;
        let loop_w = self_loop_width(&m.lines);
        let path = svg::node::element::Path::new()
            .set(
                "d",
                format!(
                    "M {x} {y} L {} {y} L {} {} L {x} {}",
                    x + loop_w,
                    x + loop_w,
                    y + SELF_LOOP_H,
                    y + SELF_LOOP_H
                ),
            )
            .set("class", class)
            .set("marker-end", marker);
        // Label centred above the top arc — same convention as a
        // cross-participant message label, just relative to the loop's
        // bounding box instead of the line itself.
        return text_lines(lines, x + loop_w / 2.0, label_y, "middle")
            .into_iter()
            .fold(Group::new().add(path), |g, t| g.add(t));
    }

    let line = Line::new()
        .set("x1", m.from_x)
        .set("y1", y)
        .set("x2", m.to_x)
        .set("y2", y)
        .set("class", class)
        .set("marker-end", marker);

    let text_x = (m.from_x + m.to_x) / 2.0;
    text_lines(lines, text_x, label_y, "middle")
        .into_iter()
        .fold(Group::new().add(line), |g, t| g.add(t))
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
        let t = Text::new(line.clone()).set("x", n.x + 6.0).set("y", ty);
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
        .set("fill", "none")
        .set("stroke", "#888888")
        .set("stroke-width", "1.2");

    let tab_label = Text::new(kind_display)
        .set("x", g.x + 8.0)
        .set("y", y + tab_h - 4.0)
        .set("font-weight", "bold")
        .set("font-size", FONT_SIZE - 2.0);

    let mut grp = Group::new().add(frame).add(tab).add(tab_label);

    if !g.label.is_empty() {
        let label = Text::new(format!("[{}]", g.label))
            .set("x", g.x + tab_w + tab_h * 0.5 + 8.0)
            .set("y", y + tab_h - 4.0)
            .set("font-size", FONT_SIZE - 1.0);
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
            let t = Text::new(format!("[{}]", lbl))
                .set("x", g.x + 8.0)
                .set("y", by + 13.0)
                .set("font-size", FONT_SIZE - 1.0)
                .set("font-weight", "bold");
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
    let label = Text::new(d.label.clone())
        .set("x", cx)
        .set("y", y - 4.0)
        .set("text-anchor", "middle")
        .set("class", "divider-label");
    Group::new().add(line).add(label)
}
