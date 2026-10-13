//! A laid-out diagram as a [`diagram_scene::Scene`], for hosts that draw
//! it natively in their own colours (the emx editor). The positions are
//! the layouts' own, and edges take the routes the SVG renderer draws, so
//! a scene shows what `puml` would, in the host's theme.
//!
//! Sequence, class (with component, deployment and C4), activity and state
//! diagrams become scenes; the other kinds return `None`, for the host to
//! fall back on SVG.

use diagram_scene::{
    route, Anchor, Container, ContainerKind, Dash, Edge, EdgeLabel, Estimate, Font, Marker, Node,
    Paint, Pen, Point, Rect, Role, Scene, Seg, Shape, Style, Text, TextLine, Weight,
};

use crate::ast::class::{ClassKind, RelationKind};
use crate::ast::sequence::ParticipantKind;
use crate::ast::state::StateKind;
use crate::ast::DiagramAst;
use crate::layout;

/// puml lays text out for 13 px type.
const FONT_SIZE: f32 = 13.0;
/// The space a title takes above the diagram, as the renderers leave.
const TITLE_HEIGHT: f32 = 30.0;

/// `ast` laid out as a scene, or `None` for kinds drawn only as SVG
/// (use case, timing, mind map, Gantt).
pub fn scene(ast: &DiagramAst) -> Option<Scene> {
    Some(match ast {
        DiagramAst::Sequence(seq) => sequence(&layout::sequence::layout(seq)),
        DiagramAst::Class(cls) => class(&layout::class::layout(cls)),
        DiagramAst::Activity(act) => activity(&layout::activity::layout(act)),
        DiagramAst::State(st) => state(&layout::state::layout(st)),
        _ => return None,
    })
}

fn new_scene(width: f64, height: f64, title: &Option<String>) -> (Scene, f32) {
    let dy = if title.is_some() { TITLE_HEIGHT } else { 0.0 };
    let mut scene = Scene::new(width as f32, height as f32 + dy);
    scene.font_size = FONT_SIZE;
    if let Some(t) = title {
        scene.title = Some(Text {
            pos: Point::new(scene.width / 2.0, 22.0),
            anchor: Anchor::Middle,
            line: TextLine::new(t.clone(), Font::Title),
            paint: Paint::Role(Role::Text),
            backed: false,
        });
    }
    (scene, dy)
}

/// A colour the source set (`#rrggbb`, `#rgb`), if it can be read.
fn color(text: &str) -> Option<Paint> {
    let h = text.trim().trim_start_matches('#');
    let hex = |s: &str| u8::from_str_radix(s, 16).ok();
    let (r, g, b) = match h.len() {
        3 => {
            let d = |i: usize| hex(&h[i..i + 1]).map(|v| v * 17);
            (d(0)?, d(1)?, d(2)?)
        }
        6 | 8 => (hex(&h[0..2])?, hex(&h[2..4])?, hex(&h[4..6])?),
        _ => return None,
    };
    Some(Paint::Rgb(r, g, b))
}

fn lines(text: &str, font: Font) -> Vec<TextLine> {
    text.split("\\n")
        .flat_map(|l| l.lines())
        .map(|l| TextLine::new(l.to_string(), font))
        .collect()
}

fn label(pos: Point, text: &str) -> EdgeLabel {
    EdgeLabel::new(pos, lines(text, Font::Small), true, &Estimate, FONT_SIZE)
}

fn polyline(points: &[(f64, f64)], dy: f32) -> Vec<Seg> {
    let pts: Vec<Point> = points
        .iter()
        .map(|&(x, y)| Point::new(x as f32, y as f32 + dy))
        .collect();
    route::rounded_polyline(&pts, 6.0)
}

fn line(a: Point, b: Point) -> Vec<Seg> {
    vec![Seg::Move(a), Seg::Line(b)]
}

// ----- sequence ---------------------------------------------------------------

/// Participant boxes, as the renderer sizes them.
const PARTICIPANT_W: f32 = 120.0;
const PARTICIPANT_H: f32 = 40.0;
const SEQ_TOP: f32 = 20.0;

