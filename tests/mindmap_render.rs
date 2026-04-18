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
fn mindmap_basic_renders() {
    let svg = render_fixture("mindmap_basic.puml");
    assert!(svg.contains("Project"));
    assert!(svg.contains("Backend"));
    assert!(svg.contains("Frontend"));
    assert!(svg.contains("API"));
    assert!(svg.contains("Monitoring"));
}

#[test]
fn mindmap_basic_snapshot() {
    let svg = render_fixture("mindmap_basic.puml");
    insta::assert_snapshot!(svg);
}
