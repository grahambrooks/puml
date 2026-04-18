pub mod preprocessor;
pub mod sequence;

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
        "sequence" | "" => Ok(DiagramAst::Sequence(sequence::parse(&source.content)?)),
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
    // Simple heuristics
    for line in source.lines() {
        let t = line.trim();
        if t.contains("->")
            || t.contains("-->")
            || t.starts_with("participant")
            || t.starts_with("actor")
            || t.starts_with("activate")
            || t.starts_with("deactivate")
        {
            return "sequence".to_string();
        }
        if t.starts_with("class ") || t.contains("--|>") || t.contains("--*") {
            return "class".to_string();
        }
        if t.starts_with("start") || t.starts_with(":") && t.ends_with(";") {
            return "activity".to_string();
        }
        if t.starts_with("state ") || t.contains("[*]") {
            return "state".to_string();
        }
    }
    "sequence".to_string() // default
}
