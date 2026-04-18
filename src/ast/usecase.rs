#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Actor,   // rendered as a stick figure
    UseCase, // rendered as an ellipse
}

#[derive(Debug, Clone)]
pub struct UseCaseNode {
    pub name: String,          // canonical identifier used to match associations
    pub label: Option<String>, // display text if different from name
    pub kind: NodeKind,
    pub stereotype: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssocKind {
    Solid,  // -> / --> / --
    Dashed, // ..> / ..
}

#[derive(Debug, Clone)]
pub struct Association {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: AssocKind,
}

#[derive(Debug, Clone, Default)]
pub struct UseCaseDiagram {
    pub title: Option<String>,
    pub nodes: Vec<UseCaseNode>,
    pub associations: Vec<Association>,
    pub skinparams: Vec<(String, String)>,
}
