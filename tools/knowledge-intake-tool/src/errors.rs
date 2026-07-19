use std::{fmt, io};

#[derive(Debug)]
pub enum KnowledgeToolError {
    Io(io::Error),
    MissingText,
    UnsupportedFileType(String),
}

impl fmt::Display for KnowledgeToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::MissingText => write!(f, "source has no analyzable text"),
            Self::UnsupportedFileType(ext) => write!(f, "unsupported file type: {ext}"),
        }
    }
}

impl std::error::Error for KnowledgeToolError {}

impl From<io::Error> for KnowledgeToolError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub type KnowledgeToolResult<T> = Result<T, KnowledgeToolError>;
