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
fn state_basic_renders() {
    let svg = render_fixture("state_basic.puml");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Idle"));
    assert!(svg.contains("Active"));
}

#[test]
fn state_basic_snapshot() {
    let svg = render_fixture("state_basic.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn state_advanced_renders() {
    let svg = render_fixture("state_advanced.puml");
    assert!(svg.contains("Idle"));
    assert!(svg.contains("Active"));
    // Choice state should be a diamond (polygon) not a rect
    assert!(svg.contains("polygon"));
    // History indicator label should appear as a text node containing just "H"
    assert!(svg.contains(">H<") || svg.contains(">\nH\n<"));
}

#[test]
fn state_advanced_snapshot() {
    let svg = render_fixture("state_advanced.puml");
    insta::assert_snapshot!(svg);
}
