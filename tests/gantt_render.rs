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
fn gantt_basic_renders() {
    let svg = render_fixture("gantt_basic.puml");
    assert!(svg.contains("Design"));
    assert!(svg.contains("Build"));
    assert!(svg.contains("Test"));
    assert!(svg.contains("Launch"));
    // Milestone renders as a polygon (diamond), tasks as rects.
    assert!(
        svg.contains("<polygon"),
        "milestone should render as a polygon diamond"
    );
    assert!(svg.contains("<rect"), "task should render as a rectangle");
}

#[test]
fn gantt_basic_snapshot() {
    let svg = render_fixture("gantt_basic.puml");
    insta::assert_snapshot!(svg);
}
