use crate::ast::sequence::*;

const PARTICIPANT_WIDTH: f64 = 120.0;
const PARTICIPANT_HEIGHT: f64 = 40.0;
const PARTICIPANT_H_PADDING: f64 = 30.0; // horizontal gap between participants
const ROW_HEIGHT: f64 = 40.0;
const TOP_MARGIN: f64 = 20.0;
const SIDE_MARGIN: f64 = 20.0;
const FONT_SIZE: f64 = 13.0;
const CHAR_WIDTH: f64 = 7.5; // approximate character width at default font size
/// Baseline-to-baseline distance between the lines of a message label or
/// a participant name.
pub const LINE_HEIGHT: f64 = 16.0;
/// How far a message label's last baseline sits above its arrow: enough
/// that descenders stay clear of the line.
pub const LABEL_BASELINE: f64 = 9.0;
/// A message row, top to bottom: padding, the label (one line's ascent
/// here, each further line `LINE_HEIGHT` more), the gap down to the
/// arrow, then room below the arrow before the next row. One-line rows
/// are `ROW_HEIGHT` tall.
const ROW_TOP: f64 = 5.0 + 12.0 + LABEL_BASELINE;
const ROW_BOTTOM: f64 = ROW_HEIGHT - ROW_TOP;
/// A self-message's loop drops this far below its arrow.
const SELF_LOOP_DROP: f64 = 20.0;

pub struct ParticipantLayout {
    #[allow(dead_code)]
    pub name: String,
    /// The name shown in the header, one entry per line (`\n` breaks).
    pub display: Vec<String>,
    pub x: f64,
    pub kind: ParticipantKind,
    pub color: Option<String>,
}

pub struct MessageLayout {
    pub from_x: f64,
    pub to_x: f64,
    pub y: f64,
    /// The label's lines, top to bottom (`\n` breaks); empty for none.
    pub lines: Vec<String>,
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
    /// Height of the participant header (and footer) boxes: the default
    /// box, taller when a name runs to several lines.
    pub participant_height: f64,
}

/// How far a box extends beyond its outermost participant boxes.
const BOX_PAD: f64 = 8.0;
/// The row a box title takes above the participant headers.
const BOX_TITLE_H: f64 = 20.0;
const BOX_TOP: f64 = 6.0;

fn text_width(s: &str) -> f64 {
    s.len() as f64 * CHAR_WIDTH
}

/// Width of the widest of `lines`.
pub fn lines_width(lines: &[String]) -> f64 {
    lines.iter().map(|l| text_width(l)).fold(0.0, f64::max)
}

/// Display name shown inside the participant header box. PlantUML's
/// convention is `participant "Display Name" as ALIAS` — the quoted name
/// is the human-readable label, and the alias is just a short referent
/// for messages. So always show `name`, falling back to alias only if
/// the parser produced an aliased entry with no canonical name (rare but
/// possible from forward declarations).
fn participant_display(p: &Participant) -> Vec<String> {
    let name = if !p.name.is_empty() {
        p.name.clone()
    } else {
        p.alias.clone().unwrap_or_default()
    };
    super::text_lines(&name)
}

