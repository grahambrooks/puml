#[derive(Debug, Clone, PartialEq)]
pub enum Side {
    /// Auto — renderer chooses based on balance.
    Auto,
    /// `+` prefix — forced right-of-root.
    Right,
    /// `-` prefix — forced left-of-root.
    Left,
}

#[derive(Debug, Clone)]
pub struct MindMapNode {
    pub label: String,
    pub depth: usize, // 1 = root
    pub side: Side,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MindMapDiagram {
    pub title: Option<String>,
    pub nodes: Vec<MindMapNode>, // flat list in source order; tree is reconstructed by depth
    pub skinparams: Vec<(String, String)>,
}
