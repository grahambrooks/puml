#[derive(Debug, Clone, PartialEq)]
pub enum ClassKind {
    Class,
    Abstract,
    Interface,
    Enum,
    Annotation,
    Object,    // `object Foo { ... }` — instance-style, name rendered underlined
    Component, // `component Foo` — box with two port tabs on the left edge
    // Deployment containers — same layout as classes, different shapes.
    Node,      // 3D box with slanted top
    Cloud,     // cloud outline
    Database,  // cylinder
    Folder,    // folder tab
    Frame,     // box with corner tab
    Rectangle, // plain rectangle
    Artifact,  // document page icon
    Queue,     // elongated cylinder
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
    /// Alias from `class "Foo" as F` — relations may reference the node
    /// by either `name` or `alias`, so layout must check both when
    /// resolving edge endpoints.
    pub alias: Option<String>,
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

/// A C4 (or future container) boundary: a labeled rectangle drawn around
/// a set of classes after they're laid out. The rectangle does not influence
/// layout — children are placed by the normal class engine and the boundary
/// is a post-layout overlay sized to fit them.
#[derive(Debug, Clone)]
pub struct Boundary {
    pub alias: String,
    pub label: String,
    pub kind: String,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ClassDiagram {
    pub title: Option<String>,
    pub classes: Vec<ClassNode>,
    pub relations: Vec<Relation>,
    pub notes: Vec<ClassNote>,
    pub hide_empty_members: bool,
    pub skinparams: Vec<(String, String)>,
    /// True for diagrams that came from a `!include <C4/...>` source. C4
    /// reads top-down from caller to callee — the inverse of UML class
    /// inheritance — so rank propagation flips for Dependency edges.
    pub c4_mode: bool,
    pub boundaries: Vec<Boundary>,
}
