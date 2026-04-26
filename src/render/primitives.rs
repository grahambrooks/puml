use svg::node::element::{Definitions, Marker, Path, Polygon};

pub fn arrowhead_defs() -> Definitions {
    // Filled triangle arrowhead
    let arrow = Polygon::new()
        .set("points", "0 0, 10 5, 0 10")
        .set("fill", "#181818");
    let marker = Marker::new()
        .set("id", "arrowhead")
        .set("markerWidth", "10")
        .set("markerHeight", "10")
        .set("refX", "9")
        .set("refY", "5")
        .set("orient", "auto")
        .add(arrow);

    // Open arrowhead
    let open = Path::new()
        .set("d", "M0,0 L10,5 L0,10")
        .set("fill", "none")
        .set("stroke", "#181818")
        .set("stroke-width", "1.5");
    let open_marker = Marker::new()
        .set("id", "arrowhead-open")
        .set("markerWidth", "10")
        .set("markerHeight", "10")
        .set("refX", "9")
        .set("refY", "5")
        .set("orient", "auto")
        .add(open);

    Definitions::new().add(marker).add(open_marker)
}

/// Escape characters that must not appear literally in SVG text nodes.
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Build a text-content node with SVG-safe escaping applied.
pub fn text_node(s: impl AsRef<str>) -> svg::node::Text {
    svg::node::Text::new(escape_text(s.as_ref()))
}

/// Place an edge label perpendicular to the polyline at its midpoint.
///
/// Returns `(x, y, text_anchor)` for an SVG text element. The label is
/// offset along the right-hand perpendicular of the direction of travel,
/// so labels on counter-flowing edges between the same pair of nodes
/// land on opposite sides naturally — eliminating the standard
/// bidirectional-edge collision.
pub fn label_perpendicular(points: &[(f64, f64)], gap: f64) -> (f64, f64, &'static str) {
    if points.len() < 2 {
        let (x, y) = points.first().copied().unwrap_or((0.0, 0.0));
        return (x + gap, y, "start");
    }
    let seg_lens: Vec<f64> = points
        .windows(2)
        .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
        .collect();
    let total: f64 = seg_lens.iter().sum();
    let half = total / 2.0;
    let mut travelled = 0.0;
    for (i, &len) in seg_lens.iter().enumerate() {
        if travelled + len < half {
            travelled += len;
            continue;
        }
        let t = if len > 0.0 {
            (half - travelled) / len
        } else {
            0.0
        };
        let (x0, y0) = points[i];
        let (x1, y1) = points[i + 1];
        let dx = x1 - x0;
        let dy = y1 - y0;
        let mag = (dx * dx + dy * dy).sqrt().max(0.0001);
        let (ux, uy) = (dx / mag, dy / mag);
        let (px, py) = (uy, -ux);
        let mx = x0 + dx * t;
        let my = y0 + dy * t;
        let lx = mx + px * gap;
        // Bias text downward when the perpendicular goes mostly up so it
        // doesn't visually sit on top of the edge stroke. The 4px nudge for
        // near-horizontal labels keeps the baseline below the line.
        let dy_baseline = if py < -0.3 {
            0.0
        } else if py > 0.3 {
            11.0
        } else {
            4.0
        };
        let ly = my + py * gap + dy_baseline;
        let anchor = if px > 0.3 {
            "start"
        } else if px < -0.3 {
            "end"
        } else {
            "middle"
        };
        return (lx, ly, anchor);
    }
    let last = *points.last().unwrap();
    (last.0 + gap, last.1, "start")
}

/// Full-canvas background rectangle tuned for the theme.
///
/// In `Light` / `Dark` we set the colour inline so the SVG renders correctly
/// even in viewers that ignore `<style>` blocks (GitHub raw view, `open`
/// on macOS, etc.). In `Auto` we additionally tag the rect with `class="bg"`
/// so the media-query rule in the style block can override the inline fill
/// when the viewer prefers dark.
pub fn background_rect(theme: &crate::render::theme::Theme) -> svg::node::element::Rectangle {
    use crate::render::theme::ColorScheme;
    let mut r = svg::node::element::Rectangle::new()
        .set("width", "100%")
        .set("height", "100%")
        .set("fill", theme.background_color.as_str());
    if matches!(theme.color_scheme, ColorScheme::Auto) {
        r = r.set("class", "bg");
    }
    r
}

