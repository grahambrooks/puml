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
fn class_basic_renders() {
    let svg = render_fixture("class_basic.puml");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Animal"));
    assert!(svg.contains("Dog"));
    assert!(svg.contains("Cat"));
}

#[test]
fn class_relations_renders() {
    let svg = render_fixture("class_relations.puml");
    assert!(svg.contains("Company"));
    assert!(svg.contains("Employee"));
    assert!(svg.contains("Role"));
}

#[test]
fn class_basic_snapshot() {
    let svg = render_fixture("class_basic.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn class_relations_snapshot() {
    let svg = render_fixture("class_relations.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn generics_and_notes_render() {
    let svg = render_fixture("class_generics_notes.puml");
    // Generics should be in the class header (SVG may entity-escape `<` and `>`)
    assert!(svg.contains("Container&lt;T&gt;") || svg.contains("Container<T>"));
    assert!(svg.contains("Iterable&lt;T&gt;") || svg.contains("Iterable<T>"));
    // Stereotype rendering
    assert!(svg.contains("data"));
    // Note text
    assert!(svg.contains("Generic container"));
}

#[test]
fn generics_and_notes_snapshot() {
    let svg = render_fixture("class_generics_notes.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn inheritance_chain_renders() {
    let svg = render_fixture("class_inheritance_chain.puml");
    assert!(svg.contains("Shape"));
    assert!(svg.contains("Circle"));
    assert!(svg.contains("Square"));
    assert!(svg.contains("Printable"));
}

#[test]
fn inheritance_chain_snapshot() {
    let svg = render_fixture("class_inheritance_chain.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn multirank_renders() {
    // A 3-deep inheritance chain (A → B → C → D) plus a long association
    // A--D that spans every layer. Verifies that the long edge threads
    // through virtual nodes rather than cutting straight across the diagram.
    let svg = render_fixture("class_multirank.puml");
    assert!(svg.contains("\nA\n"));
    assert!(svg.contains("\nD\n"));
    assert!(svg.contains("\nE\n"));
    // The long A--D edge crosses three layers; with virtual nodes the
    // routing produces a polyline with ≥5 segments. Without them it would
    // be a 3-segment Z.
    let class_lines: Vec<&str> = svg
        .lines()
        .filter(|l| l.contains("class=\"class-line\""))
        .collect();
    let max_bends = class_lines
        .iter()
        .map(|l| l.matches(" L").count())
        .max()
        .unwrap_or(0);
    assert!(
        max_bends >= 5,
        "expected stair-step routing to produce ≥5 L segments on the long edge; max found {max_bends}"
    );
}

#[test]
fn multirank_snapshot() {
    let svg = render_fixture("class_multirank.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn associations_renders() {
    let svg = render_fixture("class_associations.puml");
    assert!(svg.contains("Order"));
    assert!(svg.contains("LineItem"));
    assert!(svg.contains("Customer"));
}

#[test]
fn associations_snapshot() {
    let svg = render_fixture("class_associations.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn object_renders() {
    let svg = render_fixture("object_basic.puml");
    // Both object instances appear, and names are underlined (UML convention).
    assert!(svg.contains("alice"));
    assert!(svg.contains("bob"));
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "object name should be underlined"
    );
}

#[test]
fn object_snapshot() {
    let svg = render_fixture("object_basic.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn component_renders() {
    let svg = render_fixture("component_basic.puml");
    assert!(svg.contains("WebServer"));
    assert!(svg.contains("AuthService"));
    assert!(svg.contains("Database"));
    assert!(
        svg.contains("«component»"),
        "component should carry «component» stereotype"
    );
}

#[test]
fn component_snapshot() {
    let svg = render_fixture("component_basic.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn deployment_renders() {
    let svg = render_fixture("deployment_basic.puml");
    // All four deployment container kinds land with their stereotype labels.
    assert!(svg.contains("«node»"), "missing «node»: {svg}");
    assert!(svg.contains("«cloud»"), "missing «cloud»");
    assert!(svg.contains("«database»"), "missing «database»");
    assert!(svg.contains("«folder»"), "missing «folder»");
    assert!(svg.contains("«queue»"), "missing «queue»");
    assert!(svg.contains("Web Server"));
    assert!(svg.contains("UserDB"));
}

#[test]
fn deployment_snapshot() {
    let svg = render_fixture("deployment_basic.puml");
    insta::assert_snapshot!(svg);
}

fn parse_class(src: &str) -> puml::ast::class::ClassDiagram {
    let diagrams = puml::parser::preprocessor::preprocess(src, None);
    match puml::parser::parse(&diagrams[0]).unwrap_or_else(|e| panic!("{src}: {e}")) {
        puml::ast::DiagramAst::Class(d) => d,
        other => panic!("not a class diagram: {other:?}"),
    }
}

#[test]
fn relation_label_after_name_with_or_without_space() {
    // PlantUML allows the label colon to touch the target name. The name
    // used to swallow the colon ("expected card_label or relation_arrow").
    for (line, from, to, label, from_card, to_card) in [
        ("A -- B : uses", "A", "B", "uses", None, None),
        ("A -- B: uses", "A", "B", "uses", None, None),
        ("A --> B:uses", "A", "B", "uses", None, None),
        // `*--` puts the diamond on A, so the AST stores B -> A.
        (
            "A \"1\" *-- \"many\" B : contains",
            "B",
            "A",
            "contains",
            Some("1"),
            Some("many"),
        ),
        (
            "ns::A ..> ns::B: depends",
            "ns::A",
            "ns::B",
            "depends",
            None,
            None,
        ),
    ] {
        let d = parse_class(&format!("@startuml\nclass A\nclass B\n{line}\n@enduml\n"));
        let r = &d.relations[0];
        assert_eq!(
            (r.from.as_str(), r.to.as_str()),
            (from, to),
            "{line}: endpoints"
        );
        assert_eq!(r.label.as_deref(), Some(label), "{line}: label");
        assert_eq!(r.from_label.as_deref(), from_card, "{line}");
        assert_eq!(r.to_label.as_deref(), to_card, "{line}");
    }
}

#[test]
fn inline_note_colon_may_touch_target() {
    let d = parse_class("@startuml\nclass A\nnote right of A: hello\n@enduml\n");
    assert_eq!(d.notes[0].target, "A");
}

#[test]
fn compensating_command_diagram_renders_label() {
    // From hugo-blog-migration _pages/event-driven-architecture/compensating-actions.adoc.
    let svg = render_fixture("class_compensating_command.puml");
    assert!(svg.contains("Entity") && svg.contains("Event"));
    assert!(svg.contains("\naccepts\n"), "label missing: {svg}");
}
