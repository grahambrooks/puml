#[derive(Debug, Clone, Default)]
pub struct StateDiagram {
    pub title: Option<String>,
    pub states: Vec<StateNode>,
    pub transitions: Vec<Transition>,
    pub skinparams: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct StateNode {
    pub name: String,
    pub label: Option<String>,
    pub kind: StateKind,
    pub body: Vec<StateElement>, // for composite states
}

#[derive(Debug, Clone, PartialEq)]
pub enum StateKind {
    Normal,
    Start,  // [*] — rendered identically whether used as source or target
    Choice, // <<choice>>
    Fork,   // <<fork>>
    Join,   // <<join>>
}

#[derive(Debug, Clone)]
pub enum StateElement {
    SubState(#[allow(dead_code)] StateNode),
    Transition(#[allow(dead_code)] Transition),
    Divider,
    #[allow(dead_code)]
    Note(StateNote),
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StateNote {
    pub position: NotePos,
    pub text: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum NotePos {
    Left,
    Right,
}