fn sequence(l: &layout::sequence::SequenceLayout) -> Scene {
    use layout::sequence::LayoutElement;
    let (mut scene, dy) = new_scene(l.total_width, l.total_height, &l.title);
    let top = dy + SEQ_TOP;
    let bottom = dy + l.lifeline_height as f32;
    // Room for the actors' names under the bottom row.
    scene.height = scene.height.max(bottom + PARTICIPANT_H + 22.0);

    for (i, p) in l.participants.iter().enumerate() {
        let x = p.x as f32;
        scene.containers.push(Container {
            rect: Rect::new(
                x - 1.0,
                top + PARTICIPANT_H,
                2.0,
                bottom - top - PARTICIPANT_H,
            ),
            kind: ContainerKind::Lifeline,
            label: Vec::new(),
            style: Style::for_container(&ContainerKind::Lifeline),
        });
        let rows = if l.hide_footbox {
            vec![top]
        } else {
            vec![top, bottom]
        };
        for (r, y) in rows.into_iter().enumerate() {
            let (shape, rect) = match p.kind {
                ParticipantKind::Actor => (
                    Shape::Actor,
                    Rect::new(x - 30.0, y, 60.0, PARTICIPANT_H + 18.0),
                ),
                ParticipantKind::Database => (
                    Shape::Cylinder,
                    Rect::new(
                        x - PARTICIPANT_W / 2.0,
                        y - 6.0,
                        PARTICIPANT_W,
                        PARTICIPANT_H + 12.0,
                    ),
                ),
                ParticipantKind::Queue => (
                    Shape::Queue,
                    Rect::new(x - PARTICIPANT_W / 2.0, y, PARTICIPANT_W, PARTICIPANT_H),
                ),
                _ => (
                    Shape::Rounded,
                    Rect::new(x - PARTICIPANT_W / 2.0, y, PARTICIPANT_W, PARTICIPANT_H),
                ),
            };
            let mut node = Node::new(format!("participant:{i}:{r}"), shape, rect);
            if shape == Shape::Actor {
                // Under the figure, on the page's colour: the first message
                // can run just below the box.
                scene.texts.push(Text {
                    pos: Point::new(x, rect.bottom() - 3.0),
                    anchor: Anchor::Middle,
                    line: TextLine::new(p.display.clone(), Font::Body),
                    paint: Paint::Role(Role::Text),
                    backed: true,
                });
            } else {
                node.label = lines(&p.display, Font::Body);
            }
            if let Some(fill) = p.color.as_deref().and_then(color) {
                node.style.fill = fill;
            } else if shape == Shape::Actor {
                node.style.fill = Paint::Role(Role::Surface);
            }
            scene.nodes.push(node);
        }
    }

    for element in &l.elements {
        match element {
            LayoutElement::Group(g) => {
                let rect = Rect::new(g.x as f32, g.y as f32 + dy, g.width as f32, g.height as f32);
                let mut guards =
                    vec![(rect.y, TextLine::new(format!("[{}]", g.label), Font::Small))];
                if g.label.is_empty() {
                    guards.clear();
                }
                for (y, text) in &g.section_breaks {
                    guards.push((
                        *y as f32 + dy,
                        TextLine::new(
                            text.as_ref().map_or(String::new(), |t| format!("[{t}]")),
                            Font::Small,
                        ),
                    ));
                }
                let kind = ContainerKind::Fragment {
                    operator: g.kind.to_lowercase(),
                    guards,
                };
                scene.containers.push(Container {
                    rect,
                    style: Style::for_container(&kind),
                    kind,
                    label: Vec::new(),
                });
            }
            LayoutElement::Message(m) => {
                let y = m.y as f32 + dy;
                let (a, b) = (m.from_x as f32, m.to_x as f32);
                let path = if m.self_msg {
                    let w = crate::render::sequence::self_loop_width(&m.label) as f32;
                    route::rounded_polyline(
                        &[
                            Point::new(a, y),
                            Point::new(a + w, y),
                            Point::new(a + w, y + 20.0),
                            Point::new(a, y + 20.0),
                        ],
                        4.0,
                    )
                } else {
                    line(Point::new(a, y), Point::new(b, y))
                };
                let mut edge = Edge::new(path);
                if m.dashed {
                    edge.pen.dash = Dash::Dashed;
                }
                if m.lost {
                    edge.end = Marker::Cross;
                }
                if !m.label.is_empty() {
                    let mid = if m.self_msg {
                        a + crate::render::sequence::self_loop_width(&m.label) as f32 / 2.0
                    } else {
                        (a + b) / 2.0
                    };
                    let mut l = label(Point::new(mid, y - 10.0), &m.label);
                    l.pill = false;
                    edge.labels.push(l);
                }
                scene.edges.push(edge);
            }
            LayoutElement::Note(n) => {
                let rect = Rect::new(n.x as f32, n.y as f32 + dy, n.width as f32, n.height as f32);
                let mut node = Node::new(format!("note:{}", scene.nodes.len()), Shape::Note, rect);
                node.label = n
                    .lines
                    .iter()
                    .map(|t| TextLine::new(t.clone(), Font::Body))
                    .collect();
                scene.nodes.push(node);
            }
            LayoutElement::Activation(a) => {
                let x = a.participant_x as f32 - 5.0 + a.depth as f32 * 3.0;
                let rect = Rect::new(
                    x,
                    a.y_start as f32 + dy,
                    10.0,
                    ((a.y_end - a.y_start) as f32).max(4.0),
                );
                let node = Node::new(
                    format!("activation:{}", scene.nodes.len()),
                    Shape::Rect,
                    rect,
                );
                scene.nodes.push(node);
            }
            LayoutElement::Divider(d) => {
                let y = d.y as f32 + dy;
                for off in [-2.0, 2.0] {
                    scene.edges.push(Edge {
                        end: Marker::None,
                        pen: Pen {
                            dash: Dash::Solid,
                            weight: Weight::Thin,
                            paint: Paint::Role(Role::Muted),
                        },
                        ..Edge::new(line(
                            Point::new(0.0, y + off),
                            Point::new(d.total_width as f32, y + off),
                        ))
                    });
                }
                scene.texts.push(Text {
                    pos: Point::new(d.total_width as f32 / 2.0, y + 4.0),
                    anchor: Anchor::Middle,
                    line: TextLine::new(d.label.clone(), Font::Bold),
                    paint: Paint::Role(Role::Text),
                    backed: true,
                });
            }
        }
    }
    scene
}

