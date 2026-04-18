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
fn timing_basic_renders() {
    let svg = render_fixture("timing_basic.puml");
    // Lane labels
    assert!(svg.contains("Browser"));
    assert!(svg.contains("User"));
    // Distinct state labels
    assert!(svg.contains("Idle"));
    assert!(svg.contains("Waiting"));
    assert!(svg.contains("Processing"));
    // Time ticks
    assert!(svg.contains(">0<") || svg.contains(">\n0\n<"));
    assert!(svg.contains(">100<") || svg.contains(">\n100\n<"));
    assert!(svg.contains(">300<") || svg.contains(">\n300\n<"));
}

#[test]
fn timing_basic_snapshot() {
    let svg = render_fixture("timing_basic.puml");
    insta::assert_snapshot!(svg);
}
