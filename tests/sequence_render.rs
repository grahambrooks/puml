use std::path::Path;

fn render_fixture(name: &str) -> String {
    let path = Path::new("tests/fixtures").join(name);
    let source = std::fs::read_to_string(&path).expect("read fixture");
    let base = path.parent();
    let diagrams = puml::parser::preprocessor::preprocess(&source, base);
    assert!(!diagrams.is_empty(), "no diagrams in {name}");
    let ast = puml::parser::parse(&diagrams[0]).expect("parse");
    let doc = puml::render::render(&ast);
    doc.to_string()
}

#[test]
fn basic_renders() {
    let svg = render_fixture("basic.puml");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Alice"));
    assert!(svg.contains("Bob"));
}

#[test]
fn participants_renders() {
    let svg = render_fixture("participants.puml");
    assert!(svg.contains("User"));
    assert!(svg.contains("PostgreSQL"));
}

#[test]
fn notes_renders() {
    let svg = render_fixture("notes.puml");
    assert!(svg.contains("multiline note") || svg.contains("note-box"));
}

#[test]
fn basic_snapshot() {
    let svg = render_fixture("basic.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn participants_snapshot() {
    let svg = render_fixture("participants.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn groups_renders() {
    let svg = render_fixture("sequence_groups.puml");
    assert!(svg.contains("alt"));
    assert!(svg.contains("loop"));
    // autonumber prefix should appear on at least the first message label
    assert!(svg.contains("1:"));
}

#[test]
fn groups_snapshot() {
    let svg = render_fixture("sequence_groups.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn reverse_arrows_render() {
    let svg = render_fixture("sequence_return.puml");
    // All three messages land, regardless of which direction they're written.
    assert!(svg.contains("request"));
    assert!(svg.contains("reverse"));
    assert!(svg.contains("response"));
}

#[test]
fn reverse_arrows_snapshot() {
    let svg = render_fixture("sequence_return.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn self_message_renders() {
    let svg = render_fixture("sequence_self_message.puml");
    assert!(svg.contains("reflect"));
    assert!(svg.contains("recompute"));
}

#[test]
fn self_message_snapshot() {
    let svg = render_fixture("sequence_self_message.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn nested_groups_renders() {
    let svg = render_fixture("sequence_nested_groups.puml");
    // Both outer loop and inner alt groups should render their kind labels.
    assert!(svg.contains("loop"), "missing loop tab: {svg}");
    assert!(svg.contains("alt"), "missing alt tab: {svg}");
    // Inner else-section label should render too.
    assert!(svg.contains("timeout"));
}

#[test]
fn nested_groups_snapshot() {
    let svg = render_fixture("sequence_nested_groups.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn note_over_renders() {
    let svg = render_fixture("sequence_note_over.puml");
    assert!(svg.contains("Shared state flows"));
}

#[test]
fn note_over_snapshot() {
    let svg = render_fixture("sequence_note_over.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn skinparam_changes_background() {
    let svg = render_fixture("sequence_themed.puml");
    // User-supplied background should land in the SVG
    assert!(
        svg.contains("fill=\"#f5f5dc\""),
        "skinparam backgroundColor not applied: {}",
        &svg[..svg.len().min(500)]
    );
    // Default white should no longer appear as the root background
    assert!(
        !svg.contains("fill=\"#ffffff\""),
        "default bg still present"
    );
}

#[test]
fn themed_snapshot() {
    let svg = render_fixture("sequence_themed.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn amiga_preset_applies() {
    let svg = render_fixture("sequence_amiga_theme.puml");
    // amiga preset sets background to deep blue
    assert!(
        svg.contains("fill=\"#000088\""),
        "!theme amiga did not apply: {}",
        &svg[..svg.len().min(500)]
    );
}

#[test]
fn amiga_preset_snapshot() {
    let svg = render_fixture("sequence_amiga_theme.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn dark_preset_applies() {
    let svg = render_fixture("sequence_dark_theme.puml");
    // Dark palette: near-black background and near-white text.
    assert!(
        svg.contains("fill=\"#1e1e1e\""),
        "dark background not applied"
    );
    assert!(
        svg.contains("fill:#e8e8e8"),
        "dark font colour not applied in style block"
    );
    // No `prefers-color-scheme` emitted — dark is a static palette, not auto.
    assert!(
        !svg.contains("prefers-color-scheme"),
        "dark should bake the palette, not emit a media query"
    );
}

#[test]
fn dark_preset_snapshot() {
    let svg = render_fixture("sequence_dark_theme.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn auto_preset_emits_media_query() {
    let svg = render_fixture("sequence_auto_theme.puml");
    // Light defaults still land (background, text colour) so the SVG is
    // readable in viewers that ignore media queries.
    assert!(
        svg.contains("fill=\"#ffffff\""),
        "auto mode should keep the light background as fallback"
    );
    // The root rectangle carries `class="bg"` so the media query can override.
    assert!(
        svg.contains("class=\"bg\""),
        "auto mode should tag the background rect with class=\"bg\""
    );
    // The media query itself must be present.
    assert!(
        svg.contains("@media (prefers-color-scheme:dark)"),
        "auto mode should emit a prefers-color-scheme CSS rule"
    );
    // And the dark fallback colour must appear inside that rule.
    assert!(
        svg.contains("#1e1e1e"),
        "auto mode should reference a dark background colour"
    );
}

#[test]
fn auto_preset_snapshot() {
    let svg = render_fixture("sequence_auto_theme.puml");
    insta::assert_snapshot!(svg);
}
