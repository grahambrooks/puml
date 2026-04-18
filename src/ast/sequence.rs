#[derive(Debug, Clone, PartialEq)]
pub enum ParticipantKind {
    Participant,
    Actor,
    Boundary,
    Control,
    Entity,
    Database,
    Collections,
    Queue,
}

#[derive(Debug, Clone)]
pub struct Participant {
    pub name: String,
    pub alias: Option<String>,
    pub kind: ParticipantKind,
    pub color: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowStyle {
    Solid,       // ->
    Dashed,      // -->
    SolidThick,  // ->>
    DashedThick, // -->>
    SolidNoHead, // ->)  / -\
    Lost,        // ->x
    Async,       // ->>  (right pointing)
}

#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub label: String,
    pub arrow: ArrowStyle,
    #[allow(dead_code)]
    pub activate: bool,
    #[allow(dead_code)]
    pub deactivate: bool,
    #[allow(dead_code)]
    pub return_msg: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotePosition {
    Left,
    Right,
    Over,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub position: NotePosition,
    pub participants: Vec<String>,
    pub lines: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct GroupBlock {
    pub kind: String,
    pub label: String,
    pub sections: Vec<(Option<String>, Vec<SequenceElement>)>,
}

#[derive(Debug, Clone)]
pub struct Divider {
    pub label: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum SequenceElement {
    Message(Message),
    Note(Note),
    Group(GroupBlock),
    Divider(Divider),
    Activate(String),
    Deactivate(String),
    Delay(String),
    Space(u32),
    Autonumber(Option<u32>),
}

#[derive(Debug, Clone, Default)]
pub struct SequenceDiagram {
    pub title: Option<String>,
    pub participants: Vec<Participant>,
    pub elements: Vec<SequenceElement>,
    pub hide_footbox: bool,
    pub skinparams: Vec<(String, String)>,
}

impl SequenceDiagram {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns ordered list of participant names as they appear (declared or first used).
    pub fn ordered_participants(&self) -> Vec<&Participant> {
        self.participants.iter().collect()
    }
}
