pub mod activity;
pub mod class;
pub mod preprocessor;
pub mod sequence;
pub mod state;

use crate::ast::DiagramAst;
use crate::error::PumlError;
use preprocessor::DiagramSource;

pub fn parse(source: &DiagramSource) -> Result<DiagramAst, PumlError> {
    let type_hint = source.type_hint.as_deref();

    // Detect diagram type: hint > syntax markers
    let diagram_type = type_hint
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| detect_type(&source.content));

    match diagram_type.as_str() {
        "sequence" => Ok(DiagramAst::Sequence(sequence::parse(&source.content)?)),
        "class" => Ok(DiagramAst::Class(class::parse(&source.content)?)),
        "activity" => Ok(DiagramAst::Activity(activity::parse(&source.content)?)),
        "state" => Ok(DiagramAst::State(state::parse(&source.content)?)),
        "" => {
            // Auto-detect failed — try sequence first, then class
            sequence::parse(&source.content)
                .map(DiagramAst::Sequence)
                .or_else(|_| class::parse(&source.content).map(DiagramAst::Class))
        }
        other => {
            eprintln!(
                "puml: warning: unsupported diagram type '{}', attempting sequence parse",
                other
            );
            Ok(DiagramAst::Sequence(sequence::parse(&source.content)?))
        }
    }
}

fn detect_type(source: &str) -> String {
    // Two-pass detection: strong markers win over generic arrow heuristics,
    // even if they appear later in the source.
    let mut has_state = false;
    let mut has_activity = false;
    let mut has_class = false;
    let mut has_sequence_strong = false;
    let mut has_arrow = false;

    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }

        if t.starts_with("state ") || t.contains("[*]") {
            has_state = true;
        }
        if t == "start"
            || t == "stop"
            || t.starts_with("if (")
            || t.starts_with("while (")
            || t == "repeat"
            || t.starts_with("repeat while")
            || t == "fork"
            || t == "fork again"
            || t.starts_with("partition ")
            || (t.starts_with(':') && t.ends_with(';'))
        {
            has_activity = true;
        }
        if t.starts_with("class ")
            || t.starts_with("interface ")
            || t.starts_with("abstract ")
            || t.starts_with("enum ")
            || t.contains("--|>")
            || t.contains("<|--")
            || t.contains("--*")
            || t.contains("*--")
            || t.contains("--o")
            || t.contains("o--")
        {
            has_class = true;
        }
        if t.starts_with("participant")
            || t.starts_with("actor")
            || t.starts_with("boundary")
            || t.starts_with("control")
            || t.starts_with("entity")
            || t.starts_with("database")
            || t.starts_with("collections")
            || t.starts_with("queue")
            || t.starts_with("activate")
            || t.starts_with("deactivate")
            || t.starts_with("autonumber")
        {
            has_sequence_strong = true;
        }
        if t.contains("->") || t.contains("-->") {
            has_arrow = true;
        }
    }

    // Priority: strong diagram-specific markers > arrow-only detection
    if has_state {
        return "state".to_string();
    }
    if has_activity {
        return "activity".to_string();
    }
    if has_class {
        return "class".to_string();
    }
    if has_sequence_strong || has_arrow {
        return "sequence".to_string();
    }
    "sequence".to_string()
}
