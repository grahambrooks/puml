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

pub fn style_block() -> svg::node::element::Style {
    svg::node::element::Style::new(concat!(
        r#"text{font-family:"Liberation Sans",Helvetica,Arial,sans-serif;font-size:13px;fill:#181818}"#,
        r#".participant-box{fill:#dae8fc;stroke:#6c8ebf;stroke-width:1.5}"#,
        r#".lifeline{stroke:#6c8ebf;stroke-width:1;stroke-dasharray:6,4}"#,
        r#".activation{fill:#cce0ff;stroke:#6c8ebf;stroke-width:1}"#,
        r#".arrow{stroke:#181818;stroke-width:1.5;fill:none}"#,
        r#".arrow-dashed{stroke:#181818;stroke-width:1.5;fill:none;stroke-dasharray:6,3}"#,
        r#".note-box{fill:#ffffc0;stroke:#bbbb00;stroke-width:1}"#,
        r#".divider-line{stroke:#888;stroke-width:1.5;stroke-dasharray:8,3}"#,
        r#".divider-label{font-weight:bold}"#,
        r#".title{font-size:15px;font-weight:bold}"#,
        r#".class-line{stroke:#181818;stroke-width:1.5;fill:none}"#,
        r#".class-line-dashed{stroke:#181818;stroke-width:1.5;fill:none;stroke-dasharray:6,3}"#,
    ))
}
