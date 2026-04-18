use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::state::*;
use crate::error::PumlError;

#[derive(Parser)]
#[grammar = "parser/grammars/state.pest"]
struct StateParser;

pub fn parse(source: &str) -> Result<StateDiagram, PumlError> {
    let pairs = StateParser::parse(Rule::diagram, source).map_err(|e| {
        let line = match e.line_col {
            pest::error::LineColLocation::Pos((l, _)) => l,
            pest::error::LineColLocation::Span((l, _), _) => l,
        };
        PumlError::Parse {
            line,
            message: e.to_string(),
        }
    })?;

    let mut diagram = StateDiagram::default();

    for pair in pairs {
        for stmt in pair.into_inner() {
            match stmt.as_rule() {
                Rule::title_stmt => {
                    diagram.title = Some(stmt.into_inner().as_str().trim().to_string());
                }
                Rule::transition_stmt => {
                    if let Some(t) = parse_transition(stmt) {
                        ensure_states(&mut diagram, &t);
                        diagram.transitions.push(t);
                    }
                }
                Rule::state_block => {
                    let node = parse_state_block(stmt, &mut diagram.transitions);
                    ensure_state_node(&mut diagram.states, node);
                }
                Rule::skinparam_stmt => {
                    if let Some((k, v)) = super::extract_skinparam(stmt.as_str()) {
                        diagram.skinparams.push((k, v));
                    }
                }
                Rule::note_stmt | Rule::hide_stmt | Rule::EOI => {}
                _ => {}
            }
        }
    }

    // Any transition endpoints declared only inside composite states were
    // pushed to `diagram.transitions` but never registered as top-level
    // states — back-fill them so the layout engine can see them.
    let endpoints: Vec<(String, String)> = diagram
        .transitions
        .iter()
        .map(|t| (t.from.clone(), t.to.clone()))
        .collect();
    for (from, to) in endpoints {
        ensure_state_by_name(&mut diagram.states, &from);
        ensure_state_by_name(&mut diagram.states, &to);
    }

    Ok(diagram)
}

fn parse_transition(pair: Pair<Rule>) -> Option<Transition> {
    let mut refs = Vec::new();
    let mut label = None;
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::state_ref => refs.push(state_ref_name(inner)),
            Rule::transition_label => label = Some(inner.as_str().trim().to_string()),
            _ => {}
        }
    }
    if refs.len() >= 2 {
        Some(Transition {
            from: refs[0].clone(),
            to: refs[1].clone(),
            label,
        })
    } else {
        None
    }
}

fn state_ref_name(pair: Pair<Rule>) -> String {
    let s = pair.as_str().trim();
    if s == "[*]" || s == "[H]" || s == "[H*]" {
        return s.to_string();
    }
    // Strip stereotype if present
    if let Some(colon) = s.find(':') {
        s[..colon].trim().to_string()
    } else {
        s.to_string()
    }
}

fn stereotype_to_kind(stereo: &str) -> Option<StateKind> {
    match stereo.to_lowercase().as_str() {
        "choice" => Some(StateKind::Choice),
        "fork" => Some(StateKind::Fork),
        "join" => Some(StateKind::Join),
        _ => None,
    }
}

fn parse_state_block(pair: Pair<Rule>, transitions: &mut Vec<Transition>) -> StateNode {
    let mut name = String::new();
    let mut label = None;
    let mut kind = StateKind::Normal;
    let mut body = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::state_ref if name.is_empty() => {
                name = state_ref_name(inner);
            }
            Rule::state_name => {
                label = Some(inner.as_str().trim().to_string());
            }
            Rule::stereotype => {
                let raw = inner.as_str();
                let inner_text = raw[2..raw.len() - 2].trim();
                if let Some(k) = stereotype_to_kind(inner_text) {
                    kind = k;
                }
            }
            Rule::transition_stmt => {
                if let Some(t) = parse_transition(inner) {
                    transitions.push(t.clone());
                    body.push(StateElement::Transition(t));
                }
            }
            Rule::state_block => {
                let child = parse_state_block(inner, transitions);
                body.push(StateElement::SubState(child));
            }
            Rule::divider_stmt => {
                body.push(StateElement::Divider);
            }
            _ => {}
        }
    }

    StateNode {
        name,
        label,
        kind,
        body,
    }
}

fn ensure_states(diagram: &mut StateDiagram, t: &Transition) {
    for name in [&t.from, &t.to] {
        ensure_state_by_name(&mut diagram.states, name);
    }
}

fn ensure_state_by_name(states: &mut Vec<StateNode>, name: &str) {
    if !states.iter().any(|s| s.name == name) {
        let kind = match name {
            "[*]" => StateKind::Start,
            "[H]" | "[H*]" => StateKind::Normal,
            _ => StateKind::Normal,
        };
        states.push(StateNode {
            name: name.to_string(),
            label: None,
            kind,
            body: Vec::new(),
        });
    }
}

fn ensure_state_node(states: &mut Vec<StateNode>, node: StateNode) {
    if let Some(existing) = states.iter_mut().find(|s| s.name == node.name) {
        existing.body = node.body;
        existing.label = existing.label.take().or(node.label);
    } else {
        states.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_transitions() {
        let src = "[*] --> Idle\nIdle --> Active : start\nActive --> [*]\n";
        let d = parse(src).unwrap();
        assert_eq!(d.transitions.len(), 3);
        assert_eq!(d.transitions[0].from, "[*]");
        assert_eq!(d.transitions[1].label.as_deref(), Some("start"));
    }

    #[test]
    fn parses_title() {
        let src = "title My State\n[*] --> A\n";
        let d = parse(src).unwrap();
        assert_eq!(d.title.as_deref(), Some("My State"));
    }
}