// ----- class --------------------------------------------------------------------

fn class(l: &layout::class::ClassLayout) -> Scene {
    let (mut scene, dy) = new_scene(l.total_width, l.total_height, &l.title);
    for b in &l.boundaries {
        let kind = ContainerKind::Boundary;
        scene.containers.push(Container {
            rect: Rect::new(b.x as f32, b.y as f32 + dy, b.width as f32, b.height as f32),
            style: Style::for_container(&kind),
            kind,
            label: vec![TextLine::new(b.label.clone(), Font::Bold)],
        });
    }

    let mut edges: Vec<(Vec<(f64, f64)>, &layout::class::EdgeLayout)> =
        l.edges.iter().map(|e| (e.points.clone(), e)).collect();
    for (i, n) in l.nodes.iter().enumerate() {
        let shape = match n.kind {
            ClassKind::Class
            | ClassKind::Abstract
            | ClassKind::Interface
            | ClassKind::Enum
            | ClassKind::Annotation
            | ClassKind::Object => Shape::Class,
            ClassKind::Component => Shape::Component,
            ClassKind::Cloud => Shape::Cloud,
            ClassKind::Database => Shape::Cylinder,
            ClassKind::Folder => Shape::Package,
            ClassKind::Artifact => Shape::Document,
            ClassKind::Queue => Shape::Queue,
            ClassKind::Node | ClassKind::Frame | ClassKind::Rectangle => Shape::Rect,
        };
        let rect = Rect::new(n.x as f32, n.y as f32 + dy, n.width as f32, n.height as f32);
        let mut node = Node::new(format!("class:{i}:{}", n.name), shape, rect);
        let kind_label = match n.kind {
            ClassKind::Interface => Some("interface"),
            ClassKind::Abstract => Some("abstract"),
            ClassKind::Enum => Some("enum"),
            ClassKind::Annotation => Some("annotation"),
            ClassKind::Component => Some("component"),
            ClassKind::Node => Some("node"),
            ClassKind::Cloud => Some("cloud"),
            ClassKind::Database => Some("database"),
            ClassKind::Folder => Some("folder"),
            ClassKind::Frame => Some("frame"),
            ClassKind::Artifact => Some("artifact"),
            ClassKind::Queue => Some("queue"),
            _ => None,
        };
        if let Some(st) = n.stereotype.as_deref().or(kind_label) {
            node.stereotype = vec![TextLine::new(format!("\u{ab}{st}\u{bb}"), Font::Small)];
        }
        let name_font = if n.kind == ClassKind::Abstract {
            Font::Italic
        } else {
            Font::Bold
        };
        node.label = vec![TextLine::new(n.display_name.clone(), name_font)];
        node.style.fill = Paint::Role(match n.kind {
            ClassKind::Interface | ClassKind::Annotation => Role::Container,
            ClassKind::Enum => Role::Component,
            ClassKind::Database => Role::Database,
            ClassKind::Component => Role::Container,
            _ => Role::Surface,
        });
        if shape == Shape::Class {
            node.compartments = n
                .member_sections
                .iter()
                .map(|s| {
                    s.members
                        .iter()
                        .map(|m| {
                            let font = if m.is_abstract {
                                Font::Italic
                            } else {
                                Font::Body
                            };
                            TextLine::new(m.text.clone(), font)
                        })
                        .collect()
                })
                .collect();
            if node.compartments.is_empty() {
                node.compartments = vec![Vec::new()];
            }
            // Tall enough for its rows in the scene's metrics; edges meeting
            // the old bottom move down with it.
            let (w, h) = diagram_scene::shape::natural_size(&node, FONT_SIZE, &Estimate);
            let old_bottom = (n.y + n.height) as f32;
            if h > node.rect.height {
                let grow = h - node.rect.height;
                node.rect.height = h;
                for (points, _) in &mut edges {
                    for p in points.iter_mut() {
                        let inside_x =
                            (p.0 as f32) >= rect.x - 0.5 && (p.0 as f32) <= rect.right() + 0.5;
                        if inside_x && (p.1 as f32 - old_bottom).abs() < 0.5 {
                            p.1 += grow as f64;
                        }
                    }
                }
            }
            node.rect.width = node.rect.width.max(w);
        }
        scene.nodes.push(node);
    }

    for (points, e) in &edges {
        let mut edge = Edge::new(polyline(points, dy));
        let (dash, end) = match e.kind {
            RelationKind::Extension => (Dash::Solid, Marker::HollowTriangle),
            RelationKind::Implementation | RelationKind::Realization => {
                (Dash::Dashed, Marker::HollowTriangle)
            }
            RelationKind::Composition => (Dash::Solid, Marker::FilledDiamond),
            RelationKind::Aggregation => (Dash::Solid, Marker::HollowDiamond),
            RelationKind::Dependency => (Dash::Solid, Marker::Arrow),
            RelationKind::DashedLink => (Dash::Dashed, Marker::Arrow),
            RelationKind::Association => (Dash::Solid, Marker::None),
        };
        edge.pen.dash = dash;
        edge.end = end;
        if let Some(text) = &e.label {
            let (x, y, _) = crate::render::primitives::label_perpendicular(points, 8.0);
            edge.labels
                .push(label(Point::new(x as f32, y as f32 + dy), text));
        }
        let ends = [
            (points.first(), &e.from_label),
            (points.last(), &e.to_label),
        ];
        for (at, text) in ends {
            if let (Some(&(x, y)), Some(text)) = (at, text) {
                scene.texts.push(Text {
                    pos: Point::new(x as f32 + 6.0, y as f32 + dy - 4.0),
                    anchor: Anchor::Start,
                    line: TextLine::new(text.clone(), Font::Small),
                    paint: Paint::Role(Role::Muted),
                    backed: true,
                });
            }
        }
        scene.edges.push(edge);
    }

    for (i, note) in l.notes.iter().enumerate() {
        let rect = Rect::new(
            note.x as f32,
            note.y as f32 + dy,
            note.width as f32,
            note.height as f32,
        );
        let mut node = Node::new(format!("note:{i}"), Shape::Note, rect);
        node.label = note
            .lines
            .iter()
            .map(|t| TextLine::new(t.clone(), Font::Body))
            .collect();
        let target = Point::new(note.target_x as f32, note.target_y as f32 + dy);
        let from = route::clip(Shape::Rect, rect, target);
        scene.edges.push(Edge {
            end: Marker::None,
            pen: Pen {
                dash: Dash::Dashed,
                weight: Weight::Thin,
                paint: Paint::Role(Role::Muted),
            },
            ..Edge::new(line(from, target))
        });
        scene.nodes.push(node);
    }
    scene
}

