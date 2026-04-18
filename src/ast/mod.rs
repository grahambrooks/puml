pub mod class;
pub mod sequence;

pub use class::ClassDiagram;
pub use sequence::SequenceDiagram;

#[derive(Debug)]
pub enum DiagramAst {
    Sequence(SequenceDiagram),
    Class(ClassDiagram),
}
