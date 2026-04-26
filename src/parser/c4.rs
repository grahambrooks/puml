//! C4 model macro translation.
//!
//! PlantUML's C4-PlantUML stdlib (`!include <C4/Container>` etc.) defines
//! function-style macros — `Person(alias, "label", "description")`,
//! `System(...)`, `Container(...)`, `Rel(from, to, "label", "tech")`, …
//! Rather than build a parallel AST + renderer, we translate each macro
//! call into existing puml class + relation syntax at preprocess time. The
//! existing class layout/render handles the rest.
//!
//! Translation runs only when a `!include <C4/...>` directive is present in
//! the source — the rest of puml stays untouched for non-C4 diagrams.
//!
//! What's intentionally left out for the MVP:
//! - `*_Boundary(...) { … }` blocks render as flat children with an `@note`
//!   marker — proper container shapes need composite-state container support
//!   (a separate task).
//! - `LAYOUT_TOP_DOWN()`, `LAYOUT_LEFT_RIGHT()`, `LAYOUT_WITH_LEGEND()` are
//!   ignored. Layout direction always follows the existing class engine.
//! - Sprite icons, tags, custom rel types — silently passed through; if they
//!   trip the class grammar the user gets a parse error pointing to the line.

/// Does the source pull in any C4-PlantUML stdlib file? Accepts the
/// canonical stdlib forms (`<C4/C4_Container>`, `<C4/Container>`), the
/// alternate `!includeurl` directive, and any URL containing `C4_` or
/// `C4-PlantUML` in the path.
pub fn is_c4_source(source: &str) -> bool {
    source.lines().any(|l| {
        let t = l.trim();
        let is_include = t.starts_with("!include") || t.starts_with("!includeurl");
        is_include
            && (t.contains("<C4/")
                || t.contains("/C4_")
                || t.contains("C4-PlantUML")
                || t.contains("c4-plantuml"))
    })
}

