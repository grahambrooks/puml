use svg::node::element::{Group, Line, Rectangle, Text};
use svg::Document;

use super::primitives::{style_block, text_node};
use super::theme::Theme;
use crate::ast::timing::LaneKind;
use crate::layout::timing::{LaneRow, TimingLayout};

const FONT_SIZE: f64 = 13.0;
const TOP_MARGIN: f64 = 20.0;
const TITLE_HEIGHT: f64 = 30.0;

pub fn render(layout: &TimingLayout, theme: &Theme) -> Document {
    let title_offset = if layout.title.is_some() {
        TITLE_HEIGHT
    } else {
        0.0
    };
    let height = layout.total_height + title_offset;

    let mut doc = Document::new()
        .set("xmlns", "http://www.w3.org/2000/svg")
        .set("width", layout.total_width)
        .set("height", height)
        .set("viewBox", format!("0 0 {} {}", layout.total_width, height));

    doc = doc.add(style_block());

    let bg = Rectangle::new()
        .set("width", "100%")
        .set("height", "100%")
        .set("fill", theme.background_color.as_str());
    doc = doc.add(bg);

    if let Some(ref t) = layout.title {
        let title_el = Text::new()
            .set("x", layout.total_width / 2.0)
            .set("y", TOP_MARGIN)
            .set("text-anchor", "middle")
            .set("class", "title")
            .add(text_node(t.clone()));
        doc = doc.add(title_el);
    }

    for lane in &layout.lanes {
        doc = doc.add(render_lane(lane, layout));
    }

    // Time axis ticks along the bottom, under the last lane.
    let axis_y = layout.total_height - 10.0;
    for tick in &layout.ticks {
        let line = Line::new()
            .set("x1", tick.x)
            .set("y1", axis_y - 4.0)
            .set("x2", tick.x)
            .set("y2", axis_y + 4.0)
            .set("stroke", "#666")
            .set("stroke-width", "1");
        let label = Text::new()
            .set("x", tick.x)
            .set("y", axis_y + 18.0)
            .set("text-anchor", "middle")
            .set("font-size", "10")
            .add(text_node(tick.label.clone()));
        doc = doc.add(line).add(label);
    }
    // Baseline under the axis ticks.
    let baseline = Line::new()
        .set("x1", layout.timeline_x0)
        .set("y1", axis_y)
        .set("x2", layout.timeline_x1)
        .set("y2", axis_y)
        .set("stroke", "#666")
        .set("stroke-width", "1");
    doc = doc.add(baseline);

    doc
}

fn render_lane(lane: &LaneRow, layout: &TimingLayout) -> Group {
    let y = lane.y;
    let lane_h = 48.0;
    let label_x = layout.timeline_x0 - 8.0;

    // Lane label to the left of the timeline.
    let label = Text::new()
        .set("x", label_x)
        .set("y", y + lane_h / 2.0 + FONT_SIZE / 3.0)
        .set("text-anchor", "end")
        .set("font-size", FONT_SIZE)
        .set("font-weight", "bold")
        .add(text_node(lane.display.clone()));

    // Horizontal guide line for the lane.
    let guide = Line::new()
        .set("x1", layout.timeline_x0)
        .set("y1", y + lane_h / 2.0)
        .set("x2", layout.timeline_x1)
        .set("y2", y + lane_h / 2.0)
        .set("stroke", "#cccccc")
        .set("stroke-width", "1")
        .set("stroke-dasharray", "3,3");

    let mut g = Group::new().add(label).add(guide);

    // Segments as translucent bars with the state label centred inside.
    let (fill, stroke) = match lane.kind {
        LaneKind::Concise => ("#d4e6f1", "#3d6aa0"),
        LaneKind::Clock => ("#eeeeee", "#666666"),
        LaneKind::Binary => ("#fff2cc", "#d6b656"),
        _ => ("#dae8fc", "#6c8ebf"),
    };

    for seg in &lane.segments {
        let seg_w = (seg.x_end - seg.x_start).max(1.0);
        let rect = Rectangle::new()
            .set("x", seg.x_start)
            .set("y", y + 8.0)
            .set("width", seg_w)
            .set("height", lane_h - 16.0)
            .set("fill", fill)
            .set("stroke", stroke)
            .set("stroke-width", "1.2");
        let label = Text::new()
            .set("x", seg.x_start + seg_w / 2.0)
            .set("y", y + lane_h / 2.0 + FONT_SIZE / 3.0)
            .set("text-anchor", "middle")
            .set("font-size", FONT_SIZE - 2.0)
            .add(text_node(seg.state.clone()));
        g = g.add(rect).add(label);
    }

    g
}
