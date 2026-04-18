use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::gantt::*;
use crate::error::PumlError;

#[derive(Parser)]
#[grammar = "parser/grammars/gantt.pest"]
struct GanttParser;

pub fn parse(source: &str) -> Result<GanttDiagram, PumlError> {
    let pairs = GanttParser::parse(Rule::diagram, source).map_err(|e| {
        let line = match e.line_col {
            pest::error::LineColLocation::Pos((l, _)) => l,
            pest::error::LineColLocation::Span((l, _), _) => l,
        };
        PumlError::Parse {
            line,
            message: e.to_string(),
        }
    })?;

    let mut diagram = GanttDiagram::default();

    for pair in pairs {
        for stmt in pair.into_inner() {
            match stmt.as_rule() {
                Rule::title_stmt => {
                    diagram.title = Some(stmt.into_inner().as_str().trim().to_string());
                }
                Rule::lasts_stmt => {
                    let (name, duration) = parse_lasts(stmt);
                    update_task(&mut diagram.tasks, &name, |t| t.duration = duration);
                }
                Rule::starts_after_stmt => {
                    let (name, dep) = parse_starts_after(stmt);
                    update_task(&mut diagram.tasks, &name, |t| {
                        t.depends_on = Some(dep.clone());
                    });
                }
                Rule::starts_at_day_stmt => {
                    let (name, day) = parse_starts_at_day(stmt);
                    update_task(&mut diagram.tasks, &name, |t| t.fixed_start = Some(day));
                }
                Rule::milestone_stmt => {
                    let name = parse_single_task(stmt);
                    update_task(&mut diagram.tasks, &name, |t| {
                        t.milestone = true;
                        t.duration = 0;
                    });
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

fn update_task<F: FnOnce(&mut GanttTask)>(tasks: &mut Vec<GanttTask>, name: &str, f: F) {
    if let Some(t) = tasks.iter_mut().find(|t| t.name == name) {
        f(t);
    } else {
        let mut t = GanttTask::new(name.to_string());
        f(&mut t);
        tasks.push(t);
    }
}

fn parse_lasts(pair: Pair<Rule>) -> (String, u32) {
    let mut name = String::new();
    let mut duration = 0u32;
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::task_ref if name.is_empty() => {
                name = task_name(inner.as_str());
            }
            Rule::duration => duration = inner.as_str().parse().unwrap_or(0),
            _ => {}
        }
    }
    (name, duration)
}

fn parse_starts_after(pair: Pair<Rule>) -> (String, String) {
    let mut refs: Vec<String> = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::task_ref {
            refs.push(task_name(inner.as_str()));
        }
    }
    let name = refs.first().cloned().unwrap_or_default();
    let dep = refs.get(1).cloned().unwrap_or_default();
    (name, dep)
}

fn parse_starts_at_day(pair: Pair<Rule>) -> (String, u32) {
    let mut name = String::new();
    let mut day = 0u32;
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::task_ref => name = task_name(inner.as_str()),
            Rule::duration => day = inner.as_str().parse().unwrap_or(0),
            _ => {}
        }
    }
    (name, day)
}

fn parse_single_task(pair: Pair<Rule>) -> String {
    pair.into_inner()
        .find(|p| p.as_rule() == Rule::task_ref)
        .map(|p| task_name(p.as_str()))
        .unwrap_or_default()
}

fn task_name(raw: &str) -> String {
    let s = raw.trim();
    let s = s.strip_prefix('[').unwrap_or(s);
    let s = s.strip_suffix(']').unwrap_or(s);
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_duration_and_dependency() {
        let src =
            "[Design] lasts 5 days\n[Build] lasts 10 days\n[Build] starts at [Design]'s end\n";
        let d = parse(src).unwrap();
        assert_eq!(d.tasks.len(), 2);
        assert_eq!(d.tasks[0].name, "Design");
        assert_eq!(d.tasks[0].duration, 5);
        assert_eq!(d.tasks[1].name, "Build");
        assert_eq!(d.tasks[1].duration, 10);
        assert_eq!(d.tasks[1].depends_on.as_deref(), Some("Design"));
    }

    #[test]
    fn parses_milestone() {
        let src = "[Launch] is a milestone\n[Launch] starts at day 30\n";
        let d = parse(src).unwrap();
        assert_eq!(d.tasks.len(), 1);
        assert!(d.tasks[0].milestone);
        assert_eq!(d.tasks[0].fixed_start, Some(30));
    }
}
