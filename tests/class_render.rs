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
