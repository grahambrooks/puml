use crate::ast::sequence::*;

const PARTICIPANT_WIDTH: f64 = 120.0;
const PARTICIPANT_HEIGHT: f64 = 40.0;
const PARTICIPANT_H_PADDING: f64 = 30.0; // horizontal gap between participants
const ROW_HEIGHT: f64 = 40.0;
const TOP_MARGIN: f64 = 20.0;
const SIDE_MARGIN: f64 = 20.0;
const FONT_SIZE: f64 = 13.0;
const CHAR_WIDTH: f64 = 7.5; // approximate character width at default font size

pub struct ParticipantLayout {
    #[allow(dead_code)]
    pub name: String,
    pub display: String,
    pub x: f64,
    pub kind: ParticipantKind,
    pub color: Option<String>,
}

pub struct MessageLayout {
    pub from_x: f64,
    pub to_x: f64,
    pub y: f64,
    pub label: String,
    pub dashed: bool,
    pub self_msg: bool,
    #[allow(dead_code)]
    pub lost: bool,
}

pub struct NoteLayout {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub lines: Vec<String>,
}

pub struct ActivationLayout {
    pub participant_x: f64,
    pub y_start: f64,
    pub y_end: f64,
    pub depth: u32,
}

pub struct DividerLayout {
    pub y: f64,
    pub label: String,
    pub total_width: f64,
}

pub struct GroupLayout {
    pub kind: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// y positions of section dividers (else/also breaks) relative to diagram origin
    pub section_breaks: Vec<(f64, Option<String>)>,
}

