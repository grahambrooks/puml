use svg::node::element::{Line, Polygon, Rectangle, Text};
use svg::Document;

use super::primitives::{background_rect, style_block, text_node};
use super::theme::Theme;
use crate::layout::gantt::GanttLayout;

const FONT_SIZE: f64 = 13.0;
const TOP_MARGIN: f64 = 20.0;
const TITLE_HEIGHT: f64 = 30.0;

pub fn render(layout: &GanttLayout, theme: &Theme) -> Document {
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

    doc = doc.add(style_block(theme));

    doc = doc.add(background_rect(theme));

    if let Some(ref t) = layout.title {
        let title_el = Text::new()
            .set("x", layout.total_width / 2.0)
            .set("y", TOP_MARGIN)
            .set("text-anchor", "middle")
            .set("class", "title")
            .add(text_node(t.clone()));
        doc = doc.add(title_el);
    }

    // Day axis at the top of the chart area.
    for tick in &layout.axis_ticks {
        let line = Line::new()
            .set("x1", tick.x)
            .set("y1", layout.axis_y - 4.0)
            .set("x2", tick.x)
            .set("y2", layout.axis_y + 4.0)
            .set("stroke", "#666")
            .set("stroke-width", "1");
        let label = Text::new()
            .set("x", tick.x)
            .set("y", layout.axis_y - 8.0)
            .set("text-anchor", "middle")
            .set("font-size", "10")
            .add(text_node(tick.label.clone()));
        doc = doc.add(line).add(label);
    }
    if let (Some(first), Some(last)) = (layout.axis_ticks.first(), layout.axis_ticks.last()) {
        let axis = Line::new()
            .set("x1", first.x)
            .set("y1", layout.axis_y)
            .set("x2", last.x)
            .set("y2", layout.axis_y)
            .set("stroke", "#666")
            .set("stroke-width", "1");
        doc = doc.add(axis);
    }

    // Task rows.
    for bar in &layout.bars {
        let label = Text::new()
            .set("x", layout.label_col_x)
            .set("y", bar.y + bar.h / 2.0 + FONT_SIZE / 3.0)
            .set("font-size", FONT_SIZE)
            .add(text_node(bar.name.clone()));
        doc = doc.add(label);

        if bar.milestone {
            let cx = bar.x;
            let cy = bar.y + bar.h / 2.0;
            let r = bar.h / 2.0 - 4.0;
            let diamond = Polygon::new()
                .set(
                    "points",
                    format!(
                        "{},{} {},{} {},{} {},{}",
                        cx,
                        cy - r,
                        cx + r,
                        cy,
                        cx,
                        cy + r,
                        cx - r,
                        cy
                    ),
                )
                .set("fill", "#b86e11")
                .set("stroke", "#7a4a0b")
                .set("stroke-width", "1.2");
            doc = doc.add(diamond);
        } else {
            let rect = Rectangle::new()
                .set("x", bar.x)
                .set("y", bar.y + 4.0)
                .set("width", bar.w)
                .set("height", bar.h - 8.0)
                .set("rx", "3")
                .set("fill", "#6c8ebf")
                .set("stroke", "#3d6aa0")
                .set("stroke-width", "1");
            doc = doc.add(rect);
        }
    }

    doc
}
