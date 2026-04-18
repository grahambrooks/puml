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

pub enum LayoutElement {
    Message(MessageLayout),
    Note(NoteLayout),
    Activation(ActivationLayout),
    Divider(DividerLayout),
}

pub struct SequenceLayout {
    pub participants: Vec<ParticipantLayout>,
    pub elements: Vec<LayoutElement>,
    pub lifeline_height: f64,
    pub total_width: f64,
    pub total_height: f64,
    pub title: Option<String>,
    pub hide_footbox: bool,
}

fn text_width(s: &str) -> f64 {
    s.len() as f64 * CHAR_WIDTH
}

fn participant_display(p: &Participant) -> String {
    p.alias.clone().unwrap_or_else(|| p.name.clone())
}

pub fn layout(diagram: &SequenceDiagram) -> SequenceLayout {
    let participants = diagram.ordered_participants();

    // Compute column widths based on longest label they carry
    let n = participants.len();
    let mut col_widths: Vec<f64> = participants
        .iter()
        .map(|p| {
            let w = text_width(&participant_display(p)) + 20.0;
            w.max(PARTICIPANT_WIDTH)
        })
        .collect();

    // Widen columns to fit message labels that span them
    for elem in &diagram.elements {
        if let SequenceElement::Message(msg) = elem {
            let fi = participants.iter().position(|p| {
                p.alias.as_deref().unwrap_or(&p.name) == msg.from || p.name == msg.from
            });
            let ti = participants
                .iter()
                .position(|p| p.alias.as_deref().unwrap_or(&p.name) == msg.to || p.name == msg.to);
            if let (Some(fi), Some(ti)) = (fi, ti) {
                if fi != ti {
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

    // Compute center x for each participant
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

    // Build participant layouts
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

    // Lay out elements row by row
    let mut y = TOP_MARGIN + PARTICIPANT_HEIGHT + 10.0;
    let mut elements: Vec<LayoutElement> = Vec::new();
    let mut active_depth: Vec<u32> = vec![0; n]; // activation depth per participant
    let mut open_activations: Vec<(usize, f64)> = Vec::new(); // (participant idx, start y)

    for elem in &diagram.elements {
        match elem {
            SequenceElement::Message(msg) => {
                let fi = find_participant(&participants, &msg.from);
                let ti = find_participant(&participants, &msg.to);
                let (from_x, to_x) = match (fi, ti) {
                    (Some(f), Some(t)) => (centers[f], centers[t]),
                    (Some(f), None) => (centers[f], centers[f] + 60.0),
                    _ => (SIDE_MARGIN + 60.0, SIDE_MARGIN + 180.0),
                };
                let dashed = matches!(msg.arrow, ArrowStyle::Dashed | ArrowStyle::SolidThick);
                let self_msg = fi == ti;
                elements.push(LayoutElement::Message(MessageLayout {
                    from_x,
                    to_x,
                    y,
                    label: msg.label.clone(),
                    dashed,
                    self_msg,
                    lost: matches!(msg.arrow, ArrowStyle::Lost),
                }));
                y += ROW_HEIGHT;
            }
            SequenceElement::Note(note) => {
                let anchor_x = note
                    .participants
                    .first()
                    .and_then(|name| find_participant(&participants, name).map(|i| centers[i]))
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
                elements.push(LayoutElement::Note(NoteLayout {
                    x: note_x,
                    y,
                    width: note_w,
                    height: note_h,
                    lines: note.lines.clone(),
                }));
                y += note_h + 10.0;
            }
            SequenceElement::Activate(name) => {
                if let Some(i) = find_participant(&participants, name) {
                    open_activations.push((i, y));
                    active_depth[i] += 1;
                }
            }
            SequenceElement::Deactivate(name) => {
                if let Some(i) = find_participant(&participants, name) {
                    if active_depth[i] > 0 {
                        active_depth[i] -= 1;
                        if let Some(pos) = open_activations.iter().rposition(|(idx, _)| *idx == i) {
                            let (_, y_start) = open_activations.remove(pos);
                            let depth = active_depth[i];
                            elements.push(LayoutElement::Activation(ActivationLayout {
                                participant_x: centers[i],
                                y_start,
                                y_end: y,
                                depth,
                            }));
                        }
                    }
                }
            }
            SequenceElement::Divider(div) => {
                elements.push(LayoutElement::Divider(DividerLayout {
                    y,
                    label: div.label.clone(),
                    total_width,
                }));
                y += ROW_HEIGHT;
            }
            SequenceElement::Space(px) => {
                y += *px as f64;
            }
            SequenceElement::Delay(label) => {
                elements.push(LayoutElement::Divider(DividerLayout {
                    y,
                    label: label.clone(),
                    total_width,
                }));
                y += ROW_HEIGHT / 2.0;
            }
            SequenceElement::Group(_) | SequenceElement::Autonumber(_) => {}
        }
    }

    // Close any still-open activations
    for (i, y_start) in open_activations {
        elements.push(LayoutElement::Activation(ActivationLayout {
            participant_x: centers[i],
            y_start,
            y_end: y,
            depth: 0,
        }));
    }

    let lifeline_height = y + 10.0;
    let footer_h = if diagram.hide_footbox {
        0.0
    } else {
        PARTICIPANT_HEIGHT + 10.0
    };
    let total_height = lifeline_height + footer_h + TOP_MARGIN;

    SequenceLayout {
        participants: participant_layouts,
        elements,
        lifeline_height,
        total_width,
        total_height,
        title: diagram.title.clone(),
        hide_footbox: diagram.hide_footbox,
    }
}

fn find_participant(participants: &[&Participant], name: &str) -> Option<usize> {
    participants
        .iter()
        .position(|p| p.alias.as_deref().unwrap_or(&p.name) == name || p.name == name)
}
