use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::sequence::*;
use crate::error::PumlError;

#[derive(Parser)]
#[grammar = "parser/grammars/sequence.pest"]
struct SequenceParser;

pub fn parse(source: &str) -> Result<SequenceDiagram, PumlError> {
    let pairs = SequenceParser::parse(Rule::diagram, source).map_err(|e| {
        let line = match e.line_col {
            pest::error::LineColLocation::Pos((l, _)) => l,
            pest::error::LineColLocation::Span((l, _), _) => l,
        };
        PumlError::Parse {
            line,
            message: e.to_string(),
        }
    })?;

    let mut diagram = SequenceDiagram::new();
    // Track implicit participants (those first seen in messages)
    let mut seen_names: Vec<String> = Vec::new();

    for pair in pairs {
        for stmt in pair.into_inner() {
            match stmt.as_rule() {
                Rule::title_stmt => {
                    diagram.title = Some(stmt.into_inner().as_str().trim().to_string());
                }
                Rule::participant_stmt => {
                    let p = parse_participant(stmt);
                    let name = p.alias.clone().unwrap_or_else(|| p.name.clone());
                    if !seen_names.contains(&name) {
                        seen_names.push(name.clone());
                    }
                    // Replace if already seen implicitly
                    if let Some(pos) = diagram
                        .participants
                        .iter()
                        .position(|x| x.alias.as_deref().unwrap_or(&x.name) == name)
                    {
                        diagram.participants[pos] = p;
                    } else {
                        diagram.participants.push(p);
                    }
                }
                Rule::message_stmt => {
                    let (msg, from, to) = parse_message(stmt);
                    for name in [&from, &to] {
                        if !seen_names.contains(name) {
                            seen_names.push(name.clone());
                            diagram.participants.push(Participant {
                                name: name.clone(),
                                alias: None,
                                kind: ParticipantKind::Participant,
                                color: None,
                            });
                        }
                    }
                    diagram.elements.push(SequenceElement::Message(msg));
                }
                Rule::note_stmt => {
                    if let Some(note) = parse_note(stmt) {
                        diagram.elements.push(SequenceElement::Note(note));
                    }
                }
                Rule::group_stmt => {
                    let group = parse_group(stmt);
                    diagram.elements.push(SequenceElement::Group(group));
                }
                Rule::divider_stmt => {
                    let label = stmt.into_inner().as_str().trim().to_string();
                    diagram
                        .elements
                        .push(SequenceElement::Divider(Divider { label }));
                }
                Rule::activate_stmt => {
                    let name = stmt.into_inner().as_str().trim().to_string();
                    diagram.elements.push(SequenceElement::Activate(name));
                }
                Rule::deactivate_stmt => {
                    let name = stmt.into_inner().as_str().trim().to_string();
                    diagram.elements.push(SequenceElement::Deactivate(name));
                }
                Rule::delay_stmt => {
                    let label = stmt.as_str().trim_start_matches('.').trim().to_string();
                    diagram.elements.push(SequenceElement::Delay(label));
                }
                Rule::space_stmt => {
                    let inner = stmt.as_str().trim_start_matches('|');
                    let px: u32 = inner.parse().unwrap_or(5);
                    diagram.elements.push(SequenceElement::Space(px));
                }
                Rule::autonumber_stmt => {
                    let inner = stmt.into_inner().as_str().trim();
                    let start = inner.parse().ok();
                    diagram.elements.push(SequenceElement::Autonumber(start));
                }
                Rule::hide_footbox_stmt => {
                    diagram.hide_footbox = true;
                }
                Rule::skinparam_stmt | Rule::EOI => {}
                _ => {}
            }
        }
    }

    Ok(diagram)
}

