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

#[test]
fn skinparam_block_form_parses() {
    // `skinparam <element> { ... }` is standard PlantUML. The block's closing
    // brace may be indented or in column 0; both used to fail with
    // "expected arrow" on the block's first line.
    for close in ["}", " }", "   }"] {
        let src = format!(
            "@startuml\nskinparam sequence {{\n   ArrowColor blue\n  ParticipantBorderColor #BAE4B3\n{close}\nAlice -> Bob: hello\n@enduml\n"
        );
        let diagrams = puml::parser::preprocessor::preprocess(&src, None);
        let ast = puml::parser::parse(&diagrams[0])
            .unwrap_or_else(|e| panic!("closing brace {close:?}: {e}"));
        let svg = puml::render::render(&ast).to_string();
        assert!(
            svg.contains("Alice") && svg.contains("hello"),
            "closing brace {close:?}"
        );
    }
}

#[test]
fn autonumber_with_step_and_format_parses() {
    let parse = |line: &str| {
        let src = format!("@startuml\n{line}\nAlice -> Bob: hello\n@enduml\n");
        let diagrams = puml::parser::preprocessor::preprocess(&src, None);
        puml::parser::parse(&diagrams[0])
    };
    for ok in [
        "autonumber",
        "autonumber 5",
        "autonumber 1 1",
        "autonumber 1 1 \"<b>[0]\"",
    ] {
        parse(ok).unwrap_or_else(|e| panic!("{ok:?}: {e}"));
    }
    // A step other than 1 would number messages wrongly if ignored.
    assert!(parse("autonumber 10 5").is_err());
}

fn parse_seq(src: &str) -> Result<puml::ast::sequence::SequenceDiagram, puml::error::PumlError> {
    let diagrams = puml::parser::preprocessor::preprocess(src, None);
    puml::parser::parse(&diagrams[0]).map(|ast| match ast {
        puml::ast::DiagramAst::Sequence(d) => d,
        other => panic!("not a sequence diagram: {other:?}"),
    })
}

#[test]
fn box_groups_participants_created_inside_it() {
    let d = parse_seq(
        "@startuml\nparticipant Client\nbox \"Backend\" #LightBlue\n  participant \"Request Handler\" as handler\n  participant Store\n  handler -> Cache: lookup\nend box\nClient -> handler: go\nClient -> Late: after\n@enduml\n",
    )
    .unwrap();
    assert_eq!(d.boxes.len(), 1);
    let b = &d.boxes[0];
    assert_eq!(b.title.as_deref(), Some("Backend"));
    assert_eq!(b.color.as_deref(), Some("#LightBlue"));
    // Declared and implicitly created participants join; earlier and later ones don't.
    assert_eq!(b.participants, ["handler", "Store", "Cache"]);

    let d = parse_seq("@startuml\nbox\nparticipant A\nend box\nA -> B\n@enduml\n").unwrap();
    assert_eq!(d.boxes[0].title, None);
    assert_eq!(d.boxes[0].color, None);
    assert_eq!(d.boxes[0].participants, ["A"]);
}

#[test]
fn box_errors_are_parse_errors() {
    for src in [
        // never closed
        "@startuml\nbox \"A\"\nparticipant A\n@enduml\n",
        // stray end box
        "@startuml\nparticipant A\nend box\n@enduml\n",
        // nested
        "@startuml\nbox \"A\"\nbox \"B\"\nparticipant A\nend box\nend box\n@enduml\n",
        // inside a fragment
        "@startuml\nA -> B\nalt x\nbox \"A\"\nparticipant C\nend box\nend\n@enduml\n",
        // `end box` must not close a fragment
        "@startuml\nA -> B\nalt x\nA -> B\nend box\n@enduml\n",
        // unquoted title and gradients are not supported
        "@startuml\nbox Backend\nparticipant A\nend box\n@enduml\n",
        "@startuml\nbox \"A\" #red-blue\nparticipant A\nend box\n@enduml\n",
    ] {
        assert!(parse_seq(src).is_err(), "should fail: {src}");
    }
    // A participant whose name starts with `box` is still a participant.
    parse_seq("@startuml\nboxer -> A: hi\n@enduml\n").unwrap();
}

#[test]
fn box_renders_behind_its_members_with_title() {
    let svg = render_fixture("sequence_box_event_sourcing.puml");
    // From hugo-blog-migration _pages/event-driven-architecture/patterns/event-sourcing.adoc,
    // with plantuml.skin inlined.
    let box_at = svg.find("class=\"participant-group\"").expect("box drawn");
    assert!(svg.contains("Event Sourced Application"));
    let first_participant = svg
        .find(">\nRequest Handler\n<")
        .expect("participant drawn");
    assert!(box_at < first_participant, "box must be drawn first");
    assert!(svg.contains("customer-email-event"));
}

#[test]
fn boxes_with_implicit_members_render() {
    // From notification-event-pattern.adoc: members are created by messages
    // inside each box rather than declared.
    let svg = render_fixture("sequence_box_regions.puml");
    assert_eq!(svg.matches("class=\"participant-group\"").count(), 2);
    for t in ["Region AB", "Region CDE", "Subscriber E", "CDE Cache"] {
        assert!(svg.contains(t), "{t}");
    }
}

#[test]
fn box_snapshot() {
    let svg = render_fixture("sequence_box_regions.puml");
    insta::assert_snapshot!(svg);
}
