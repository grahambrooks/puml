use crate::ast::usecase::*;

const ACTOR_WIDTH: f64 = 60.0;
const ACTOR_HEIGHT: f64 = 80.0;
const USECASE_MIN_W: f64 = 120.0;
const USECASE_H: f64 = 50.0;
const H_GAP: f64 = 80.0;
const V_GAP: f64 = 30.0;
const SIDE_MARGIN: f64 = 40.0;
const TOP_MARGIN: f64 = 30.0;
const TITLE_H: f64 = 30.0;
const CHAR_W: f64 = 7.5;

#[derive(Debug, Clone)]
pub struct UseCaseLayoutNode {
    pub name: String,
    pub display: String,
    pub kind: NodeKind,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub stereotype: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UseCaseLayoutEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub dashed: bool,
}

pub struct UseCaseLayout {
    pub nodes: Vec<UseCaseLayoutNode>,
    pub edges: Vec<UseCaseLayoutEdge>,
    pub title: Option<String>,
    pub total_width: f64,
    pub total_height: f64,
}

/// Minimum-viable layout: actors stack in a left column, use cases stack in a
/// centre column. Use-case columns could be expanded per rank later.
pub fn layout(diagram: &UseCaseDiagram) -> UseCaseLayout {
    let title_off = if diagram.title.is_some() {
        TITLE_H
    } else {
        0.0
    };

    let actors: Vec<&UseCaseNode> = diagram
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Actor)
        .collect();
    let cases: Vec<&UseCaseNode> = diagram
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::UseCase)
        .collect();

    let case_col_w: f64 = cases
        .iter()
        .map(|c| usecase_width(c))
        .fold(USECASE_MIN_W, f64::max);

    let left_col_x = SIDE_MARGIN + ACTOR_WIDTH / 2.0;
    let case_col_x = left_col_x + ACTOR_WIDTH / 2.0 + H_GAP + case_col_w / 2.0;

    let mut out_nodes: Vec<UseCaseLayoutNode> = Vec::new();
    let mut y_actors = TOP_MARGIN + title_off;
    for actor in &actors {
        out_nodes.push(UseCaseLayoutNode {
            name: actor.name.clone(),
            display: actor.label.clone().unwrap_or_else(|| actor.name.clone()),
            kind: NodeKind::Actor,
            x: left_col_x - ACTOR_WIDTH / 2.0,
            y: y_actors,
            w: ACTOR_WIDTH,
            h: ACTOR_HEIGHT,
            stereotype: actor.stereotype.clone(),
        });
        y_actors += ACTOR_HEIGHT + V_GAP;
    }

    let mut y_cases = TOP_MARGIN + title_off;
    for case in &cases {
        let w = usecase_width(case);
        out_nodes.push(UseCaseLayoutNode {
            name: case.name.clone(),
            display: case.label.clone().unwrap_or_else(|| case.name.clone()),
            kind: NodeKind::UseCase,
            x: case_col_x - w / 2.0,
            y: y_cases,
            w,
            h: USECASE_H,
            stereotype: case.stereotype.clone(),
        });
        y_cases += USECASE_H + V_GAP;
    }

    let edges: Vec<UseCaseLayoutEdge> = diagram
        .associations
        .iter()
        .map(|a| UseCaseLayoutEdge {
            from: a.from.clone(),
            to: a.to.clone(),
            label: a.label.clone(),
            dashed: matches!(a.kind, AssocKind::Dashed),
        })
        .collect();

    let total_width = case_col_x + case_col_w / 2.0 + SIDE_MARGIN;
    let total_height = y_actors.max(y_cases) + SIDE_MARGIN;

    UseCaseLayout {
        nodes: out_nodes,
        edges,
        title: diagram.title.clone(),
        total_width,
        total_height,
    }
}

fn usecase_width(case: &UseCaseNode) -> f64 {
    let text = case.label.as_deref().unwrap_or(&case.name);
    (text.len() as f64 * CHAR_W + 30.0).max(USECASE_MIN_W)
}
