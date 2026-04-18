use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::timing::*;
use crate::error::PumlError;

#[derive(Parser)]
#[grammar = "parser/grammars/timing.pest"]
struct TimingParser;

pub fn parse(source: &str) -> Result<TimingDiagram, PumlError> {
    let pairs = TimingParser::parse(Rule::diagram, source).map_err(|e| {
        let line = match e.line_col {
            pest::error::LineColLocation::Pos((l, _)) => l,
            pest::error::LineColLocation::Span((l, _), _) => l,
        };
        PumlError::Parse {
            line,
            message: e.to_string(),
        }
    })?;

    let mut diagram = TimingDiagram::default();
    let mut current_time: u64 = 0;

    for pair in pairs {
        for stmt in pair.into_inner() {
            match stmt.as_rule() {
                Rule::title_stmt => {
                    diagram.title = Some(stmt.into_inner().as_str().trim().to_string());
                }
                Rule::lane_stmt => {
                    let lane = parse_lane(stmt);
                    diagram.lanes.push(lane);
                }
                Rule::time_stmt => {
                    current_time = parse_time(stmt);
                }
                Rule::is_stmt => {
                    if let Some(event) = parse_is(stmt, current_time) {
                        diagram.events.push(event);
                    }
                }
                Rule::skinparam_stmt => {
                    if let Some((k, v)) = super::extract_skinparam(stmt.as_str()) {
                        diagram.skinparams.push((k, v));
                    }
                }
                Rule::EOI => {}
                _ => {}
            }
        }
    }

    Ok(diagram)
}

fn parse_lane(pair: Pair<Rule>) -> Lane {
    let mut kind = LaneKind::Robust;
    let mut display: Option<String> = None;
    let mut alias: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::lane_kw => {
                kind = match inner.as_str().to_lowercase().as_str() {
                    "concise" => LaneKind::Concise,
                    "clock" => LaneKind::Clock,
                    "binary" => LaneKind::Binary,
                    _ => LaneKind::Robust,
                };
            }
            Rule::quoted_string => {
                let raw = inner.as_str();
                display = Some(raw[1..raw.len() - 1].to_string());
            }
            Rule::bare_name => {
                if display.is_none() && alias.is_none() {
                    display = Some(inner.as_str().trim().to_string());
                } else {
                    alias = Some(inner.as_str().trim().to_string());
                }
            }
            _ => {}
        }
    }

    let display = display.unwrap_or_default();
    let (name, label) = match alias {
        Some(a) => (a, Some(display)),
        None => (display, None),
    };
    Lane { name, label, kind }
}

fn parse_time(pair: Pair<Rule>) -> u64 {
    pair.into_inner()
        .find(|p| p.as_rule() == Rule::time_value)
        .and_then(|p| p.as_str().parse().ok())
        .unwrap_or(0)
}

fn parse_is(pair: Pair<Rule>, time: u64) -> Option<Event> {
    let mut lane = String::new();
    let mut state = String::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::lane_ref => lane = inner.as_str().trim().to_string(),
            Rule::state_name => state = inner.as_str().trim().to_string(),
            Rule::quoted_string => {
                let raw = inner.as_str();
                state = raw[1..raw.len() - 1].to_string();
            }
            _ => {}
        }
    }
    if lane.is_empty() || state.is_empty() {
        return None;
    }
    Some(Event { time, lane, state })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_timing() {
        let src = "\
robust \"Browser\" as B
concise \"User\" as U
@0
U is Idle
B is Idle
@100
U is Waiting
B is Processing
";
        let d = parse(src).unwrap();
        assert_eq!(d.lanes.len(), 2);
        assert_eq!(d.lanes[0].name, "B");
        assert_eq!(d.lanes[0].kind, LaneKind::Robust);
        assert_eq!(d.lanes[1].kind, LaneKind::Concise);
        assert_eq!(d.events.len(), 4);
        assert_eq!(d.events[2].time, 100);
        assert_eq!(d.events[2].lane, "U");
        assert_eq!(d.events[2].state, "Waiting");
    }
}
