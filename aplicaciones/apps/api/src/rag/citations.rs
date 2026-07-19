use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceChunk {
    pub text: String,
    pub score: f32,
    pub source_file: String,
    pub file_name: String,
    pub document_type: String,
    pub page_number: Option<i32>,
    pub chunk_index: usize,
}
