use std::collections::HashMap;

use crate::ast::gantt::*;

const LABEL_W: f64 = 160.0;
const ROW_H: f64 = 30.0;
const ROW_GAP: f64 = 6.0;
const DAY_W: f64 = 16.0;
const TOP_MARGIN: f64 = 60.0; // extra room for the day-axis labels
const SIDE_MARGIN: f64 = 20.0;
const TITLE_H: f64 = 30.0;

#[derive(Debug, Clone)]
pub struct GanttBar {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub milestone: bool,
}

#[derive(Debug, Clone)]
pub struct GanttAxisTick {
    pub x: f64,
    pub label: String,
}

pub struct GanttLayout {
    pub bars: Vec<GanttBar>,
    pub axis_ticks: Vec<GanttAxisTick>,
    pub label_col_x: f64,
    pub title: Option<String>,
    pub total_width: f64,
    pub total_height: f64,
    pub axis_y: f64,
}

pub fn layout(diagram: &GanttDiagram) -> GanttLayout {
    let title_off = if diagram.title.is_some() {
        TITLE_H
    } else {
        0.0
    };

    // Resolve start offsets: honour fixed_start > depends_on chains > default 0.
    // One or two passes over the list should resolve typical fixture graphs;
    // cap at 8 to stay safe against cycles (which we ignore here).
    let mut starts: HashMap<String, u32> = HashMap::new();
    for task in &diagram.tasks {
        if let Some(s) = task.fixed_start {
            starts.insert(task.name.clone(), s);
        } else if task.depends_on.is_none() {
            starts.insert(task.name.clone(), 0);
        }
    }
    for _ in 0..8 {
        let mut made_progress = false;
        for task in &diagram.tasks {
            if starts.contains_key(&task.name) {
                continue;
            }
            if let Some(ref dep) = task.depends_on {
                if let Some(dep_task) = diagram.tasks.iter().find(|t| &t.name == dep) {
                    if let Some(&dep_start) = starts.get(&dep_task.name) {
                        let dep_end = dep_start + dep_task.duration;
                        starts.insert(task.name.clone(), dep_end);
                        made_progress = true;
                    }
                }
            }
        }
        if !made_progress {
            break;
        }
    }
    // Anything still unresolved (cycles / dangling refs) starts at 0.
    for task in &diagram.tasks {
        starts.entry(task.name.clone()).or_insert(0);
    }

    // Max end for canvas sizing.
    let max_end = diagram
        .tasks
        .iter()
        .map(|t| starts.get(&t.name).copied().unwrap_or(0) + t.duration)
        .max()
        .unwrap_or(1)
        .max(1);

    let chart_x0 = SIDE_MARGIN + LABEL_W;
    let chart_width = max_end as f64 * DAY_W;
    let chart_x1 = chart_x0 + chart_width;

    let mut bars: Vec<GanttBar> = Vec::new();
    let mut y = TOP_MARGIN + title_off;
    for task in &diagram.tasks {
        let start = starts.get(&task.name).copied().unwrap_or(0);
        let x = chart_x0 + start as f64 * DAY_W;
        let w = if task.milestone {
            ROW_H // drawn as a diamond centred on this square
        } else {
            (task.duration as f64 * DAY_W).max(4.0)
        };
        bars.push(GanttBar {
            name: task.name.clone(),
            x,
            y,
            w,
            h: ROW_H,
            milestone: task.milestone,
        });
        y += ROW_H + ROW_GAP;
    }

    // Axis: one tick every 5 days, plus a final tick at max_end.
    let mut axis_ticks: Vec<GanttAxisTick> = Vec::new();
    let step = if max_end <= 10 {
        1
    } else if max_end <= 30 {
        5
    } else {
        10
    };
    let mut d = 0u32;
    while d <= max_end {
        axis_ticks.push(GanttAxisTick {
            x: chart_x0 + d as f64 * DAY_W,
            label: format!("{}", d),
        });
        d += step;
    }

    let axis_y = TOP_MARGIN + title_off - 30.0;
    let total_width = chart_x1 + SIDE_MARGIN;
    let total_height = y + SIDE_MARGIN;

    GanttLayout {
        bars,
        axis_ticks,
        label_col_x: SIDE_MARGIN + 8.0,
        title: diagram.title.clone(),
        total_width,
        total_height,
        axis_y,
    }
}
