pub mod activity;
pub mod class;
pub mod primitives;
pub mod sequence;
pub mod state;

use crate::ast::DiagramAst;
use crate::layout;
use svg::Document;

pub fn render(ast: &DiagramAst) -> Document {
    match ast {
        DiagramAst::Sequence(seq) => {
            let layout = layout::sequence::layout(seq);
            sequence::render(&layout)
        }
        DiagramAst::Class(cls) => {
            let layout = layout::class::layout(cls);
            class::render(&layout)
        }
        DiagramAst::Activity(act) => {
            let layout = layout::activity::layout(act);
            activity::render(&layout)
        }
        DiagramAst::State(st) => {
            let layout = layout::state::layout(st);
            state::render(&layout)
        }
    }
}
