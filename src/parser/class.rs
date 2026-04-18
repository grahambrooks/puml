use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::class::*;
use crate::error::PumlError;

#[derive(Parser)]
#[grammar = "parser/grammars/class.pest"]
struct ClassParser;

pub fn parse(source: &str) -> Result<ClassDiagram, PumlError> {
    let pairs = ClassParser::parse(Rule::diagram, source).map_err(|e| {
        let line = match e.line_col {
            pest::error::LineColLocation::Pos((l, _)) => l,
            pest::error::LineColLocation::Span((l, _), _) => l,
        };
        PumlError::Parse {
            line,
            message: e.to_string(),
        }
    })?;

    let mut diagram = ClassDiagram::default();

    for pair in pairs {
        for stmt in pair.into_inner() {
            match stmt.as_rule() {
                Rule::title_stmt => {
                    diagram.title = Some(stmt.into_inner().as_str().trim().to_string());
                }
                Rule::class_def => {
                    let node = parse_class(stmt);
                    if let Some(existing) = diagram.classes.iter_mut().find(|c| c.name == node.name)
                    {
                        // Merge members if the class was forward-declared by a relation
                        existing.kind = node.kind;
                        existing.stereotype = node.stereotype;
                        existing.color = node.color;
                        if !node.members.is_empty() {
                            existing.members = node.members;
                        }
                    } else {
                        diagram.classes.push(node);
                    }
                }
                Rule::relation_stmt => {
                    let (rel, from, to) = parse_relation(stmt);
                    ensure_class(&mut diagram.classes, &from);
                    ensure_class(&mut diagram.classes, &to);
                    diagram.relations.push(rel);
                }
                Rule::hide_stmt => {
                    let content = stmt.into_inner().as_str().trim().to_lowercase();
                    if content.contains("empty members") || content.contains("empty attributes") {
                        diagram.hide_empty_members = true;
                    }
                }
                Rule::namespace_stmt => {
                    parse_namespace(stmt, &mut diagram);
                }
                Rule::note_stmt => {
                    if let Some(note) = parse_note(stmt) {
                        diagram.notes.push(note);
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

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn ensure_class(classes: &mut Vec<ClassNode>, name: &str) {
    if !classes.iter().any(|c| c.name == name) {
        classes.push(ClassNode {
            name: name.to_string(),
            generics: None,
            kind: ClassKind::Class,
            stereotype: None,
            color: None,
            members: Vec::new(),
            namespace: None,
        });
    }
}

fn parse_class(pair: Pair<Rule>) -> ClassNode {
    let mut kind = ClassKind::Class;
    let mut name = String::new();
    let mut generics: Option<String> = None;
    let mut stereotype = None;
    let mut color = None;
    let mut members = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::class_kw => {
                let kw = inner.as_str().to_lowercase();
                kind = if kw.contains("interface") {
                    ClassKind::Interface
                } else if kw.contains("abstract") {
                    ClassKind::Abstract
                } else if kw.contains("enum") {
                    ClassKind::Enum
                } else if kw.contains("annotation") {
                    ClassKind::Annotation
                } else {
                    ClassKind::Class
                };
            }
            Rule::class_name => {
                let (bare, gen) = parse_class_name(inner);
                name = bare;
                generics = gen;
            }
            Rule::stereotype => {
                let raw = inner.as_str();
                stereotype = Some(raw[2..raw.len() - 2].trim().to_string());
            }
            Rule::color => {
                color = Some(inner.as_str().trim().to_string());
            }
            Rule::member => {
                if let Some(m) = parse_member(inner) {
                    members.push(m);
                }
            }
            _ => {}
        }
    }

    ClassNode {
        name,
        generics,
        kind,
        stereotype,
        color,
        members,
        namespace: None,
    }
}

fn parse_class_name(pair: Pair<Rule>) -> (String, Option<String>) {
    let raw = pair.as_str();
    if raw.starts_with('"') && raw.ends_with('"') {
        return (raw[1..raw.len() - 1].to_string(), None);
    }
    let mut bare_name_str = String::new();
    let mut generics: Option<String> = None;
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::class_bare_name {
            for part in inner.into_inner() {
                match part.as_rule() {
                    Rule::bare_name => bare_name_str = part.as_str().to_string(),
                    Rule::generic_params => {
                        let gs = part.as_str();
                        generics = Some(gs[1..gs.len() - 1].trim().to_string());
                    }
                    _ => {}
                }
            }
        }
    }
    if bare_name_str.is_empty() {
        bare_name_str = unquote(raw);
    }
    (bare_name_str, generics)
}

fn parse_note(pair: Pair<Rule>) -> Option<ClassNote> {
    let inner = pair.into_inner().next()?;
    let mut position = NotePosition::Right;
    let mut target = String::new();
    let mut lines: Vec<String> = Vec::new();
    for p in inner.into_inner() {
        match p.as_rule() {
            Rule::note_pos => {
                position = match p.as_str().to_lowercase().as_str() {
                    "left" => NotePosition::Left,
                    "top" => NotePosition::Top,
                    "bottom" => NotePosition::Bottom,
                    _ => NotePosition::Right,
                };
            }
            Rule::class_ref => {
                target = unquote(p.as_str());
            }
            Rule::rest_of_line => {
                lines.push(p.as_str().trim().to_string());
            }
            Rule::note_line => {
                lines.push(p.as_str().trim_end_matches('\n').to_string());
            }
            _ => {}
        }
    }
    Some(ClassNote {
        position,
        target,
        lines,
    })
}

fn parse_member(pair: Pair<Rule>) -> Option<Member> {
    let inner = pair.into_inner().next()?;
    match inner.as_rule() {
        Rule::separator_line => None, // visual separator — skip
        Rule::member_line => {
            let mut visibility = Visibility::None;
            let mut is_static = false;
            let mut is_abstract = false;
            let mut name = String::new();
            let mut type_annotation = None;
            let mut is_method = false;
            let mut params = None;

            for p in inner.into_inner() {
                match p.as_rule() {
                    Rule::visibility => {
                        visibility = match p.as_str() {
                            "+" => Visibility::Public,
                            "-" => Visibility::Private,
                            "#" => Visibility::Protected,
                            "~" => Visibility::Package,
                            _ => Visibility::None,
                        };
                    }
                    Rule::modifier => {
                        let m = p.as_str().to_lowercase();
                        if m.contains("static") {
                            is_static = true;
                        }
                        if m.contains("abstract") {
                            is_abstract = true;
                        }
                    }
                    Rule::member_name => {
                        name = p.as_str().to_string();
                    }
                    Rule::params => {
                        is_method = true;
                        params = Some(p.as_str().to_string());
                    }
                    Rule::type_name => {
                        type_annotation = Some(p.as_str().trim().to_string());
                    }
                    _ => {}
                }
            }

            if name.is_empty() {
                return None;
            }

            Some(Member {
                visibility,
                name,
                type_annotation,
                is_static,
                is_abstract,
                is_method,
                params,
            })
        }
        _ => None,
    }
}

fn parse_relation(pair: Pair<Rule>) -> (Relation, String, String) {
    let mut parts = pair.into_inner();

    let from_raw = parts
        .next()
        .map(|p| unquote(p.as_str()))
        .unwrap_or_default();

    // Consume optional card_label before arrow
    let mut next = parts.next();
    let from_label = if next.as_ref().map(|p| p.as_rule()) == Some(Rule::card_label) {
        let lbl = next.map(|p| p.as_str().trim_matches('"').to_string());
        next = parts.next();
        lbl
    } else {
        None
    };

    let arrow_str = next
        .map(|p| p.as_str().trim().to_string())
        .unwrap_or_default();

    let mut next = parts.next();
    let to_label = if next.as_ref().map(|p| p.as_rule()) == Some(Rule::card_label) {
        let lbl = next.map(|p| p.as_str().trim_matches('"').to_string());
        next = parts.next();
        lbl
    } else {
        None
    };

    let to_raw = next.map(|p| unquote(p.as_str())).unwrap_or_default();
    let label = parts.next().map(|p| p.as_str().trim().to_string());

    let (kind, reversed) = parse_arrow(&arrow_str);

    let rel = Relation {
        from: if reversed {
            to_raw.clone()
        } else {
            from_raw.clone()
        },
        to: if reversed {
            from_raw.clone()
        } else {
            to_raw.clone()
        },
        kind,
        from_label,
        to_label,
        label,
        reversed,
    };

    (rel, from_raw, to_raw)
}

fn parse_arrow(s: &str) -> (RelationKind, bool) {
    match s {
        "--|>" | "-|>" => (RelationKind::Extension, false),
        "<|--" | "<|-" => (RelationKind::Extension, true),
        "..|>" | ".|>" => (RelationKind::Implementation, false),
        "<|.." | "<|." => (RelationKind::Implementation, true),
        "--*" | "-*" => (RelationKind::Composition, false),
        "*--" | "*-" => (RelationKind::Composition, true),
        "--o" | "-o" => (RelationKind::Aggregation, false),
        "o--" | "o-" => (RelationKind::Aggregation, true),
        "-->" | "->" => (RelationKind::Dependency, false),
        "<--" | "<-" => (RelationKind::Dependency, true),
        "..>" | ".>" => (RelationKind::DashedLink, false),
        "<.." | "<." => (RelationKind::DashedLink, true),
        "--" => (RelationKind::Association, false),
        ".." => (RelationKind::DashedLink, false),
        _ => (RelationKind::Association, false),
    }
}

fn parse_namespace(pair: Pair<Rule>, diagram: &mut ClassDiagram) {
    let mut ns_name = String::new();
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::bare_name => {
                ns_name = inner.as_str().to_string();
            }
            Rule::class_def => {
                let mut node = parse_class(inner);
                node.namespace = Some(ns_name.clone());
                diagram.classes.push(node);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_class() {
        let src = "class Animal {\n  +name: String\n  +speak()\n}\n";
        let diagram = parse(src).unwrap();
        assert_eq!(diagram.classes.len(), 1);
        assert_eq!(diagram.classes[0].name, "Animal");
        assert_eq!(diagram.classes[0].members.len(), 2);
    }

    #[test]
    fn parses_interface() {
        let src = "interface Drawable {\n  +draw()\n}\n";
        let d = parse(src).unwrap();
        assert_eq!(d.classes[0].kind, ClassKind::Interface);
    }

    #[test]
    fn parses_extension_relation() {
        let src = "Dog --|> Animal\n";
        let d = parse(src).unwrap();
        assert_eq!(d.relations.len(), 1);
        assert_eq!(d.relations[0].kind, RelationKind::Extension);
        assert_eq!(d.relations[0].from, "Dog");
        assert_eq!(d.relations[0].to, "Animal");
    }

    #[test]
    fn parses_reversed_relation() {
        let src = "Animal <|-- Dog\n";
        let d = parse(src).unwrap();
        assert_eq!(d.relations[0].from, "Dog");
        assert_eq!(d.relations[0].to, "Animal");
    }

    #[test]
    fn parses_member_visibility() {
        let src = "class Foo {\n  +pub\n  -priv\n  #prot\n}\n";
        let d = parse(src).unwrap();
        let members = &d.classes[0].members;
        assert_eq!(members[0].visibility, Visibility::Public);
        assert_eq!(members[1].visibility, Visibility::Private);
        assert_eq!(members[2].visibility, Visibility::Protected);
    }

    #[test]
    fn parses_enum() {
        let src = "enum Direction {\n  NORTH\n  SOUTH\n}\n";
        let d = parse(src).unwrap();
        assert_eq!(d.classes[0].kind, ClassKind::Enum);
    }
}