// ----- activity -------------------------------------------------------------------

fn activity(l: &layout::activity::ActivityLayout) -> Scene {
    use layout::activity::Shape as A;
    let (mut scene, dy) = new_scene(l.total_width, l.total_height, &l.title);
    for (i, points) in crate::render::activity::edge_routes(l) {
        let e = &l.edges[i];
        let mut edge = Edge::new(polyline(&points, dy));
        if e.dashed {
            edge.pen.dash = Dash::Dashed;
        }
        if let Some(text) = &e.label {
            let (x, y, _) = crate::render::primitives::label_perpendicular(&points, 8.0);
            edge.labels
                .push(label(Point::new(x as f32, y as f32 + dy), text));
        }
        scene.edges.push(edge);
    }
    for n in &l.nodes {
        let rect = Rect::new(n.x as f32, n.y as f32 + dy, n.w as f32, n.h as f32);
        let shape = match n.shape {
            A::StartEnd if n.label == "stop" => Shape::End,
            A::StartEnd => Shape::Start,
            A::Action => Shape::Rounded,
            A::Decision => Shape::Diamond,
            A::MergeBar => Shape::ForkBar,
            A::Note => Shape::Note,
            A::Arrow => continue,
        };
        let mut node = Node::new(format!("activity:{}", n.id), shape, rect);
        if matches!(shape, Shape::Rounded | Shape::Diamond | Shape::Note) {
            node.label = lines(&n.label, Font::Body);
        }
        if let Some(fill) = n.color.as_deref().and_then(color) {
            node.style.fill = fill;
        }
        scene.nodes.push(node);
    }
    scene
}