/// A participant box (`box` … `end box`): a panel behind its members'
/// columns, from above the header row to below the footer row. `y` and
/// `height` are in the same coordinates as the other elements.
pub struct BoxLayout {
    pub title: Option<String>,
    pub color: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub enum LayoutElement {
    Message(MessageLayout),
    Note(NoteLayout),
    Activation(ActivationLayout),
    Divider(DividerLayout),
    Group(GroupLayout),
}

pub struct SequenceLayout {
    pub participants: Vec<ParticipantLayout>,
    pub elements: Vec<LayoutElement>,
    pub lifeline_height: f64,
    pub total_width: f64,
    pub total_height: f64,
    pub title: Option<String>,
    pub hide_footbox: bool,
    pub boxes: Vec<BoxLayout>,
    /// Extra space above the participant header row, for box titles. The
    /// header row's top is `TOP_MARGIN + header_offset`.
    pub header_offset: f64,
}

/// How far a box extends beyond its outermost participant boxes.
const BOX_PAD: f64 = 8.0;
/// The row a box title takes above the participant headers.
const BOX_TITLE_H: f64 = 20.0;
const BOX_TOP: f64 = 6.0;

fn text_width(s: &str) -> f64 {
    s.len() as f64 * CHAR_WIDTH
}

/// Display name shown inside the participant header box. PlantUML's
/// convention is `participant "Display Name" as ALIAS` — the quoted name
/// is the human-readable label, and the alias is just a short referent
/// for messages. So always show `name`, falling back to alias only if
/// the parser produced an aliased entry with no canonical name (rare but
/// possible from forward declarations).
fn participant_display(p: &Participant) -> String {
    if !p.name.is_empty() {
        p.name.clone()
    } else {
        p.alias.clone().unwrap_or_default()
    }
}

pub fn layout(diagram: &SequenceDiagram) -> SequenceLayout {
    let participants = diagram.ordered_participants();

    let n = participants.len();
    let mut col_widths: Vec<f64> = participants
        .iter()
        .map(|p| {
            let w = text_width(&participant_display(p)) + 20.0;
            w.max(PARTICIPANT_WIDTH)
        })
        .collect();

    widen_for_messages(&participants, &diagram.elements, &mut col_widths);

    // Member columns of each box, as (first, last) indices.
    let box_spans: Vec<Option<(usize, usize)>> = diagram
        .boxes
        .iter()
        .map(|b| {
            let idx: Vec<usize> = b
                .participants
                .iter()
                .filter_map(|name| find_participant(&participants, name))
                .collect();
            Some((*idx.iter().min()?, *idx.iter().max()?))
        })
        .collect();
    widen_for_box_titles(&diagram.boxes, &box_spans, &mut col_widths);
    let header_offset = if diagram
        .boxes
        .iter()
        .zip(&box_spans)
        .any(|(b, s)| b.title.is_some() && s.is_some())
    {
        BOX_TITLE_H
    } else {
        0.0
    };

    let mut centers: Vec<f64> = Vec::with_capacity(n);
    let mut x = SIDE_MARGIN;
    for (i, w) in col_widths.iter().enumerate() {
        centers.push(x + w / 2.0);
        x += w;
        if i + 1 < n {
            x += PARTICIPANT_H_PADDING;
        }
    }
    let total_width = x + SIDE_MARGIN;

    let participant_layouts: Vec<ParticipantLayout> = participants
        .iter()
        .enumerate()
        .map(|(i, p)| ParticipantLayout {
            name: p.alias.clone().unwrap_or_else(|| p.name.clone()),
            display: participant_display(p),
            x: centers[i],
            kind: p.kind.clone(),
            color: p.color.clone(),
        })
        .collect();

    let mut ctx = LayoutCtx {
        participants: &participants,
        centers: &centers,
        total_width,
        elements: Vec::new(),
        active_depth: vec![0; n],
        open_activations: Vec::new(),
        y: header_offset + TOP_MARGIN + PARTICIPANT_HEIGHT + 10.0,
        autonumber: None,
    };

    layout_elements(&mut ctx, &diagram.elements);

    // Close any still-open activations
    let y_end = ctx.y;
    let remaining: Vec<(usize, f64)> = std::mem::take(&mut ctx.open_activations);
    for (i, y_start) in remaining {
        ctx.elements
            .push(LayoutElement::Activation(ActivationLayout {
                participant_x: centers[i],
                y_start,
                y_end,
                depth: 0,
            }));
    }

    let lifeline_height = ctx.y + 10.0;
    let footer_h = if diagram.hide_footbox {
        0.0
    } else {
        PARTICIPANT_HEIGHT + 10.0
    };
    let total_height = lifeline_height + footer_h + TOP_MARGIN;

    let box_bottom = lifeline_height
        + if diagram.hide_footbox {
            0.0
        } else {
            PARTICIPANT_HEIGHT
        }
        + BOX_PAD;
    let boxes = diagram
        .boxes
        .iter()
        .zip(&box_spans)
        .filter_map(|(b, span)| {
            let (lo, hi) = (*span)?;
            let x = centers[lo] - PARTICIPANT_WIDTH / 2.0 - BOX_PAD;
            let right = centers[hi] + PARTICIPANT_WIDTH / 2.0 + BOX_PAD;
            Some(BoxLayout {
                title: b.title.clone(),
                color: b.color.clone(),
                x,
                y: BOX_TOP,
                width: right - x,
                height: box_bottom - BOX_TOP,
            })
        })
        .collect();

    SequenceLayout {
        participants: participant_layouts,
        elements: ctx.elements,
        lifeline_height,
        total_width,
        total_height,
        title: diagram.title.clone(),
        hide_footbox: diagram.hide_footbox,
        boxes,
        header_offset,
    }
}

/// Widens a box's member columns so its title fits inside it.
fn widen_for_box_titles(
    boxes: &[ParticipantBox],
    spans: &[Option<(usize, usize)>],
    col_widths: &mut [f64],
) {
    for (b, span) in boxes.iter().zip(spans) {
        let (Some(title), Some((lo, hi))) = (&b.title, span) else {
            continue;
        };
        let (lo, hi) = (*lo, *hi);
        // The box spans the outer participant boxes plus its padding.
        let inner = centers_span(col_widths, lo, hi) + PARTICIPANT_WIDTH + 2.0 * BOX_PAD;
        let needed = text_width(title) + 20.0;
        if needed > inner {
            let extra = (needed - inner) / (hi - lo + 1) as f64;
            for w in &mut col_widths[lo..=hi] {
                *w += extra;
            }
        }
    }
}

/// Distance between the centres of columns `lo` and `hi`.
fn centers_span(col_widths: &[f64], lo: usize, hi: usize) -> f64 {
    if lo == hi {
        return 0.0;
    }
    col_widths[lo] / 2.0
        + col_widths[lo + 1..hi].iter().sum::<f64>()
        + col_widths[hi] / 2.0
        + PARTICIPANT_H_PADDING * (hi - lo) as f64
}

struct LayoutCtx<'a> {
    participants: &'a [&'a Participant],
    centers: &'a [f64],
    total_width: f64,
    elements: Vec<LayoutElement>,
    active_depth: Vec<u32>,
    open_activations: Vec<(usize, f64)>,
    y: f64,
    /// When Some, messages receive an autonumber prefix and the counter advances.
    autonumber: Option<u32>,
}

