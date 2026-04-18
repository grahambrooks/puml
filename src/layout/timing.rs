use std::collections::BTreeSet;

use crate::ast::timing::*;

const LANE_LABEL_W: f64 = 140.0;
const LANE_HEIGHT: f64 = 60.0;
const LANE_GAP: f64 = 10.0;
const TIME_PX_PER_UNIT: f64 = 1.5; // 1 time unit = 1.5 px
const MIN_TIMELINE_W: f64 = 360.0;
const TOP_MARGIN: f64 = 40.0;
const SIDE_MARGIN: f64 = 20.0;
const TITLE_H: f64 = 30.0;

#[derive(Debug, Clone)]
pub struct LaneRow {
    pub display: String,
    pub kind: LaneKind,
    pub y: f64,
    pub segments: Vec<LaneSegment>,
}

#[derive(Debug, Clone)]
pub struct LaneSegment {
    pub x_start: f64,
    pub x_end: f64,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct TimeTick {
    pub x: f64,
    pub label: String,
}

pub struct TimingLayout {
    pub lanes: Vec<LaneRow>,
    pub ticks: Vec<TimeTick>,
    pub title: Option<String>,
    pub total_width: f64,
    pub total_height: f64,
    pub timeline_x0: f64,
    pub timeline_x1: f64,
}

pub fn layout(diagram: &TimingDiagram) -> TimingLayout {
    let title_off = if diagram.title.is_some() {
        TITLE_H
    } else {
        0.0
    };

    // Collect every time point that appears + the origin.
    let mut times: BTreeSet<u64> = BTreeSet::new();
    times.insert(0);
    for e in &diagram.events {
        times.insert(e.time);
    }
    let max_time = *times.iter().next_back().unwrap_or(&0);

    let timeline_width = (max_time as f64 * TIME_PX_PER_UNIT).max(MIN_TIMELINE_W);
    let timeline_x0 = SIDE_MARGIN + LANE_LABEL_W;
    let timeline_x1 = timeline_x0 + timeline_width;

    let x_for = |t: u64| -> f64 {
        if max_time == 0 {
            timeline_x0
        } else {
            timeline_x0 + (t as f64 / max_time as f64) * timeline_width
        }
    };

    // Build per-lane transition lists in chronological order.
    let mut lanes: Vec<LaneRow> = Vec::new();
    let mut y = TOP_MARGIN + title_off;
    for lane in &diagram.lanes {
        let mut events: Vec<&Event> = diagram
            .events
            .iter()
            .filter(|e| e.lane == lane.name)
            .collect();
        events.sort_by_key(|e| e.time);

        let mut segments: Vec<LaneSegment> = Vec::new();
        for (i, event) in events.iter().enumerate() {
            let x_start = x_for(event.time);
            let x_end = events
                .get(i + 1)
                .map(|next| x_for(next.time))
                .unwrap_or(timeline_x1);
            segments.push(LaneSegment {
                x_start,
                x_end,
                state: event.state.clone(),
            });
        }

        lanes.push(LaneRow {
            display: lane.label.clone().unwrap_or_else(|| lane.name.clone()),
            kind: lane.kind.clone(),
            y,
            segments,
        });
        y += LANE_HEIGHT + LANE_GAP;
    }

    let ticks: Vec<TimeTick> = times
        .into_iter()
        .map(|t| TimeTick {
            x: x_for(t),
            label: t.to_string(),
        })
        .collect();

    let total_width = timeline_x1 + SIDE_MARGIN;
    let total_height = y + SIDE_MARGIN;

    TimingLayout {
        lanes,
        ticks,
        title: diagram.title.clone(),
        total_width,
        total_height,
        timeline_x0,
        timeline_x1,
    }
}