fn unquote(s: &str) -> String {
    if s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn parse_participant(pair: Pair<Rule>) -> Participant {
    let mut kind = ParticipantKind::Participant;
    let mut name = String::new();
    let mut alias = None;
    let mut color = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::participant_kw => {
                kind = match inner.as_str().to_lowercase().as_str() {
                    "actor" => ParticipantKind::Actor,
                    "boundary" => ParticipantKind::Boundary,
                    "control" => ParticipantKind::Control,
                    "entity" => ParticipantKind::Entity,
                    "database" => ParticipantKind::Database,
                    "collections" => ParticipantKind::Collections,
                    "queue" => ParticipantKind::Queue,
                    _ => ParticipantKind::Participant,
                };
            }
            Rule::participant_name => {
                name = unquote(inner.as_str().trim());
            }
            Rule::alias => {
                alias = Some(inner.as_str().trim().to_string());
            }
            Rule::color => {
                color = Some(inner.as_str().trim().to_string());
            }
            _ => {}
        }
    }

    Participant {
        name,
        alias,
        kind,
        color,
    }
}

/// Returns (style, is_reverse). Reverse arrows (`<-`, `<--`) are rendered by
/// swapping from/to and marking the message as a return.
fn parse_arrow_style(s: &str) -> (ArrowStyle, bool) {
    let reverse = s.starts_with('<') && !s.ends_with('>');
    let style = match s {
        "-->" | "<--" => ArrowStyle::Dashed,
        "-->>" | "<<--" => ArrowStyle::Dashed,
        "->>" | "<<-" => ArrowStyle::SolidThick,
        "->x" | "x->" => ArrowStyle::Lost,
        _ => ArrowStyle::Solid,
    };
    (style, reverse)
}

fn parse_message(pair: Pair<Rule>) -> (Message, String, String) {
    let mut parts = pair.into_inner();

    let left_raw = parts
        .next()
        .map(|p| unquote(p.as_str().trim()))
        .unwrap_or_default();
    let arrow_pair = parts.next();
    let right_raw = parts
        .next()
        .map(|p| unquote(p.as_str().trim()))
        .unwrap_or_default();
    let label = parts
        .next()
        .map(|p| p.as_str().trim().to_string())
        .unwrap_or_default();

    let (arrow_style, reverse) = arrow_pair
        .map(|p| parse_arrow_style(p.as_str().trim()))
        .unwrap_or((ArrowStyle::Solid, false));

    let (from_raw, to_raw) = if reverse {
        (right_raw, left_raw)
    } else {
        (left_raw, right_raw)
    };

    let msg = Message {
        from: from_raw.clone(),
        to: to_raw.clone(),
        label,
        arrow: arrow_style,
        activate: false,
        deactivate: false,
        return_msg: reverse,
    };
    (msg, from_raw, to_raw)
}

fn parse_note(pair: Pair<Rule>) -> Option<Note> {
    let inner = pair.into_inner().next()?;
    let mut position = NotePosition::Right;
    let mut participants = Vec::new();
    let mut lines = Vec::new();

    for p in inner.into_inner() {
        match p.as_rule() {
            Rule::note_pos => {
                position = match p.as_str().to_lowercase().as_str() {
                    "left" => NotePosition::Left,
                    "over" => NotePosition::Over,
                    _ => NotePosition::Right,
                };
            }
            Rule::participant_ref => {
                participants.push(unquote(p.as_str().trim()));
            }
            Rule::rest_of_line => {
                lines.push(p.as_str().trim().to_string());
            }
            Rule::note_body => {
                for line in p.into_inner() {
                    lines.push(line.as_str().trim_end_matches('\n').to_string());
                }
            }
            _ => {}
        }
    }

    Some(Note {
        position,
        participants,
        lines,
    })
}

