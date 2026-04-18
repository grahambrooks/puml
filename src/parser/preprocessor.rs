use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Output of preprocessing: one source block per @startuml...@enduml pair.
/// If there are no markers the entire source is treated as one block.
#[derive(Debug, Clone)]
pub struct DiagramSource {
    pub content: String,
    /// Hint supplied via @startuml(type)
    pub type_hint: Option<String>,
}

pub fn preprocess(source: &str, base_dir: Option<&Path>) -> Vec<DiagramSource> {
    let expanded = expand_includes(source, base_dir, &mut HashMap::new());
    let expanded = expand_defines(&expanded);
    split_diagrams(&expanded)
}

fn expand_includes(
    source: &str,
    base_dir: Option<&Path>,
    seen: &mut HashMap<PathBuf, ()>,
) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("!include ") {
            let path_str = rest.trim().trim_matches('"');
            let resolved = base_dir
                .map(|d| d.join(path_str))
                .unwrap_or_else(|| PathBuf::from(path_str));
            if seen.contains_key(&resolved) {
                continue;
            }
            seen.insert(resolved.clone(), ());
            match std::fs::read_to_string(&resolved) {
                Ok(contents) => {
                    let sub_dir = resolved.parent();
                    out.push_str(&expand_includes(&contents, sub_dir, seen));
                }
                Err(_) => eprintln!("puml: warning: could not include {:?}", resolved),
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn expand_defines(source: &str) -> String {
    let mut defines: HashMap<String, String> = HashMap::new();
    let mut out = String::with_capacity(source.len());

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("!define ") {
            let mut parts = rest.splitn(2, ' ');
            let name = parts.next().unwrap_or("").to_string();
            let value = parts.next().unwrap_or("").to_string();
            defines.insert(name, value);
        } else if let Some(theme_name) = trimmed.strip_prefix("!theme ") {
            // PlantUML `!theme foo` selects a named preset. Translate to an
            // internal skinparam so the theme layer can resolve it uniformly.
            out.push_str(&format!("skinparam theme {}\n", theme_name.trim()));
        } else if trimmed.starts_with("!") {
            // Unknown directive — pass through with a warning
            eprintln!("puml: warning: unknown directive: {}", trimmed);
            out.push_str(line);
            out.push('\n');
        } else {
            let mut result = line.to_string();
            for (k, v) in &defines {
                result = result.replace(k.as_str(), v.as_str());
            }
            out.push_str(&result);
            out.push('\n');
        }
    }
    out
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\'' {
            // Line comment — skip to end of line
            for c in chars.by_ref() {
                if c == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else if ch == '/' && chars.peek() == Some(&'\'') {
            // Block comment /' ... '/
            chars.next(); // consume '
            let mut prev = ' ';
            for c in chars.by_ref() {
                if prev == '\'' && c == '/' {
                    break;
                }
                if c == '\n' {
                    out.push('\n');
                }
                prev = c;
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Recognise every PlantUML start marker we support and return
/// `(type_hint, true)` on a match. A diagram-type-specific marker like
/// `@startmindmap` yields `Some("mindmap")`; the generic `@startuml` yields
/// either the explicit hint (`@startuml(sequence)`) or `None`.
fn parse_start_marker(line: &str) -> Option<(Option<String>, bool)> {
    if let Some(rest) = line.strip_prefix("@startuml") {
        let rest = rest.trim();
        let hint = if rest.starts_with('(') && rest.ends_with(')') {
            Some(rest[1..rest.len() - 1].trim().to_string())
        } else {
            None
        };
        return Some((hint, true));
    }
    for (marker, diag_type) in [
        ("@startmindmap", "mindmap"),
        ("@startgantt", "gantt"),
        ("@startwbs", "wbs"),
        ("@startsalt", "salt"),
        ("@startjson", "json"),
        ("@startyaml", "yaml"),
    ] {
        if line.starts_with(marker) {
            return Some((Some(diag_type.to_string()), true));
        }
    }
    None
}

fn is_end_marker(line: &str) -> bool {
    matches!(
        line,
        "@enduml" | "@endmindmap" | "@endgantt" | "@endwbs" | "@endsalt" | "@endjson" | "@endyaml"
    )
}

fn split_diagrams(source: &str) -> Vec<DiagramSource> {
    let cleaned = strip_comments(source);
    let mut diagrams: Vec<DiagramSource> = Vec::new();
    let mut current: Option<(Option<String>, String)> = None;
    let mut bare_lines: Vec<&str> = Vec::new();
    let mut has_markers = false;

    for line in cleaned.lines() {
        let trimmed = line.trim();
        if let Some((hint, is_start)) = parse_start_marker(trimmed) {
            if is_start {
                has_markers = true;
                current = Some((hint, String::new()));
                continue;
            }
        }
        if is_end_marker(trimmed) {
            if let Some((hint, content)) = current.take() {
                diagrams.push(DiagramSource {
                    content,
                    type_hint: hint,
                });
            }
            continue;
        }
        if let Some((_, ref mut content)) = current {
            content.push_str(line);
            content.push('\n');
        } else if !has_markers {
            bare_lines.push(line);
        }
    }

    if !has_markers && !bare_lines.is_empty() {
        let mut content = bare_lines.join("\n");
        content.push('\n');
        diagrams.push(DiagramSource {
            content,
            type_hint: None,
        });
    }

    diagrams
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_line_comments() {
        let src = "Alice -> Bob ' this is a comment\nBob -> Alice\n";
        let result = strip_comments(src);
        assert!(result.contains("Alice -> Bob"));
        assert!(!result.contains("this is a comment"));
    }

    #[test]
    fn strips_block_comments() {
        let src = "Alice /' block comment '/ -> Bob\n";
        let result = strip_comments(src);
        assert!(result.contains("Alice"));
        assert!(result.contains("-> Bob"));
        assert!(!result.contains("block comment"));
    }

    #[test]
    fn splits_on_markers() {
        let src = "@startuml\nAlice -> Bob\n@enduml\n@startuml\nBob -> Carol\n@enduml\n";
        let diagrams = preprocess(src, None);
        assert_eq!(diagrams.len(), 2);
    }

    #[test]
    fn bare_source_treated_as_single_diagram() {
        let src = "Alice -> Bob\n";
        let diagrams = preprocess(src, None);
        assert_eq!(diagrams.len(), 1);
        assert!(diagrams[0].content.contains("Alice -> Bob"));
    }

    #[test]
    fn type_hint_parsed() {
        let src = "@startuml(sequence)\nAlice -> Bob\n@enduml\n";
        let diagrams = preprocess(src, None);
        assert_eq!(diagrams[0].type_hint.as_deref(), Some("sequence"));
    }
}
