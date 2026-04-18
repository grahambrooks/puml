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
fn activity_basic_renders() {
    let svg = render_fixture("activity_basic.puml");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Read input"));
    assert!(svg.contains("Process data"));
}

#[test]
fn activity_basic_snapshot() {
    let svg = render_fixture("activity_basic.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn activity_flow_renders() {
    let svg = render_fixture("activity_flow.puml");
    assert!(svg.contains("Read request"));
    assert!(svg.contains("Load user"));
    assert!(svg.contains("Reject"));
    assert!(svg.contains("Send response"));
}

#[test]
fn activity_flow_snapshot() {
    let svg = render_fixture("activity_flow.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn activity_no_negative_x() {
    let svg = render_fixture("activity_basic.puml");
    // The if/else branches were previously landing with negative `x=-45`.
    // Ensure we no longer emit off-canvas shapes.
    assert!(
        !svg.contains("x=\"-"),
        "layout emits negative x attributes: {}",
        svg
    );
}

#[test]
fn while_repeat_renders() {
    let svg = render_fixture("activity_while_repeat.puml");
    assert!(svg.contains("more items?"));
    assert!(svg.contains("Take item"));
    assert!(svg.contains("Poll queue"));
    assert!(svg.contains("Shutdown"));
}

#[test]
fn while_repeat_snapshot() {
    let svg = render_fixture("activity_while_repeat.puml");
    insta::assert_snapshot!(svg);
}

#[test]
fn fork_renders() {
    let svg = render_fixture("activity_fork.puml");
    assert!(svg.contains("Validate headers"));
    assert!(svg.contains("Validate body"));
    assert!(svg.contains("Log request"));
    assert!(svg.contains("Dispatch handler"));
}

#[test]
fn fork_snapshot() {
    let svg = render_fixture("activity_fork.puml");
    insta::assert_snapshot!(svg);
}
