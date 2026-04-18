use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::activity::*;
use crate::error::PumlError;

#[derive(Parser)]
#[grammar = "parser/grammars/activity.pest"]
struct ActivityParser;

pub fn parse(source: &str) -> Result<ActivityDiagram, PumlError> {
    let pairs = ActivityParser::parse(Rule::diagram, source).map_err(|e| {
        let line = match e.line_col {
            pest::error::LineColLocation::Pos((l, _)) => l,
            pest::error::LineColLocation::Span((l, _), _) => l,
        };
        PumlError::Parse {
            line,
            message: e.to_string(),
        }
    })?;

    let mut diagram = ActivityDiagram::default();

    for pair in pairs {
        for stmt in pair.into_inner() {
            if stmt.as_rule() == Rule::skinparam_stmt {
                if let Some((k, v)) = super::extract_skinparam(stmt.as_str()) {
                    diagram.skinparams.push((k, v));
                }
                continue;
            }
            if let Some(elem) = parse_element(stmt) {
                match elem {
                    ActivityElement::Label(ref t) if diagram.title.is_none() => {
                        // title_stmt produces a Label variant — intercept it
                        let _ = elem;
                    }
                    other => {
                        if let Some(title) = extract_title(&other) {
                            diagram.title = Some(title);
                        } else {
                            diagram.elements.push(other);
                        }
                    }
                }
            }
        }
    }

    Ok(diagram)
}

fn extract_title(_elem: &ActivityElement) -> Option<String> {
    None
}

fn parse_element(pair: Pair<Rule>) -> Option<ActivityElement> {
    match pair.as_rule() {
        Rule::title_stmt => {
            // Store in diagram.title separately; handled by caller
            Some(ActivityElement::Label(format!(
                "__title__{}",
                pair.into_inner().as_str().trim()
            )))
        }
        Rule::start_stmt => Some(ActivityElement::Start),
        Rule::stop_stmt => Some(ActivityElement::Stop),
        Rule::end_stmt => Some(ActivityElement::End),
        Rule::detach_stmt => Some(ActivityElement::Detach),
        Rule::kill_stmt => Some(ActivityElement::Kill),
        Rule::action_stmt => Some(parse_action(pair)),
        Rule::label_stmt => {
            let text = pair.into_inner().as_str().trim().to_string();
            Some(ActivityElement::Label(text))
        }
        Rule::arrow_stmt => {
            let label = pair
                .into_inner()
                .find(|p| p.as_rule() == Rule::arrow_label)
                .map(|p| p.as_str().trim().to_string());
            Some(ActivityElement::Arrow(label))
        }
        Rule::if_stmt => Some(parse_if(pair)),
        Rule::while_stmt => Some(parse_while(pair)),
        Rule::repeat_stmt => Some(parse_repeat(pair)),
        Rule::fork_stmt => Some(parse_fork(pair)),
        Rule::partition_stmt => Some(parse_partition(pair)),
        Rule::note_stmt => Some(parse_note(pair)),
        Rule::skinparam_stmt => None, // collected separately in parse()
        Rule::EOI => None,
        _ => None,
    }
}

fn parse_action(pair: Pair<Rule>) -> ActivityElement {
    let mut label = String::new();
    let mut color = None;
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::action_text => label = inner.as_str().trim().to_string(),
            Rule::color => color = Some(inner.as_str().trim().to_string()),
            _ => {}
        }
    }
    ActivityElement::Action(Action { label, color })
}

fn parse_if(pair: Pair<Rule>) -> ActivityElement {
    let mut condition = String::new();
    let mut then_label = None;
    let mut then_body: Vec<ActivityElement> = Vec::new();
    let mut elseif_branches: Vec<ElseIfBranch> = Vec::new();
    let mut else_label = None;
    let mut else_body: Vec<ActivityElement> = Vec::new();

    let mut in_then = true;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::condition => condition = inner.as_str().trim().to_string(),
            Rule::branch_label => {
                if in_then {
                    then_label = Some(inner.as_str().trim().to_string());
                }
            }
            Rule::elseif_clause => {
                in_then = false;
                elseif_branches.push(parse_elseif(inner));
            }
            Rule::else_clause => {
                in_then = false;
                let (lbl, body) = parse_else(inner);
                else_label = lbl;
                else_body = body;
            }
            _ => {
                if in_then {
                    if let Some(e) = parse_element(inner) {
                        if !is_title_marker(&e) {
                            then_body.push(e);
                        }
                    }
                }
            }
        }
    }

    ActivityElement::If(IfBlock {
        condition,
        then_label,
        then_body,
        elseif_branches,
        else_label,
        else_body,
    })
}

fn parse_elseif(pair: Pair<Rule>) -> ElseIfBranch {
    let mut condition = String::new();
    let mut label = None;
    let mut body = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::condition => condition = inner.as_str().trim().to_string(),
            Rule::branch_label => label = Some(inner.as_str().trim().to_string()),
            _ => {
                if let Some(e) = parse_element(inner) {
                    if !is_title_marker(&e) {
                        body.push(e);
                    }
                }
            }
        }
    }
    ElseIfBranch {
        condition,
        label,
        body,
    }
}

