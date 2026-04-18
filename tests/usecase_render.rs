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
fn usecase_basic_renders() {
    let svg = render_fixture("usecase_basic.puml");
    // Actor stick figures + elliptical use cases should all land.
    assert!(svg.contains("User"));
    assert!(svg.contains("Admin"));
    assert!(svg.contains("Login"));
    assert!(svg.contains("Manage Users"));
    // Ellipse = <ellipse …> element, stick figures have a head Circle.
    assert!(
        svg.contains("<ellipse"),
        "use cases should render as ellipses"
    );
    assert!(svg.contains("<circle"), "actors should have a head circle");
}

#[test]
fn usecase_basic_snapshot() {
    let svg = render_fixture("usecase_basic.puml");
    insta::assert_snapshot!(svg);
}
