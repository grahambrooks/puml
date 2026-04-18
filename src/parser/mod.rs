pub mod activity;
pub mod class;
pub mod preprocessor;
pub mod sequence;
pub mod state;
pub mod timing;
pub mod usecase;

/// Parse a `skinparam key value` line (trailing newline ok) into `(key, value)`.
/// Returns None for empty pairs or block forms.
pub(crate) fn extract_skinparam(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let after_kw = trimmed
        .strip_prefix("skinparam")
        .or_else(|| trimmed.strip_prefix("Skinparam"))
        .or_else(|| trimmed.strip_prefix("SKINPARAM"))
        .map(str::trim_start)
        .unwrap_or(trimmed);
    // Skip block form: `skinparam Type { ... }`
    if after_kw.contains('{') {
        return None;
    }
    let (key, value) = after_kw.split_once(char::is_whitespace)?;
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), value.to_string()))
}

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
        "usecase" => Ok(DiagramAst::UseCase(usecase::parse(&source.content)?)),
        "timing" => Ok(DiagramAst::Timing(timing::parse(&source.content)?)),
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
    let mut has_usecase = false;
    let mut has_timing = false;

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
        // Class-family markers. `database` and `queue` are intentionally NOT
        // listed here — they're ambiguous with sequence participant types,
        // and the class grammar still accepts them if the file carries other
        // class/deployment markers that win routing.
        if t.starts_with("class ")
            || t.starts_with("interface ")
            || t.starts_with("abstract ")
            || t.starts_with("enum ")
            || t.starts_with("object ")
            || t.starts_with("component ")
            || t.starts_with("node ")
            || t.starts_with("cloud ")
            || t.starts_with("folder ")
            || t.starts_with("frame ")
            || t.starts_with("rectangle ")
            || t.starts_with("artifact ")
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
        // Use-case markers: explicit keyword OR standalone bracketed name.
        // `(X)` and `:X:` must occupy the entire line to avoid conflict with
        // sequence notes and message labels.
        if t.starts_with("usecase ")
            || is_standalone_bracketed(t, '(', ')')
            || is_standalone_bracketed(t, ':', ':')
        {
            has_usecase = true;
        }
        // Timing markers: lane-type keywords and `@N` time markers are
        // unique to timing. `robust` is the strongest signal.
        if t.starts_with("robust ")
            || t.starts_with("concise ")
            || t.starts_with("clock ")
            || t.starts_with("binary ")
            || (t.starts_with('@') && t.len() > 1 && t[1..].chars().all(|c| c.is_ascii_digit()))
        {
            has_timing = true;
        }
    }

    // Priority: strong diagram-specific markers > arrow-only detection
    if has_timing {
        return "timing".to_string();
    }
    if has_state {
        return "state".to_string();
    }
    if has_activity {
        return "activity".to_string();
    }
    if has_class {
        return "class".to_string();
    }
    // Use case before generic sequence: shorthand (X) / :X: shouldn't be
    // mistaken for sequence messages since those contain arrows.
    if has_usecase && !has_sequence_strong {
        return "usecase".to_string();
    }
    if has_sequence_strong || has_arrow {
        return "sequence".to_string();
    }
    "sequence".to_string()
}

/// True if `t` is `<open><inner><close>` with `inner` non-empty and containing
/// no other instance of `close` — e.g. `(Login)` or `:User:`.
fn is_standalone_bracketed(t: &str, open: char, close: char) -> bool {
    if t.len() < 3 {
        return false;
    }
    let first = t.chars().next().unwrap();
    let last = t.chars().last().unwrap();
    if first != open || last != close {
        return false;
    }
    let inner = &t[open.len_utf8()..t.len() - close.len_utf8()];
    !inner.is_empty() && !inner.contains(close)
}
