use std::collections::HashMap;

use super::sugiyama::{assign_x_balanced, reorder_barycentric};
use crate::ast::state::*;

const NODE_W: f64 = 120.0;
const NODE_H: f64 = 36.0;
const CIRCLE_R: f64 = 12.0;
const H_GAP: f64 = 60.0;
const V_GAP: f64 = 50.0;
const SIDE_MARGIN: f64 = 40.0;
const TOP_MARGIN: f64 = 30.0;
const TITLE_H: f64 = 30.0;

#[derive(Debug, Clone)]
pub struct StateLayoutNode {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub kind: StateKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StateLayoutEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

pub struct StateLayout {
    pub nodes: Vec<StateLayoutNode>,
    pub edges: Vec<StateLayoutEdge>,
    pub total_width: f64,
    pub total_height: f64,
    pub title: Option<String>,
}

pub fn layout(diagram: &StateDiagram) -> StateLayout {
    let title_off = if diagram.title.is_some() {
        TITLE_H
    } else {
        0.0
    };

    // Build a rank map using transitions: [*] starts at rank 0, each reachable
    // state increments the rank.
    let mut ranks: HashMap<&str, usize> = HashMap::new();
    let mut queue = std::collections::VecDeque::new();

    // Find all [*] sources
    for t in &diagram.transitions {
        if t.from == "[*]" {
            let r = ranks.entry(&t.to).or_insert(0);
            if *r == 0 {
                queue.push_back(t.to.as_str());
            }
        }
    }

    // BFS to assign ranks
    let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();
    while let Some(name) = queue.pop_front() {
        if !visited.insert(name) {
            continue;
        }
        let cur_rank = ranks.get(name).copied().unwrap_or(0);
        for t in &diagram.transitions {
            if t.from == name && t.to != "[*]" {
                let next_rank = ranks.entry(&t.to).or_insert(cur_rank + 1);
                if *next_rank <= cur_rank {
                    *next_rank = cur_rank + 1;
                }
                queue.push_back(&t.to);
            }
        }
    }

    // Assign remaining unvisited states sequentially
    let max_rank = ranks.values().copied().max().unwrap_or(0);
    let mut next_rank = max_rank + 1;
    for s in &diagram.states {
        if s.name != "[*]" && !ranks.contains_key(s.name.as_str()) {
            ranks.insert(&s.name, next_rank);
            next_rank += 1;
        }
    }

    // Group by rank, carrying *indices* so barycentric reorder below can
    // reshuffle within-rank order without borrow-checker grief.
    let final_max_rank = ranks.values().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); final_max_rank + 2];

    for (i, s) in diagram.states.iter().enumerate() {
        if s.name == "[*]" || s.kind == StateKind::Start {
            layers[0].push(i);
        } else {
            let r = ranks.get(s.name.as_str()).copied().unwrap_or(0);
            let layer_idx = (r + 1).min(layers.len() - 1);
            layers[layer_idx].push(i);
        }
    }

    // Reduce edge crossings: use every transition (not just [*]→x) so the
    // within-layer order reflects actual connectivity.
    let name_to_idx: HashMap<&str, usize> = diagram
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i))
        .collect();
    let edge_pairs: Vec<(usize, usize)> = diagram
        .transitions
        .iter()
        .filter_map(|t| {
            let a = name_to_idx.get(t.from.as_str()).copied()?;
            let b = name_to_idx.get(t.to.as_str()).copied()?;
            Some((a, b))
        })
        .collect();
    reorder_barycentric(&mut layers, &edge_pairs, 6);

    // Global widths indexed by state index for the x-assignment helper.
    let mut global_widths: Vec<f64> = vec![0.0; diagram.states.len()];
    for (i, s) in diagram.states.iter().enumerate() {
        global_widths[i] = node_width(s);
    }
    let xs = assign_x_balanced(&layers, &edge_pairs, &global_widths, H_GAP, SIDE_MARGIN);

    let mut name_to_layout: HashMap<String, StateLayoutNode> = HashMap::new();
    let mut y = TOP_MARGIN + title_off;
    let mut total_width: f64 = 0.0;

    for layer in &layers {
        if layer.is_empty() {
            continue;
        }
        let layer_h = layer
            .iter()
            .map(|&i| node_height(&diagram.states[i]))
            .fold(0.0_f64, f64::max);
        for &i in layer {
            let s = &diagram.states[i];
            let w = global_widths[i];
            let h = node_height(s);
            let x = xs.get(&i).copied().unwrap_or(SIDE_MARGIN);
            total_width = total_width.max(x + w + SIDE_MARGIN);
            name_to_layout.insert(
                s.name.clone(),
                StateLayoutNode {
                    name: s.name.clone(),
                    x,
                    y,
                    w,
                    h,
                    kind: s.kind.clone(),
                    label: s.label.clone(),
                },
            );
        }
        y += layer_h + V_GAP;
    }

    // Recentre the whole graph — parent/child alignment is preserved because
    // every node shifts by the same amount.
    let (min_x, max_right) = name_to_layout.values().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min_x, max_right), nl| (min_x.min(nl.x), max_right.max(nl.x + nl.w)),
    );
    if min_x.is_finite() {
        let content_w = max_right - min_x;
        let target_left = (total_width - content_w) / 2.0;
        let shift = target_left - min_x;
        for nl in name_to_layout.values_mut() {
            nl.x += shift;
        }
    }

    let total_height = y + SIDE_MARGIN;

    // Build edges
    let edges: Vec<StateLayoutEdge> = diagram
        .transitions
        .iter()
        .map(|t| StateLayoutEdge {
            from: t.from.clone(),
            to: t.to.clone(),
            label: t.label.clone(),
        })
        .collect();

    // Preserve declaration order
    let nodes: Vec<StateLayoutNode> = diagram
        .states
        .iter()
        .filter_map(|s| name_to_layout.remove(&s.name))
        .collect();

    StateLayout {
        nodes,
        edges,
        total_width,
        total_height,
        title: diagram.title.clone(),
    }
}

fn node_width(s: &StateNode) -> f64 {
    if s.name == "[H]" || s.name == "[H*]" {
        return CIRCLE_R * 2.0;
    }
    match s.kind {
        StateKind::Start => CIRCLE_R * 2.0,
        StateKind::Choice => 32.0,
        StateKind::Fork | StateKind::Join => NODE_W,
        _ if s.name == "[*]" => CIRCLE_R * 2.0,
        _ => {
            let label = s.label.as_deref().unwrap_or(&s.name);
            (label.len() as f64 * 7.5 + 24.0).max(NODE_W)
        }
    }
}

fn node_height(s: &StateNode) -> f64 {
    if s.name == "[H]" || s.name == "[H*]" {
        return CIRCLE_R * 2.0;
    }
    match s.kind {
        StateKind::Start => CIRCLE_R * 2.0,
        StateKind::Choice => 32.0,
        StateKind::Fork | StateKind::Join => 10.0,
        _ if s.name == "[*]" => CIRCLE_R * 2.0,
        _ => NODE_H,
    }
}
