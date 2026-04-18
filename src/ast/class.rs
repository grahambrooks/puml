#[derive(Debug, Clone, PartialEq)]
pub enum ClassKind {
    Class,
    Abstract,
    Interface,
    Enum,
    Annotation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,    // +
    Private,   // -
    Protected, // #
    Package,   // ~
    None,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub visibility: Visibility,
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_static: bool,
    pub is_abstract: bool,
    pub is_method: bool,
    pub params: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClassNode {
    pub name: String,
    pub generics: Option<String>,
    pub kind: ClassKind,
    pub stereotype: Option<String>,
    pub color: Option<String>,
    pub members: Vec<Member>,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NotePosition {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone)]
pub struct ClassNote {
    pub position: NotePosition,
    pub target: String,
    pub lines: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum RelationKind {
    Extension,
    Implementation,
    Composition,
    Aggregation,
    Association,
    Dependency,
    DashedLink,
    Realization,
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    pub from_label: Option<String>,
    pub to_label: Option<String>,
    pub label: Option<String>,
    #[allow(dead_code)]
    pub reversed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ClassDiagram {
    pub title: Option<String>,
    pub classes: Vec<ClassNode>,
    pub relations: Vec<Relation>,
    pub notes: Vec<ClassNote>,
    pub hide_empty_members: bool,
}