/// Translate a source string containing C4 macros into pure puml syntax.
/// No-op when the source has no C4 include marker.
pub fn translate(source: &str) -> String {
    if !is_c4_source(source) {
        return source.to_string();
    }

    let mut out = String::with_capacity(source.len() + 256);

    // `*_Boundary(...) {` opens a block whose closing `}` would otherwise
    // hit the class grammar as a stray brace. We swallow the brace and
    // emit `pumlC4Boundary` / `pumlC4BoundaryMember` skinparam sentinels so
    // the renderer can draw a labeled rectangle around the contained
    // classes after layout. Stack tracks the open boundary aliases so a
    // closing `}` can record the right end-of-boundary.
    let mut boundary_stack: Vec<String> = Vec::new();
    // Preamble must land *inside* the @startuml...@enduml block — anything
    // before the start marker is dropped by split_diagrams. We emit it
    // immediately after the first @startuml line we see, or at the top if
    // the source is bare (no markers).
    let mut preamble_emitted = false;
    let has_start_marker = source
        .lines()
        .any(|l| l.trim_start().starts_with("@startuml"));
    if !has_start_marker {
        out.push_str(C4_STYLE_PREAMBLE);
        preamble_emitted = true;
    }

    for line in source.lines() {
        let trimmed = line.trim();
        if !preamble_emitted && trimmed.starts_with("@startuml") {
            out.push_str(line);
            out.push('\n');
            out.push_str(C4_STYLE_PREAMBLE);
            preamble_emitted = true;
            continue;
        }
        if is_c4_include_line(trimmed) || is_c4_layout_line(trimmed) {
            continue;
        }
        if let Some(macro_call) = parse_macro_call(trimmed) {
            let lower = macro_call.name.to_ascii_lowercase();
            // `Deployment_Node(...) {` and `Node(...) {` are containers that
            // hold other nodes/instances. Treat them as boundaries so the
            // contained elements render flat with a labeled box around them.
            // Without the trailing `{` they fall through to the standard
            // class translation below.
            let is_block_node = matches!(
                lower.as_str(),
                "deployment_node" | "node" | "node_l" | "node_r" | "node_b"
            ) && macro_call.has_block_open;
            if (lower.ends_with("_boundary") || lower == "boundary" || is_block_node)
                && macro_call.has_block_open
            {
                let alias = macro_call.args.first().cloned().unwrap_or_default();
                let label = macro_call.args.get(1).cloned().unwrap_or_default();
                let kind = if is_block_node {
                    let type_arg = macro_call.args.get(2).cloned().unwrap_or_default();
                    if type_arg.is_empty() {
                        "deployment node".into()
                    } else {
                        format!("deployment node: {}", type_arg)
                    }
                } else {
                    boundary_kind(&macro_call)
                };
                if !alias.is_empty() {
                    out.push_str(&format!(
                        "skinparam pumlC4Boundary {}|{}|{}\n",
                        alias, label, kind
                    ));
                    boundary_stack.push(alias);
                }
                continue;
            }
            if let Some(expansion) = expand(&macro_call) {
                out.push_str(&expansion);
                out.push('\n');
                if let Some(alias) = first_arg(&macro_call) {
                    // Membership records to *every* boundary on the stack —
                    // a node nested two levels deep belongs to both the
                    // inner and outer boundary, so they both grow to wrap it.
                    for boundary in &boundary_stack {
                        out.push_str(&format!(
                            "skinparam pumlC4BoundaryMember {}|{}\n",
                            boundary, alias
                        ));
                    }
                }
                continue;
            }
        }
        if trimmed == "}" && !boundary_stack.is_empty() {
            boundary_stack.pop();
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Best-effort C4 boundary kind: `Enterprise_Boundary` → "enterprise",
/// `System_Boundary` → "system", `Container_Boundary` → "container",
/// otherwise empty.
fn boundary_kind(call: &MacroCall) -> String {
    let lower = call.name.to_ascii_lowercase();
    if lower.starts_with("enterprise") {
        "enterprise".into()
    } else if lower.starts_with("system") {
        "system".into()
    } else if lower.starts_with("container") {
        "container".into()
    } else {
        // Some C4-PlantUML variants pass the kind as a third argument.
        call.args.get(2).cloned().unwrap_or_default()
    }
}

/// Pull the alias (first arg) out of a macro call we just expanded, so the
/// translator can record boundary membership immediately afterwards.
fn first_arg(call: &MacroCall) -> Option<String> {
    let kind = call.name.to_ascii_lowercase();
    // Only meaningful for class-producing macros — ignore `Rel`, `BiRel`,
    // `LAYOUT_*`, etc.
    let is_class_macro = matches!(
        kind.as_str(),
        "person"
            | "person_ext"
            | "system"
            | "system_ext"
            | "systemdb"
            | "systemdb_ext"
            | "systemqueue"
            | "systemqueue_ext"
            | "container"
            | "container_ext"
            | "containerdb"
            | "containerdb_ext"
            | "containerqueue"
            | "component"
            | "component_ext"
            | "componentdb"
            | "componentqueue"
            | "deployment_node"
            | "node"
            | "node_l"
            | "node_r"
            | "node_b"
            | "containerinstance"
            | "componentinstance"
    );
    if !is_class_macro {
        return None;
    }
    call.args.first().cloned()
}

fn is_c4_include_line(t: &str) -> bool {
    let is_include = t.starts_with("!include") || t.starts_with("!includeurl");
    is_include
        && (t.contains("<C4/")
            || t.contains("/C4_")
            || t.contains("C4-PlantUML")
            || t.contains("c4-plantuml"))
}

fn is_c4_layout_line(t: &str) -> bool {
    // C4-PlantUML carries a long tail of layout / theming / metadata macros
    // that we don't model. Drop the no-op ones up-front so they don't reach
    // the class grammar and trip a parse error. Anything we don't recognise
    // here passes through and gets handled (or rejected) downstream.
    t.starts_with("LAYOUT_")
        || t.starts_with("Lay_")
        || t.starts_with("UpdateElementStyle")
        || t.starts_with("UpdateRelStyle")
        || t.starts_with("UpdateBoundaryStyle")
        || t.starts_with("AddElementTag")
        || t.starts_with("AddRelTag")
        || t.starts_with("AddBoundaryTag")
        || t.starts_with("AddProperty")
        || t.starts_with("WithoutPropertyHeader")
        || t.starts_with("SetPropertyHeader")
        || t.starts_with("SHOW_FLOATING_LEGEND")
        || t.starts_with("SHOW_LEGEND")
        || t == "SHOW_PERSON_OUTLINE()"
        || t == "HIDE_STEREOTYPE()"
        || t == "HIDE_PERSON_OUTLINE()"
}

#[derive(Debug)]
struct MacroCall<'a> {
    name: &'a str,
    args: Vec<String>,
    has_block_open: bool,
}

/// Parse `Name(arg1, "arg with spaces", arg3) {` (trailing `{` optional)
/// into name + arg list. Returns `None` for anything that doesn't look like
/// a macro call so the caller can fall back to passing the line through.
fn parse_macro_call(line: &str) -> Option<MacroCall<'_>> {
    let open = line.find('(')?;
    let name = line[..open].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    // Tolerate a trailing `{` (used by `*_Boundary` blocks) after the
    // closing paren — find the matching `)` by scanning, respecting quoted
    // strings.
    let after_name = &line[open + 1..];
    let (args_raw, tail) = split_at_matching_paren(after_name)?;
    let has_block_open = tail.trim_start().starts_with('{');
    let args = split_args(args_raw);
    Some(MacroCall {
        name,
        args,
        has_block_open,
    })
}

fn split_at_matching_paren(s: &str) -> Option<(&str, &str)> {
    let mut depth = 1;
    let mut in_quote = false;
    for (i, ch) in s.char_indices() {
        match ch {
            '"' => in_quote = !in_quote,
            '(' if !in_quote => depth += 1,
            ')' if !in_quote => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[..i], &s[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// Split a comma-delimited C4 argument list, respecting double-quoted
/// strings (which may themselves contain commas). Strips one level of
/// surrounding quotes from each argument.
fn split_args(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    let mut depth = 0;
    for ch in s.chars() {
        match ch {
            '"' => {
                in_quote = !in_quote;
                cur.push(ch);
            }
            '(' if !in_quote => {
                depth += 1;
                cur.push(ch);
            }
            ')' if !in_quote => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if !in_quote && depth == 0 => {
                out.push(unquote_trim(&cur));
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(unquote_trim(&cur));
    }
    out
}

fn unquote_trim(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Render one macro call as one or more puml lines. Returns `None` for
/// unrecognised macros so the original line is passed through (and the user
/// sees a parse error if it's not valid puml).
fn expand(call: &MacroCall) -> Option<String> {
    // Person, Person_Ext
    let kind = call.name;
    let lower = kind.to_ascii_lowercase();
    let tail_block = if call.has_block_open { " {" } else { "" };

    // People
    if matches!(lower.as_str(), "person" | "person_ext") {
        let alias = call.args.first()?.as_str();
        let label = call
            .args
            .get(1)
            .cloned()
            .unwrap_or_else(|| alias.to_string());
        let stereo = if lower == "person_ext" {
            "person external"
        } else {
            "person"
        };
        return Some(format!(
            "class \"{}\" as {} <<{}>>{}",
            label, alias, stereo, tail_block
        ));
    }

    // Systems
    if matches!(
        lower.as_str(),
        "system" | "system_ext" | "systemdb" | "systemdb_ext" | "systemqueue" | "systemqueue_ext"
    ) {
        let alias = call.args.first()?.as_str();
        let label = call
            .args
            .get(1)
            .cloned()
            .unwrap_or_else(|| alias.to_string());
        let kw = if lower.contains("db") {
            "database"
        } else if lower.contains("queue") {
            "queue"
        } else {
            "rectangle"
        };
        let mut stereo = String::from("system");
        if lower.contains("ext") {
            stereo.push_str(" external");
        }
        return Some(format!(
            "{} \"{}\" as {} <<{}>>{}",
            kw, label, alias, stereo, tail_block
        ));
    }

    // Containers — `Container(alias, label, technology, desc?)`
    if matches!(
        lower.as_str(),
        "container" | "container_ext" | "containerdb" | "containerdb_ext" | "containerqueue"
    ) {
        let alias = call.args.first()?.as_str();
        let label = call
            .args
            .get(1)
            .cloned()
            .unwrap_or_else(|| alias.to_string());
        let tech = call.args.get(2).cloned().unwrap_or_default();
        let kw = if lower.contains("db") {
            "database"
        } else if lower.contains("queue") {
            "queue"
        } else {
            "component"
        };
        let mut stereo = String::from("container");
        if !tech.is_empty() {
            stereo.push_str(": ");
            stereo.push_str(&tech);
        }
        if lower.contains("ext") {
            stereo.push_str(" (external)");
        }
        return Some(format!(
            "{} \"{}\" as {} <<{}>>{}",
            kw, label, alias, stereo, tail_block
        ));
    }

    // Components
    if matches!(
        lower.as_str(),
        "component" | "component_ext" | "componentdb" | "componentqueue"
    ) {
        let alias = call.args.first()?.as_str();
        let label = call
            .args
            .get(1)
            .cloned()
            .unwrap_or_else(|| alias.to_string());
        let tech = call.args.get(2).cloned().unwrap_or_default();
        let kw = if lower.contains("db") {
            "database"
        } else if lower.contains("queue") {
            "queue"
        } else {
            "component"
        };
        let mut stereo = String::from("component");
        if !tech.is_empty() {
            stereo.push_str(": ");
            stereo.push_str(&tech);
        }
        if lower.contains("ext") {
            stereo.push_str(" (external)");
        }
        return Some(format!(
            "{} \"{}\" as {} <<{}>>{}",
            kw, label, alias, stereo, tail_block
        ));
    }

    // Boundaries — for now, drop the wrapper (inline contents) and emit a
    // note-style title above. Composite container support will replace this.
    if lower.ends_with("_boundary") || lower == "boundary" {
        let label = call
            .args
            .get(1)
            .cloned()
            .unwrap_or_else(|| call.args.first().cloned().unwrap_or_default());
        // No `class` to attach a note to here — just emit a comment so the
        // original intent isn't silently lost.
        return Some(format!("' [{}: {}]", call.name, label));
    }

    // Deployment-level: `Deployment_Node(alias, label, type, descr?)`,
    // `Node(alias, label, type, descr?)`, `Node_L`/`Node_R`. All render as
    // the existing `node` shape (a 3D box) with the type carried through as
    // a stereotype.
    if matches!(
        lower.as_str(),
        "deployment_node" | "node" | "node_l" | "node_r" | "node_b"
    ) {
        let alias = call.args.first()?.as_str();
        let label = call
            .args
            .get(1)
            .cloned()
            .unwrap_or_else(|| alias.to_string());
        let kind_arg = call.args.get(2).cloned().unwrap_or_default();
        let stereo = if kind_arg.is_empty() {
            "deployment node".to_string()
        } else {
            format!("deployment node: {}", kind_arg)
        };
        return Some(format!(
            "node \"{}\" as {} <<{}>>{}",
            label, alias, stereo, tail_block
        ));
    }

    // Deployment instances: `ContainerInstance(alias, container_alias)` and
    // `ComponentInstance(alias, component_alias)`. The original instance
    // semantics (a deployed copy of an upstream class) collapse into a
    // simple component box for the MVP — the upstream alias appears in the
    // stereotype so the link is still legible to the reader.
    if matches!(lower.as_str(), "containerinstance" | "componentinstance") {
        let alias = call.args.first()?.as_str();
        let target = call.args.get(1).cloned().unwrap_or_default();
        let label = if target.is_empty() {
            alias.to_string()
        } else {
            target.clone()
        };
        let stereo = if lower == "containerinstance" {
            "container instance"
        } else {
            "component instance"
        };
        let stereo_full = if target.is_empty() {
            stereo.to_string()
        } else {
            format!("{}: {}", stereo, target)
        };
        return Some(format!(
            "component \"{}\" as {} <<{}>>",
            label, alias, stereo_full
        ));
    }

    // Relations.
    //
    // Standard: `Rel(from, to, label, ?tech)`.
    // Directional variants `Rel_U/D/L/R/Up/Down/Left/Right` carry a
    // direction hint we don't honour yet — translate to the same arrow.
    // Reverse variants `Rel_Back`, `Rel_Back_Up` etc. flip the arrow.
    // Numbered variants `RelIndex(n, from, to, label)` and
    // `Rel_with_index(n, ...)` prepend `(n)` to the label so the call
    // sequence stays visible in dynamic diagrams.
    if lower.starts_with("rel") || lower == "birel" {
        let is_indexed = lower.starts_with("relindex") || lower.starts_with("rel_with_index");
        let (idx_str, arg_offset) = if is_indexed {
            (call.args.first().cloned().unwrap_or_default(), 1)
        } else {
            (String::new(), 0)
        };
        let from = call.args.get(arg_offset)?.as_str();
        let to = call.args.get(arg_offset + 1)?.as_str();
        let label = call.args.get(arg_offset + 2).cloned().unwrap_or_default();
        let tech = call.args.get(arg_offset + 3).cloned().unwrap_or_default();

        let arrow = if lower.starts_with("birel") {
            "<-->"
        } else if lower.starts_with("rel_back") {
            "<--"
        } else {
            "-->"
        };
        let labelled = if !idx_str.is_empty() {
            format!("({}) {}", idx_str.trim(), label).trim().to_string()
        } else {
            label
        };
        let label_part = match (labelled.as_str(), tech.as_str()) {
            ("", "") => String::new(),
            (l, "") => format!(" : {}", l),
            ("", t) => format!(" : [{}]", t),
            (l, t) => format!(" : {} [{}]", l, t),
        };
        return Some(format!("{} {} {}{}", from, arrow, to, label_part));
    }

    None
}

/// Skinparam preamble injected into every C4 diagram.
///
/// `pumlC4Mode true` is a sentinel the class parser intercepts to flip rank
/// propagation so callers render above callees (the C4 convention) instead
/// of below (the UML class-dependency convention).
///
/// The remaining colour skinparams are advisory metadata for future
/// stereotype-driven theming.
const C4_STYLE_PREAMBLE: &str = "\
skinparam pumlC4Mode true
skinparam c4_person_color #08427B
skinparam c4_system_color #1168BD
skinparam c4_system_external_color #999999
skinparam c4_container_color #438DD5
skinparam c4_component_color #85BBF0
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_through_when_no_c4_include() {
        let src = "@startuml\nclass Foo\n@enduml\n";
        assert_eq!(translate(src), src);
    }

    #[test]
    fn detects_angle_bracket_include() {
        assert!(is_c4_source("!include <C4/Container>"));
        assert!(is_c4_source("!include <C4/Context>"));
    }

    #[test]
    fn person_macro_expands_to_class_with_stereotype() {
        let src = "!include <C4/Container>\nPerson(customer, \"Customer\", \"Buys things\")\n";
        let out = translate(src);
        assert!(out.contains("class \"Customer\" as customer <<person>>"));
        // include line removed
        assert!(!out.contains("!include"));
    }

    #[test]
    fn rel_macro_expands_with_label_and_tech() {
        let src = "!include <C4/Container>\nRel(a, b, \"uses\", \"HTTPS\")\n";
        let out = translate(src);
        assert!(out.contains("a --> b : uses [HTTPS]"));
    }

    #[test]
    fn birel_emits_bidirectional_arrow() {
        let src = "!include <C4/Container>\nBiRel(a, b, \"sync\")\n";
        let out = translate(src);
        assert!(out.contains("a <--> b : sync"));
    }

    #[test]
    fn container_with_tech_carries_into_stereotype() {
        let src = "!include <C4/Container>\nContainer(api, \"API\", \"Go\", \"REST\")\n";
        let out = translate(src);
        assert!(
            out.contains("<<container: Go>>"),
            "expected stereotype with tech, got: {out}"
        );
    }

    #[test]
    fn quoted_string_with_comma_keeps_arg_intact() {
        // C4 labels often contain commas inside quoted strings.
        let src = "!include <C4/Container>\nSystem(s, \"Big, Important System\")\n";
        let out = translate(src);
        assert!(out.contains("\"Big, Important System\""), "got: {out}");
    }

    #[test]
    fn layout_calls_are_dropped() {
        let src = "!include <C4/Container>\nLAYOUT_TOP_DOWN()\nPerson(a, \"A\")\n";
        let out = translate(src);
        assert!(!out.contains("LAYOUT_"));
    }

    #[test]
    fn lay_and_styling_macros_are_dropped() {
        let src = "!include <C4/Context>\nLay_R(a, b)\nUpdateElementStyle(person, $bgColor=\"red\")\nAddElementTag(\"thing\", $bgColor=\"blue\")\n";
        let out = translate(src);
        assert!(!out.contains("Lay_"), "got: {out}");
        assert!(!out.contains("UpdateElementStyle"), "got: {out}");
        assert!(!out.contains("AddElementTag"), "got: {out}");
    }

    #[test]
    fn deployment_node_emits_node_shape() {
        let src = "!include <C4/Deployment>\nDeployment_Node(server, \"Web Server\", \"Linux\")\n";
        let out = translate(src);
        assert!(
            out.contains("node \"Web Server\" as server <<deployment node: Linux>>"),
            "got: {out}"
        );
    }

    #[test]
    fn container_instance_carries_target_alias() {
        let src = "!include <C4/Deployment>\nContainerInstance(api1, api)\n";
        let out = translate(src);
        assert!(
            out.contains("component \"api\" as api1 <<container instance: api>>"),
            "got: {out}"
        );
    }

    #[test]
    fn rel_index_prepends_step_number() {
        let src = "!include <C4/Dynamic>\nRelIndex(1, a, b, \"requests\")\n";
        let out = translate(src);
        assert!(out.contains("a --> b : (1) requests"), "got: {out}");
    }

    #[test]
    fn rel_back_emits_reverse_arrow() {
        let src = "!include <C4/Container>\nRel_Back(a, b, \"observed by\")\n";
        let out = translate(src);
        assert!(out.contains("a <-- b : observed by"), "got: {out}");
    }

    #[test]
    fn directional_rel_variants_translate_to_normal_arrow() {
        let src = "!include <C4/Container>\nRel_R(a, b, \"calls\")\nRel_U(c, d, \"reads\")\n";
        let out = translate(src);
        assert!(out.contains("a --> b : calls"), "got: {out}");
        assert!(out.contains("c --> d : reads"), "got: {out}");
    }
}