/// Emit the `<style>` block with palette tuned for `theme`.
///
/// * `Light` and `Dark` bake a single palette directly into the CSS.
/// * `Auto` defines a base light palette, then overrides the colour-carrying
///   properties inside a `@media (prefers-color-scheme: dark)` rule so the
///   SVG flips to dark automatically when embedded in a page (GitHub,
///   mkdocs, a browser, …) that exposes the user's preference.
///
/// Only text colour, divider colour, and the root background switch in Auto
/// mode. Shape fills (class blue, note yellow, choice amber) stay as-is —
/// they're already picked to read on both light and dark canvases.
pub fn style_block(theme: &crate::render::theme::Theme) -> svg::node::element::Style {
    use crate::render::theme::ColorScheme;

    let fg = theme.font_color.as_str();
    let divider = if matches!(theme.color_scheme, ColorScheme::Dark) {
        "#aaaaaa"
    } else {
        "#888888"
    };
    let arrow_stroke = theme.arrow_color.as_str();
    let font_family = theme.font_family.as_str();
    let font_size = theme.font_size;

    let mut css = String::new();
    css.push_str(&format!(
        "text{{font-family:{family};font-size:{size}px;fill:{fg}}}",
        family = font_family,
        size = font_size,
        fg = fg,
    ));
    css.push_str(r#".participant-box{fill:#dae8fc;stroke:#6c8ebf;stroke-width:1.5}"#);
    css.push_str(r#".lifeline{stroke:#6c8ebf;stroke-width:1;stroke-dasharray:6,4}"#);
    css.push_str(r#".activation{fill:#cce0ff;stroke:#6c8ebf;stroke-width:1}"#);
    css.push_str(&format!(
        ".arrow{{stroke:{a};stroke-width:1.5;fill:none}}",
        a = arrow_stroke
    ));
    css.push_str(&format!(
        ".arrow-dashed{{stroke:{a};stroke-width:1.5;fill:none;stroke-dasharray:6,3}}",
        a = arrow_stroke
    ));
    css.push_str(r#".note-box{fill:#ffffc0;stroke:#bbbb00;stroke-width:1}"#);
    css.push_str(&format!(
        ".divider-line{{stroke:{d};stroke-width:1.5;stroke-dasharray:8,3}}",
        d = divider
    ));
    css.push_str(r#".divider-label{font-weight:bold}"#);
    css.push_str(r#".title{font-size:15px;font-weight:bold}"#);
    css.push_str(&format!(
        ".class-line{{stroke:{a};stroke-width:1.5;fill:none}}",
        a = arrow_stroke
    ));
    css.push_str(&format!(
        ".class-line-dashed{{stroke:{a};stroke-width:1.5;fill:none;stroke-dasharray:6,3}}",
        a = arrow_stroke
    ));
    // `.bg` gives renderers a theme-driven handle for the root rectangle —
    // used only in Auto mode today, always available.
    css.push_str(&format!(".bg{{fill:{bg}}}", bg = theme.background_color));

    if matches!(theme.color_scheme, ColorScheme::Auto) {
        css.push_str(&format!(
            "@media (prefers-color-scheme:dark){{text{{fill:{fg}}}.bg{{fill:{bg}}}.arrow,.class-line{{stroke:{a}}}.arrow-dashed,.class-line-dashed{{stroke:{a}}}.divider-line{{stroke:#aaaaaa}}}}",
            fg = theme.dark_font_color,
            bg = theme.dark_background_color,
            a = theme.dark_arrow_color,
        ));
    }

    svg::node::element::Style::new(css)
}
