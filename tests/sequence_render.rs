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
