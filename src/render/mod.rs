pub mod class;
pub mod primitives;
pub mod sequence;

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
    }
}