// ----- state ------------------------------------------------------------------------

fn state(l: &layout::state::StateLayout) -> Scene {
    let (mut scene, dy) = new_scene(l.total_width, l.total_height, &l.title);
    for (i, points) in crate::render::state::edge_routes(l) {
        let e = &l.edges[i];
        let mut edge = Edge::new(polyline(&points, dy));
        if let Some(text) = &e.label {
            let (x, y, _) = crate::render::primitives::label_perpendicular(&points, 8.0);
            edge.labels
                .push(label(Point::new(x as f32, y as f32 + dy), text));
        }
        scene.edges.push(edge);
    }
    for n in &l.nodes {
        let rect = Rect::new(n.x as f32, n.y as f32 + dy, n.w as f32, n.h as f32);
        let shape = match n.kind {
            StateKind::Normal => Shape::Rounded,
            StateKind::Start => Shape::Start,
            StateKind::Choice => Shape::Diamond,
            StateKind::Fork | StateKind::Join => Shape::ForkBar,
        };
        let mut node = Node::new(format!("state:{}", n.name), shape, rect);
        if shape == Shape::Rounded {
            node.label = lines(n.label.as_deref().unwrap_or(&n.name), Font::Body);
        }
        scene.nodes.push(node);
    }
    scene
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_of(source: &str) -> Scene {
        let diagrams = crate::parser::preprocessor::preprocess(source, None);
        let ast = crate::parser::parse(&diagrams[0]).expect("parses");
        scene(&ast).expect("a scene")
    }

    fn inside(scene: &Scene) {
        let bounds = scene.bounds().inflate(2.0);
        for n in &scene.nodes {
            assert!(
                bounds.encloses(&n.rect, 4.0),
                "{} at {:?} outside {:?}",
                n.id,
                n.rect,
                bounds
            );
        }
    }

    #[test]
    fn sequences_have_participants_messages_and_fragments() {
        let s = scene_of(
            "@startuml\ntitle Checkout\nactor User\nparticipant Shop\ndatabase DB\nUser -> Shop: buy\nactivate Shop\nShop -> DB: save\nalt ok\n  DB --> Shop: saved\nelse failed\n  DB --> Shop: error\nend\nShop -> Shop: log\nnote right of Shop: receipt\n== Later ==\nShop ->x User: lost\n@enduml\n",
        );
        let texts = s.texts();
        for t in [
            "Checkout", "User", "Shop", "DB", "buy", "save", "saved", "receipt", "Later",
        ] {
            assert!(texts.iter().any(|x| x.contains(t)), "{t}: {texts:?}");
        }
        assert!(s.nodes.iter().any(|n| n.shape == Shape::Actor));
        assert!(s.nodes.iter().any(|n| n.shape == Shape::Cylinder));
        assert!(s.nodes.iter().any(|n| n.shape == Shape::Note));
        assert!(s.containers.iter().any(|c| matches!(&c.kind, ContainerKind::Fragment { operator, guards } if operator == "alt" && guards.len() == 2)));
        assert!(s.edges.iter().any(|e| e.pen.dash == Dash::Dashed));
        assert!(s.edges.iter().any(|e| e.end == Marker::Cross));
        inside(&s);
    }

    #[test]
    fn classes_get_compartments_and_uml_markers() {
        let s = scene_of(
            "@startuml\nabstract class Shape {\n  +area(): f64\n}\nclass Circle {\n  -r: f64\n  +area(): f64\n}\ninterface Draw\nShape <|-- Circle\nDraw <|.. Circle\nCircle *-- Point\nCircle o-- Style\nCircle --> Canvas : draws on\n@enduml\n",
        );
        let circle = s.nodes.iter().find(|n| n.id.ends_with(":Circle")).unwrap();
        assert_eq!(circle.shape, Shape::Class);
        assert!(circle
            .compartments
            .iter()
            .flatten()
            .any(|l| l.plain().contains("r: f64")));
        let ends: Vec<Marker> = s.edges.iter().map(|e| e.end).collect();
        for m in [
            Marker::HollowTriangle,
            Marker::FilledDiamond,
            Marker::HollowDiamond,
            Marker::Arrow,
        ] {
            assert!(ends.contains(&m), "{m:?} in {ends:?}");
        }
        assert!(s.texts().iter().any(|t| t == "draws on"));
        // Each class is tall enough for its rows.
        for n in s.nodes.iter().filter(|n| n.shape == Shape::Class) {
            let (_, h) = diagram_scene::shape::natural_size(n, FONT_SIZE, &Estimate);
            assert!(
                h <= n.rect.height + 0.5,
                "{}: {h} > {}",
                n.id,
                n.rect.height
            );
        }
        inside(&s);
    }

    #[test]
    fn activities_and_states_route_their_edges() {
        let s = scene_of(
            "@startuml\nstart\n:Read;\nif (ok?) then (yes)\n  :Save;\nelse (no)\n  :Retry;\nendif\nstop\n@enduml\n",
        );
        assert!(s.nodes.iter().any(|n| n.shape == Shape::Start));
        assert!(s.nodes.iter().any(|n| n.shape == Shape::End));
        assert!(s.nodes.iter().any(|n| n.shape == Shape::Diamond));
        assert!(!s.edges.is_empty());
        inside(&s);

        let s = scene_of(
            "@startuml\n[*] --> Idle\nIdle --> Busy : start\nBusy --> Idle : done\nBusy --> [*]\n@enduml\n",
        );
        assert!(s.nodes.iter().any(|n| n.shape == Shape::Start));
        assert!(s.texts().iter().any(|t| t == "start"));
        assert_eq!(s.edges.len(), 4);
        inside(&s);
    }

    #[test]
    fn other_kinds_are_left_to_svg() {
        let diagrams = crate::parser::preprocessor::preprocess(
            "@startmindmap\n* root\n** child\n@endmindmap\n",
            None,
        );
        let ast = crate::parser::parse(&diagrams[0]).unwrap();
        assert!(scene(&ast).is_none());
    }
}
