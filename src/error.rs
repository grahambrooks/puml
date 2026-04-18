use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum PumlError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("Unknown diagram type: {0}")]
    UnknownDiagramType(String),

    #[error("Layout error: {0}")]
    Layout(String),
}
