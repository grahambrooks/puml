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
