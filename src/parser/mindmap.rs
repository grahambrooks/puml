use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ast::mindmap::*;
use crate::error::PumlError;

#[derive(Parser)]
#[grammar = "parser/grammars/mindmap.pest"]
struct MindMapParser;

pub fn parse(source: &str) -> Result<MindMapDiagram, PumlError> {
    let pairs = MindMapParser::parse(Rule::diagram, source).map_err(|e| {
        let line = match e.line_col {
            pest::error::LineColLocation::Pos((l, _)) => l,
            pest::error::LineColLocation::Span((l, _), _) => l,
        };
        PumlError::Parse {
            line,
            message: e.to_string(),
        }
    })?;

    let mut diagram = MindMapDiagram::default();

    for pair in pairs {
        for stmt in pair.into_inner() {
            match stmt.as_rule() {
                Rule::title_stmt => {
                    diagram.title = Some(stmt.into_inner().as_str().trim().to_string());
                }
                Rule::node_stmt => {
                    if let Some(node) = parse_node(stmt) {
                        diagram.nodes.push(node);
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

fn parse_node(pair: Pair<Rule>) -> Option<MindMapNode> {
    let mut depth = 0usize;
    let mut side = Side::Auto;
    let mut color: Option<String> = None;
    let mut label = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::depth_marker => {
                let raw = inner.as_str();
                depth = raw.chars().count();
                side = match raw.chars().next() {
                    Some('+') => Side::Right,
                    Some('-') => Side::Left,
                    _ => Side::Auto,
                };
            }
            Rule::color_tag => {
                // `[#color]` — strip brackets.
                let raw = inner.as_str();
                color = Some(raw[1..raw.len() - 1].to_string());
            }
            Rule::node_label => {
                label = inner.as_str().trim().to_string();
            }
            _ => {}
        }
    }

    if depth == 0 || label.is_empty() {
        return None;
    }
    Some(MindMapNode {
        label,
        depth,
        side,
        color,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_depth_and_side() {
        let src = "* Root\n** Right child\n-- Left child\n*** Grand\n";
        let d = parse(src).unwrap();
        assert_eq!(d.nodes.len(), 4);
        assert_eq!(d.nodes[0].depth, 1);
        assert_eq!(d.nodes[0].side, Side::Auto);
        assert_eq!(d.nodes[1].depth, 2);
        assert_eq!(d.nodes[2].depth, 2);
        assert_eq!(d.nodes[2].side, Side::Left);
        assert_eq!(d.nodes[3].depth, 3);
        assert_eq!(d.nodes[3].label, "Grand");
    }
}