fn layout_elements(ctx: &mut LayoutCtx, elements: &[SequenceElement]) {
    for elem in elements {
        layout_one(ctx, elem);
    }
}

fn layout_one(ctx: &mut LayoutCtx, elem: &SequenceElement) {
    match elem {
        SequenceElement::Message(msg) => {
            let fi = find_participant(ctx.participants, &msg.from);
            let ti = find_participant(ctx.participants, &msg.to);
            let (from_x, to_x) = match (fi, ti) {
                (Some(f), Some(t)) => (ctx.centers[f], ctx.centers[t]),
                (Some(f), None) => (ctx.centers[f], ctx.centers[f] + 60.0),
                _ => (SIDE_MARGIN + 60.0, SIDE_MARGIN + 180.0),
            };
            let dashed = matches!(msg.arrow, ArrowStyle::Dashed);
            let self_msg = fi == ti;

            let mut label = msg.label.clone();
            if let Some(ref mut n) = ctx.autonumber {
                let prefix = format!("{}: ", n);
                label = if label.is_empty() {
                    prefix.trim_end().to_string()
                } else {
                    format!("{}{}", prefix, label)
                };
                *n += 1;
            }

            ctx.elements.push(LayoutElement::Message(MessageLayout {
                from_x,
                to_x,
                y: ctx.y,
                label,
                dashed,
                self_msg,
                lost: matches!(msg.arrow, ArrowStyle::Lost),
            }));
            // Self-messages occupy ~20px more vertical space than a
            // straight cross-participant arrow because of the loop's
            // bottom arc; without the bump the next message's label can
            // sit on top of the loop's lower edge.
            ctx.y += if self_msg {
                ROW_HEIGHT + 16.0
            } else {
                ROW_HEIGHT
            };
        }
        SequenceElement::Note(note) => {
            let anchor_x = note
                .participants
                .first()
                .and_then(|name| find_participant(ctx.participants, name).map(|i| ctx.centers[i]))
                .unwrap_or(SIDE_MARGIN + 60.0);
            let note_w = note
                .lines
                .iter()
                .map(|l| text_width(l))
                .fold(120.0_f64, f64::max)
                + 20.0;
            let note_h = note.lines.len() as f64 * (FONT_SIZE + 4.0) + 12.0;
            let note_x = match note.position {
                NotePosition::Left => anchor_x - note_w - 10.0,
                NotePosition::Right => anchor_x + 10.0,
                NotePosition::Over => anchor_x - note_w / 2.0,
            };
            ctx.elements.push(LayoutElement::Note(NoteLayout {
                x: note_x,
                y: ctx.y,
                width: note_w,
                height: note_h,
                lines: note.lines.clone(),
            }));
            ctx.y += note_h + 10.0;
        }
        SequenceElement::Activate(name) => {
            if let Some(i) = find_participant(ctx.participants, name) {
                ctx.open_activations.push((i, ctx.y));
                ctx.active_depth[i] += 1;
            }
        }
        SequenceElement::Deactivate(name) => {
            if let Some(i) = find_participant(ctx.participants, name) {
                if ctx.active_depth[i] > 0 {
                    ctx.active_depth[i] -= 1;
                    if let Some(pos) = ctx.open_activations.iter().rposition(|(idx, _)| *idx == i) {
                        let (_, y_start) = ctx.open_activations.remove(pos);
                        let depth = ctx.active_depth[i];
                        let cx = ctx.centers[i];
                        ctx.elements
                            .push(LayoutElement::Activation(ActivationLayout {
                                participant_x: cx,
                                y_start,
                                y_end: ctx.y,
                                depth,
                            }));
                    }
                }
            }
        }
        SequenceElement::Divider(div) => {
            ctx.elements.push(LayoutElement::Divider(DividerLayout {
                y: ctx.y,
                label: div.label.clone(),
                total_width: ctx.total_width,
            }));
            ctx.y += ROW_HEIGHT;
        }
        SequenceElement::Space(px) => {
            ctx.y += *px as f64;
        }
        SequenceElement::Delay(label) => {
            ctx.elements.push(LayoutElement::Divider(DividerLayout {
                y: ctx.y,
                label: label.clone(),
                total_width: ctx.total_width,
            }));
            ctx.y += ROW_HEIGHT / 2.0;
        }
        SequenceElement::Autonumber(start) => {
            ctx.autonumber = Some(start.unwrap_or(1));
        }
        SequenceElement::Group(group) => {
            layout_group(ctx, group);
        }
    }
}

