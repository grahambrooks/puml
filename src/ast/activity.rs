#[derive(Debug, Clone)]
pub enum ActivityElement {
    Start,
    Stop,
    End,
    Action(Action),
    If(IfBlock),
    While(WhileBlock),
    Repeat(RepeatBlock),
    Fork(ForkBlock),
    Partition(Partition),
    Note(ActivityNote),
    Label(String),         // :label;  with no action — used as a named anchor
    Arrow(Option<String>), // -> optional label ;
    Detach,
    Kill,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub label: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IfBlock {
    pub condition: String,
    pub then_label: Option<String>,
    pub then_body: Vec<ActivityElement>,
    pub elseif_branches: Vec<ElseIfBranch>,
    pub else_label: Option<String>,
    pub else_body: Vec<ActivityElement>,
}

#[derive(Debug, Clone)]
pub struct ElseIfBranch {
    #[allow(dead_code)]
    pub condition: String,
    pub label: Option<String>,
    pub body: Vec<ActivityElement>,
}

#[derive(Debug, Clone)]
pub struct WhileBlock {
    pub condition: String,
    pub exit_label: Option<String>,
    pub body: Vec<ActivityElement>,
}

#[derive(Debug, Clone)]
pub struct RepeatBlock {
    pub body: Vec<ActivityElement>,
    pub condition: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ForkBlock {
    pub branches: Vec<Vec<ActivityElement>>,
    #[allow(dead_code)]
    pub join: bool, // true = fork/join, false = fork/end merge
}

#[derive(Debug, Clone)]
pub struct Partition {
    #[allow(dead_code)]
    pub name: String,
    pub body: Vec<ActivityElement>,
}

#[derive(Debug, Clone)]
pub struct ActivityNote {
    pub text: String,
    #[allow(dead_code)]
    pub position: NotePos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotePos {
    Left,
    Right,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityDiagram {
    pub title: Option<String>,
    pub elements: Vec<ActivityElement>,
}
