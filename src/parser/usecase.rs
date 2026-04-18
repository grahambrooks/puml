use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::usecase::*;
use crate::error::PumlError;

#[derive(Parser)]
#[grammar = "parser/grammars/usecase.pest"]
struct UseCaseParser;

pub fn parse(source: &str) -> Result<UseCaseDiagram, PumlError> {
    let pairs = UseCaseParser::parse(Rule::diagram, source).map_err(|e| {
        let line = match e.line_col {
            pest::error::LineColLocation::Pos((l, _)) => l,
            pest::error::LineColLocation::Span((l, _), _) => l,
        };
        PumlError::Parse {
            line,
            message: e.to_string(),
        }
    })?;

    let mut diagram = UseCaseDiagram::default();

    for pair in pairs {
        for stmt in pair.into_inner() {
            match stmt.as_rule() {
                Rule::title_stmt => {
                    diagram.title = Some(stmt.into_inner().as_str().trim().to_string());
                }
                Rule::actor_explicit => {
                    let node = parse_named(stmt, NodeKind::Actor);
                    ensure_node(&mut diagram.nodes, node);
                }
                Rule::usecase_explicit => {
                    let node = parse_named(stmt, NodeKind::UseCase);
                    ensure_node(&mut diagram.nodes, node);
                }
                Rule::actor_shorthand_stmt => {
                    let name = strip_delims(stmt.as_str(), ':', ':');
                    let node = UseCaseNode {
                        name: name.clone(),
                        label: None,
                        kind: NodeKind::Actor,
                        stereotype: None,
                    };
                    ensure_node(&mut diagram.nodes, node);
                }
                Rule::usecase_shorthand_stmt => {
                    let name = strip_delims(stmt.as_str(), '(', ')');
                    let node = UseCaseNode {
                        name: name.clone(),
                        label: None,
                        kind: NodeKind::UseCase,
                        stereotype: None,
                    };
                    ensure_node(&mut diagram.nodes, node);
                }
                Rule::association_stmt => {
                    if let Some(assoc) = parse_association(stmt, &mut diagram.nodes) {
                        diagram.associations.push(assoc);
                    }
                }
                Rule::skinparam_stmt => {
                    if let Some((k, v)) = super::extract_skinparam(stmt.as_str()) {
                        diagram.skinparams.push((k, v));
                    }
                }
                Rule::note_stmt | Rule::EOI => {}
                _ => {}
            }
        }
    }

    Ok(diagram)
}

fn parse_named(pair: Pair<Rule>, kind: NodeKind) -> UseCaseNode {
    let mut display: Option<String> = None;
    let mut alias: Option<String> = None;
    let mut stereotype = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
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
            Rule::stereotype => {
                let raw = inner.as_str();
                stereotype = Some(raw[2..raw.len() - 2].trim().to_string());
            }
            _ => {}
        }
    }

    let display = display.unwrap_or_default();
    let (name, label) = match alias {
        Some(a) => (a, Some(display)),
        None => (display, None),
    };

    UseCaseNode {
        name,
        label,
        kind,
        stereotype,
    }
}

fn parse_association(pair: Pair<Rule>, nodes: &mut Vec<UseCaseNode>) -> Option<Association> {
    let mut endpoints: Vec<(String, Option<NodeKind>)> = Vec::new();
    let mut arrow = String::new();
    let mut label: Option<String> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::endpoint => {
                endpoints.push(parse_endpoint(inner));
            }
            Rule::arrow => arrow = inner.as_str().trim().to_string(),
            Rule::association_label => {
                label = Some(inner.as_str().trim().to_string());
            }
            _ => {}
        }
    }

    if endpoints.len() < 2 {
        return None;
    }
    let (from_name, from_kind) = endpoints.remove(0);
    let (to_name, to_kind) = endpoints.remove(0);

    // Associations can reference endpoints that weren't declared in
    // shorthand/explicit form; infer kind from syntax and register them.
    if let Some(k) = from_kind {
        ensure_inferred(nodes, &from_name, k);
    }
    if let Some(k) = to_kind {
        ensure_inferred(nodes, &to_name, k);
    }

    let kind = if arrow.contains("..") || arrow.contains(".>") {
        AssocKind::Dashed
    } else {
        AssocKind::Solid
    };

    // Reverse arrows (`<-`, `<--`) swap endpoints.
    let (from, to) = if arrow.starts_with('<') {
        (to_name, from_name)
    } else {
        (from_name, to_name)
    };

    Some(Association {
        from,
        to,
        label,
        kind,
    })
}

fn parse_endpoint(pair: Pair<Rule>) -> (String, Option<NodeKind>) {
    let inner = pair.into_inner().next();
    match inner {
        Some(p) => match p.as_rule() {
            Rule::actor_shorthand => (strip_delims(p.as_str(), ':', ':'), Some(NodeKind::Actor)),
            Rule::usecase_shorthand => {
                (strip_delims(p.as_str(), '(', ')'), Some(NodeKind::UseCase))
            }
            Rule::quoted_string => {
                let raw = p.as_str();
                (raw[1..raw.len() - 1].to_string(), None)
            }
            _ => (p.as_str().trim().to_string(), None),
        },
        None => (String::new(), None),
    }
}

fn strip_delims(s: &str, open: char, close: char) -> String {
    let s = s.trim();
    let s = s.strip_prefix(open).unwrap_or(s);
    let s = s.strip_suffix(close).unwrap_or(s);
    s.trim().to_string()
}

fn ensure_node(nodes: &mut Vec<UseCaseNode>, node: UseCaseNode) {
    if let Some(existing) = nodes.iter_mut().find(|n| n.name == node.name) {
        if existing.label.is_none() {
            existing.label = node.label;
        }
        if existing.stereotype.is_none() {
            existing.stereotype = node.stereotype;
        }
        return;
    }
    nodes.push(node);
}

fn ensure_inferred(nodes: &mut Vec<UseCaseNode>, name: &str, kind: NodeKind) {
    if name.is_empty() || nodes.iter().any(|n| n.name == name) {
        return;
    }
    nodes.push(UseCaseNode {
        name: name.to_string(),
        label: None,
        kind,
        stereotype: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthand_actor_and_usecase() {
        let src = ":User:\n(Login)\n:User: --> (Login)\n";
        let d = parse(src).unwrap();
        assert_eq!(d.nodes.len(), 2);
        assert_eq!(d.nodes[0].kind, NodeKind::Actor);
        assert_eq!(d.nodes[1].kind, NodeKind::UseCase);
        assert_eq!(d.associations.len(), 1);
        assert_eq!(d.associations[0].from, "User");
        assert_eq!(d.associations[0].to, "Login");
    }

    #[test]
    fn explicit_actor_with_alias() {
        let src = "actor \"Site Admin\" as Admin\n(Login)\nAdmin --> (Login)\n";
        let d = parse(src).unwrap();
        let admin = d.nodes.iter().find(|n| n.name == "Admin").unwrap();
        assert_eq!(admin.label.as_deref(), Some("Site Admin"));
    }
}