const GROUP_HEADER_H: f64 = 20.0;
const GROUP_PAD_Y: f64 = 8.0;

fn layout_group(ctx: &mut LayoutCtx, group: &GroupBlock) {
    let y_start = ctx.y;
    ctx.y += GROUP_HEADER_H + GROUP_PAD_Y;

    let mut section_breaks: Vec<(f64, Option<String>)> = Vec::new();

    for (i, (section_label, section_body)) in group.sections.iter().enumerate() {
        if i > 0 {
            section_breaks.push((ctx.y, section_label.clone()));
            ctx.y += GROUP_HEADER_H;
        }
        layout_elements(ctx, section_body);
    }

    ctx.y += GROUP_PAD_Y;
    let y_end = ctx.y;

    // Span across all participants (simple approach — tighten later)
    let x = if ctx.centers.is_empty() {
        SIDE_MARGIN
    } else {
        ctx.centers.first().copied().unwrap_or(SIDE_MARGIN) - 60.0
    };
    let x_right = ctx
        .centers
        .last()
        .copied()
        .map(|c| c + 60.0)
        .unwrap_or(ctx.total_width - SIDE_MARGIN);
    let x = x.max(SIDE_MARGIN / 2.0);
    let width = (x_right - x).max(100.0);

    ctx.elements.push(LayoutElement::Group(GroupLayout {
        kind: group.kind.clone(),
        label: group.label.clone(),
        x,
        y: y_start,
        width,
        height: y_end - y_start,
        section_breaks,
    }));
}

fn collect_messages<'a>(elements: &'a [SequenceElement], out: &mut Vec<&'a Message>) {
    for e in elements {
        match e {
            SequenceElement::Message(m) => out.push(m),
            SequenceElement::Group(g) => {
                for (_, body) in &g.sections {
                    collect_messages(body, out);
                }
            }
            _ => {}
        }
    }
}

fn widen_for_messages(
    participants: &[&Participant],
    elements: &[SequenceElement],
    col_widths: &mut [f64],
) {
    let mut msgs: Vec<&Message> = Vec::new();
    collect_messages(elements, &mut msgs);
    for msg in msgs {
        let fi = participants
            .iter()
            .position(|p| p.alias.as_deref().unwrap_or(&p.name) == msg.from || p.name == msg.from);
        let ti = participants
            .iter()
            .position(|p| p.alias.as_deref().unwrap_or(&p.name) == msg.to || p.name == msg.to);
        if let (Some(fi), Some(ti)) = (fi, ti) {
            if fi == ti {
                // Self-message: the loop arc extends loop_w to the right
                // of the participant centre, so the column needs to be
                // wide enough that the loop's right edge stays inside the
                // column (with a small buffer). Otherwise the loop
                // crashes into the next participant's lifeline.
                let loop_w = crate::render::sequence::self_loop_width(&msg.label);
                let needed_half = loop_w + 10.0;
                let current_half = col_widths[fi] / 2.0;
                if needed_half > current_half {
                    col_widths[fi] = needed_half * 2.0;
                }
            } else {
                let (lo, hi) = (fi.min(ti), fi.max(ti));
                let span: f64 = col_widths[lo..=hi].iter().sum::<f64>()
                    + PARTICIPANT_H_PADDING * (hi - lo) as f64;
                let needed = text_width(&msg.label) + 20.0;
                if needed > span && hi > lo {
                    let extra = (needed - span) / (hi - lo) as f64;
                    for w in &mut col_widths[lo..hi] {
                        *w += extra;
                    }
                }
            }
        }
    }
}

fn find_participant(participants: &[&Participant], name: &str) -> Option<usize> {
    participants
        .iter()
        .position(|p| p.alias.as_deref().unwrap_or(&p.name) == name || p.name == name)
}
