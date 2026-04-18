pub mod activity;
pub mod class;
pub mod sequence;
pub mod state;
pub mod usecase;

pub use class::ClassDiagram;
pub use sequence::SequenceDiagram;

#[derive(Debug)]
pub enum DiagramAst {
    Sequence(SequenceDiagram),
    Class(ClassDiagram),
    Activity(activity::ActivityDiagram),
    State(state::StateDiagram),
    UseCase(usecase::UseCaseDiagram),
}