fn parse_group(pair: Pair<Rule>) -> GroupBlock {
    let mut kind = String::new();
    let mut label = String::new();
    let mut sections: Vec<(Option<String>, Vec<SequenceElement>)> = vec![(None, Vec::new())];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::group_kw => kind = p.as_str().to_string(),
            Rule::rest_of_line => label = p.as_str().trim().to_string(),
            Rule::group_body => {
                for stmt in p.into_inner() {
                    if stmt.as_rule() == Rule::else_clause {
                        let mut section_label: Option<String> = None;
                        let mut section_elems: Vec<SequenceElement> = Vec::new();
                        for inner in stmt.into_inner() {
                            match inner.as_rule() {
                                Rule::rest_of_line => {
                                    section_label = Some(inner.as_str().trim().to_string());
                                }
                                _ => collect_element(inner, &mut section_elems),
                            }
                        }
                        sections.push((section_label, section_elems));
                    } else {
                        let last = sections.last_mut().unwrap();
                        collect_element(stmt, &mut last.1);
                    }
                }
            }
            _ => {}
        }
    }

    GroupBlock {
        kind,
        label,
        sections,
    }
}

fn collect_element(pair: Pair<Rule>, elements: &mut Vec<SequenceElement>) {
    match pair.as_rule() {
        Rule::message_stmt => {
            let (msg, _, _) = parse_message(pair);
            elements.push(SequenceElement::Message(msg));
        }
        Rule::note_stmt => {
            if let Some(note) = parse_note(pair) {
                elements.push(SequenceElement::Note(note));
            }
        }
        Rule::activate_stmt => {
            let name = pair.into_inner().as_str().trim().to_string();
            elements.push(SequenceElement::Activate(name));
        }
        Rule::deactivate_stmt => {
            let name = pair.into_inner().as_str().trim().to_string();
            elements.push(SequenceElement::Deactivate(name));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_message() {
        let src = "Alice -> Bob : hello\n";
        let diagram = parse(src).unwrap();
        assert_eq!(diagram.participants.len(), 2);
        assert_eq!(diagram.elements.len(), 1);
        match &diagram.elements[0] {
            SequenceElement::Message(m) => {
                assert_eq!(m.from, "Alice");
                assert_eq!(m.to, "Bob");
                assert_eq!(m.label, "hello");
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn parses_participant_declaration() {
        let src = "participant Alice as A\nA -> Bob : hi\n";
        let diagram = parse(src).unwrap();
        let alice = diagram
            .participants
            .iter()
            .find(|p| p.name == "Alice")
            .unwrap();
        assert_eq!(alice.alias.as_deref(), Some("A"));
    }

    #[test]
    fn parses_dashed_arrow() {
        let src = "Alice --> Bob : response\n";
        let diagram = parse(src).unwrap();
        match &diagram.elements[0] {
            SequenceElement::Message(m) => assert_eq!(m.arrow, ArrowStyle::Dashed),
            _ => panic!(),
        }
    }

    #[test]
    fn parses_title() {
        let src = "title My Diagram\nAlice -> Bob : hi\n";
        let diagram = parse(src).unwrap();
        assert_eq!(diagram.title.as_deref(), Some("My Diagram"));
    }

    #[test]
    fn parses_autonumber_bare() {
        let src = "autonumber\nAlice -> Bob : hi\n";
        let diagram = parse(src).unwrap();
        assert!(matches!(
            diagram.elements[0],
            SequenceElement::Autonumber(_)
        ));
    }

    #[test]
    fn parses_groups_fixture_content() {
        let src = "autonumber\nAlice -> Bob : query\nalt success\n  Bob --> Alice : 200 OK\nelse error\n  Bob --> Alice : 500 Error\nend\nloop 3 times\n  Alice -> Bob : poll\nend\n";
        match parse(src) {
            Ok(_) => {}
            Err(e) => panic!("parse failed: {}", e),
        }
    }

    #[test]
    fn parses_reverse_arrows() {
        let src = "Alice <- Bob : response\n";
        let diagram = parse(src).unwrap();
        match &diagram.elements[0] {
            SequenceElement::Message(m) => {
                assert_eq!(m.from, "Bob");
                assert_eq!(m.to, "Alice");
                assert!(m.return_msg);
            }
            _ => panic!("expected message"),
        }
    }
}