fn parse_else(pair: Pair<Rule>) -> (Option<String>, Vec<ActivityElement>) {
    let mut label = None;
    let mut body = Vec::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::branch_label => label = Some(inner.as_str().trim().to_string()),
            _ => {
                if let Some(e) = parse_element(inner) {
                    if !is_title_marker(&e) {
                        body.push(e);
                    }
                }
            }
        }
    }
    (label, body)
}

fn parse_while(pair: Pair<Rule>) -> ActivityElement {
    let mut condition = String::new();
    let mut exit_label = None;
    let mut body = Vec::new();
    let mut labels: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::condition => condition = inner.as_str().trim().to_string(),
            Rule::branch_label => labels.push(inner.as_str().trim().to_string()),
            _ => {
                if let Some(e) = parse_element(inner) {
                    if !is_title_marker(&e) {
                        body.push(e);
                    }
                }
            }
        }
    }
    // Second branch_label (if any) is the exit label on endwhile
    if labels.len() > 1 {
        exit_label = labels.pop();
    }

    ActivityElement::While(WhileBlock {
        condition,
        exit_label,
        body,
    })
}

fn parse_repeat(pair: Pair<Rule>) -> ActivityElement {
    let mut condition = String::new();
    let mut label = None;
    let mut body = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::condition => condition = inner.as_str().trim().to_string(),
            Rule::branch_label => label = Some(inner.as_str().trim().to_string()),
            _ => {
                if let Some(e) = parse_element(inner) {
                    if !is_title_marker(&e) {
                        body.push(e);
                    }
                }
            }
        }
    }

    ActivityElement::Repeat(RepeatBlock {
        body,
        condition,
        label,
    })
}

fn parse_fork(pair: Pair<Rule>) -> ActivityElement {
    let mut branches: Vec<Vec<ActivityElement>> = Vec::new();
    let mut join = true;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::fork_first_branch | Rule::fork_more_branch => {
                let mut branch = Vec::new();
                for elem in inner.into_inner() {
                    if let Some(e) = parse_element(elem) {
                        if !is_title_marker(&e) {
                            branch.push(e);
                        }
                    }
                }
                branches.push(branch);
            }
            Rule::fork_end => {
                let s = inner.as_str().to_lowercase();
                join = !s.contains("merge");
            }
            _ => {}
        }
    }

    ActivityElement::Fork(ForkBlock { branches, join })
}

fn parse_partition(pair: Pair<Rule>) -> ActivityElement {
    let mut name = String::new();
    let mut body = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::partition_name => {
                name = inner.as_str().trim_matches('"').trim().to_string();
            }
            _ => {
                if let Some(e) = parse_element(inner) {
                    if !is_title_marker(&e) {
                        body.push(e);
                    }
                }
            }
        }
    }

    ActivityElement::Partition(Partition { name, body })
}

fn parse_note(pair: Pair<Rule>) -> ActivityElement {
    let mut position = NotePos::Right;
    let mut lines: Vec<String> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::note_pos => {
                position = if inner.as_str().eq_ignore_ascii_case("left") {
                    NotePos::Left
                } else {
                    NotePos::Right
                };
            }
            Rule::note_line => {
                lines.push(inner.as_str().trim_end_matches('\n').to_string());
            }
            _ => {}
        }
    }

    ActivityElement::Note(ActivityNote {
        text: lines.join("\n"),
        position,
    })
}

fn is_title_marker(e: &ActivityElement) -> bool {
    matches!(e, ActivityElement::Label(s) if s.starts_with("__title__"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_flow() {
        let src = "start\n:Do something;\nstop\n";
        let d = parse(src).unwrap();
        assert!(matches!(d.elements[0], ActivityElement::Start));
        assert!(matches!(d.elements[1], ActivityElement::Action(_)));
        assert!(matches!(d.elements[2], ActivityElement::Stop));
    }

    #[test]
    fn parses_if_block() {
        let src = "start\nif (condition?) then (yes)\n  :action A;\nelse (no)\n  :action B;\nendif\nstop\n";
        let d = parse(src).unwrap();
        let if_block = d
            .elements
            .iter()
            .find(|e| matches!(e, ActivityElement::If(_)));
        assert!(if_block.is_some());
    }

    #[test]
    fn parses_while_loop() {
        let src = "start\nwhile (more data?) is (yes)\n  :process;\nendwhile (no)\nstop\n";
        let d = parse(src).unwrap();
        assert!(d
            .elements
            .iter()
            .any(|e| matches!(e, ActivityElement::While(_))));
    }

    #[test]
    fn parses_fork() {
        let src = "start\nfork\n  :branch A;\nfork again\n  :branch B;\nend fork\nstop\n";
        let d = parse(src).unwrap();
        assert!(d
            .elements
            .iter()
            .any(|e| matches!(e, ActivityElement::Fork(_))));
    }
}