pub fn layout(diagram: &SequenceDiagram) -> SequenceLayout {
    let participants = diagram.ordered_participants();

    let n = participants.len();
    let mut col_widths: Vec<f64> = participants
        .iter()
        .map(|p| {
            let w = lines_width(&participant_display(p)) + 20.0;
            w.max(PARTICIPANT_WIDTH)
        })
        .collect();

    // The space between neighbouring columns; a long message label
    // widens the gaps its arrow crosses.
    let mut gaps = vec![PARTICIPANT_H_PADDING; n.saturating_sub(1)];
    widen_for_messages(&participants, &diagram.elements, &mut col_widths, &mut gaps);

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
    widen_for_box_titles(&diagram.boxes, &box_spans, &mut col_widths, &gaps);
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

    let participant_height = participants
        .iter()
        .map(|p| participant_display(p).len() as f64 * LINE_HEIGHT + 14.0)
        .fold(PARTICIPANT_HEIGHT, f64::max);

    let mut centers: Vec<f64> = Vec::with_capacity(n);
    let mut x = SIDE_MARGIN;
    for (i, w) in col_widths.iter().enumerate() {
        centers.push(x + w / 2.0);
        x += w;
        if let Some(gap) = gaps.get(i) {
            x += gap;
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
        y: header_offset + TOP_MARGIN + participant_height + 5.0,
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
        participant_height + 10.0
    };
    let total_height = lifeline_height + footer_h + TOP_MARGIN;

    let box_bottom = lifeline_height
        + if diagram.hide_footbox {
            0.0
        } else {
            participant_height
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
        participant_height,
    }
}

/// Widens a box's member columns so its title fits inside it.
fn widen_for_box_titles(
    boxes: &[ParticipantBox],
    spans: &[Option<(usize, usize)>],
    col_widths: &mut [f64],
    gaps: &[f64],
) {
    for (b, span) in boxes.iter().zip(spans) {
        let (Some(title), Some((lo, hi))) = (&b.title, span) else {
            continue;
        };
        let (lo, hi) = (*lo, *hi);
        // The box spans the outer participant boxes plus its padding.
        let inner = centers_span(col_widths, gaps, lo, hi) + PARTICIPANT_WIDTH + 2.0 * BOX_PAD;
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
fn centers_span(col_widths: &[f64], gaps: &[f64], lo: usize, hi: usize) -> f64 {
    if lo == hi {
        return 0.0;
    }
    col_widths[lo] / 2.0
        + col_widths[lo + 1..hi].iter().sum::<f64>()
        + col_widths[hi] / 2.0
        + gaps[lo..hi].iter().sum::<f64>()
}

/// A message's label lines as drawn: split at `\n`, with the autonumber
/// prefix (advancing the counter) when numbering is on.
fn message_lines(label: &str, autonumber: &mut Option<u32>) -> Vec<String> {
    let mut lines = super::text_lines(label);
    if let Some(n) = autonumber {
        match lines.first_mut() {
            Some(first) => *first = format!("{n}: {first}"),
            None => lines.push(n.to_string()),
        }
        *n += 1;
    }
    lines
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

            let lines = message_lines(&msg.label, &mut ctx.autonumber);
            // The label sits in its own row above the arrow, so it can't
            // run into the row before; each extra line pushes the arrow
            // down by a line.
            let arrow_y = ctx.y + ROW_TOP + lines.len().saturating_sub(1) as f64 * LINE_HEIGHT;

            ctx.elements.push(LayoutElement::Message(MessageLayout {
                from_x,
                to_x,
                y: arrow_y,
                lines,
                dashed,
                self_msg,
                lost: matches!(msg.arrow, ArrowStyle::Lost),
            }));
            // A self-message's loop drops below its arrow; the next row
            // starts below the loop.
            ctx.y = arrow_y + ROW_BOTTOM + if self_msg { SELF_LOOP_DROP } else { 0.0 };
        }
        SequenceElement::Note(note) => {
            let anchor_x = note
                .participants
                .first()
                .and_then(|name| find_participant(ctx.participants, name).map(|i| ctx.centers[i]))
                .unwrap_or(SIDE_MARGIN + 60.0);
            let lines: Vec<String> = note
                .lines
                .iter()
                .flat_map(|l| {
                    let split = super::text_lines(l);
                    if split.is_empty() {
                        vec![String::new()]
                    } else {
                        split
                    }
                })
                .collect();
            let note_w = lines_width(&lines).max(120.0) + 20.0;
            let note_h = lines.len() as f64 * (FONT_SIZE + 4.0) + 12.0;
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
                lines,
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
            // The label sits above the line, so the line goes mid-row.
            ctx.elements.push(LayoutElement::Divider(DividerLayout {
                y: ctx.y + ROW_HEIGHT / 2.0,
                label: div.label.clone(),
                total_width: ctx.total_width,
            }));
            ctx.y += ROW_HEIGHT;
        }
        SequenceElement::Space(px) => {
            ctx.y += *px as f64;
        }
        SequenceElement::Delay(label) => {
            // A labelled delay needs room above its line for the label.
            let above = if label.is_empty() {
                ROW_HEIGHT / 4.0
            } else {
                ROW_HEIGHT / 2.0
            };
            ctx.elements.push(LayoutElement::Divider(DividerLayout {
                y: ctx.y + above,
                label: label.clone(),
                total_width: ctx.total_width,
            }));
            ctx.y += above + ROW_HEIGHT / 4.0;
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

/// Every message in drawing order, with its label lines as drawn
/// (numbered the way `layout_one` numbers them).
fn collect_messages<'a>(
    elements: &'a [SequenceElement],
    autonumber: &mut Option<u32>,
    out: &mut Vec<(&'a Message, Vec<String>)>,
) {
    for e in elements {
        match e {
            SequenceElement::Message(m) => out.push((m, message_lines(&m.label, autonumber))),
            SequenceElement::Autonumber(start) => *autonumber = Some(start.unwrap_or(1)),
            SequenceElement::Group(g) => {
                for (_, body) in &g.sections {
                    collect_messages(body, autonumber, out);
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
    gaps: &mut [f64],
) {
    let mut msgs = Vec::new();
    collect_messages(elements, &mut None, &mut msgs);
    for (msg, lines) in msgs {
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
                let loop_w = crate::render::sequence::self_loop_width(&lines);
                let needed_half = loop_w + 10.0;
                let current_half = col_widths[fi] / 2.0;
                if needed_half > current_half {
                    col_widths[fi] = needed_half * 2.0;
                }
            } else {
                // The label is centred on the arrow, which runs centre to
                // centre; the arrow must outrun the widest line.
                let (lo, hi) = (fi.min(ti), fi.max(ti));
                let span = centers_span(col_widths, gaps, lo, hi);
                let needed = lines_width(&lines) + 20.0;
                if needed > span {
                    let extra = (needed - span) / (hi - lo) as f64;
                    for gap in &mut gaps[lo..hi] {
                        *gap += extra;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_of(source: &str) -> SequenceLayout {
        let diagrams = crate::parser::preprocessor::preprocess(source, None);
        match crate::parser::parse(&diagrams[0]).expect("parses") {
            crate::ast::DiagramAst::Sequence(seq) => layout(&seq),
            _ => panic!("not a sequence diagram"),
        }
    }

    fn messages(l: &SequenceLayout) -> Vec<&MessageLayout> {
        l.elements
            .iter()
            .filter_map(|e| match e {
                LayoutElement::Message(m) => Some(m),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn backslash_n_splits_a_message_label_and_pushes_its_arrow_down() {
        let one = layout_of("@startuml\nA -> B: first\nA -> B: second\n@enduml\n");
        let two = layout_of("@startuml\nA -> B: first\nA -> B: New Order\\n http://x/1\n@enduml\n");
        let (m1, m2) = (messages(&one), messages(&two));
        assert_eq!(m2[1].lines, vec!["New Order", " http://x/1"]);
        assert_eq!(m2[1].y - m2[0].y, m1[1].y - m1[0].y + LINE_HEIGHT);
    }

    #[test]
    fn labels_sit_in_their_own_row_clear_of_the_one_above() {
        let l = layout_of(
            "@startuml\nA -> B: one\nA -> B: two\\nlines\nB -> B: self\nA -> B: after\n@enduml\n",
        );
        let m = messages(&l);
        // A label's top: its first baseline less a line's ascent.
        let top = |m: &MessageLayout| {
            m.y - LABEL_BASELINE - (m.lines.len() - 1) as f64 * LINE_HEIGHT - 12.0
        };
        let header_bottom = TOP_MARGIN + l.participant_height;
        assert!(
            top(m[0]) >= header_bottom + 4.0,
            "first label under the header"
        );
        // Clear of the previous arrow and its arrowhead (7.5 px each side).
        for pair in m.windows(2) {
            let prev_bottom = if pair[0].self_msg {
                pair[0].y + SELF_LOOP_DROP
            } else {
                pair[0].y + 7.5
            };
            assert!(top(pair[1]) >= prev_bottom + 4.0, "{:?}", pair[1].lines);
        }
    }

    #[test]
    fn autonumber_prefixes_the_first_line_only() {
        let l = layout_of("@startuml\nautonumber\nA -> B: a\\nb\nA -> B\n@enduml\n");
        let m = messages(&l);
        assert_eq!(m[0].lines, vec!["1: a", "b"]);
        assert_eq!(m[1].lines, vec!["2"]);
    }

    #[test]
    fn columns_widen_for_the_widest_line_not_the_whole_label() {
        let line = "x".repeat(40);
        let split = layout_of(&format!("@startuml\nA -> B: {line}\\n{line}\n@enduml\n"));
        let joined = layout_of(&format!("@startuml\nA -> B: {line}{line}\n@enduml\n"));
        let single = layout_of(&format!("@startuml\nA -> B: {line}\n@enduml\n"));
        assert_eq!(split.total_width, single.total_width);
        assert!(joined.total_width > split.total_width);
    }

    #[test]
    fn arrows_outrun_their_numbered_labels() {
        let label = "new-customer-event <graham, graham@grahambrooks.com>";
        let l = layout_of(&format!(
            "@startuml\nautonumber\nA -> B: x\nB -> C: {label}\n@enduml\n"
        ));
        let m = messages(&l);
        let run = (m[1].to_x - m[1].from_x).abs();
        assert!(
            run >= lines_width(&m[1].lines) + 20.0,
            "arrow {run} too short"
        );
    }

    #[test]
    fn notes_and_participant_names_break_at_backslash_n() {
        let l = layout_of(
            "@startuml\nparticipant \"Order\\nService\" as S\nS -> S: x\nnote right of S: one\\ntwo\n@enduml\n",
        );
        assert_eq!(l.participants[0].display, vec!["Order", "Service"]);
        assert!(l.participant_height > PARTICIPANT_HEIGHT);
        let note = l
            .elements
            .iter()
            .find_map(|e| match e {
                LayoutElement::Note(n) => Some(n),
                _ => None,
            })
            .expect("a note");
        assert_eq!(note.lines, vec!["one", "two"]);
    }
}
