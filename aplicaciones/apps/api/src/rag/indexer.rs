use crate::{
    error::AppResult,
    rag::{loaders, qdrant_store, splitter},
    state::AppState,
};
use serde::Serialize;
use std::path::Path;
use tracing::info;
use walkdir::WalkDir;

#[derive(Serialize)]
pub struct IndexReport {
    pub files: usize,
    pub chunks: usize,
}

pub async fn reindex(state: &AppState) -> AppResult<IndexReport> {
    qdrant_store::clear_collection(state).await?;
    let mut files = 0;
    let mut chunks_total = 0;

    for entry in WalkDir::new(&state.config.docs_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(document) = loaders::load(path)? else {
            continue;
        };
        let chunks = splitter::split_text(
            &document.text,
            state.config.chunk_size,
            state.config.chunk_overlap,
        );
        let source_file = path.to_string_lossy().to_string();
        let file_name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("document")
            .to_owned();
        qdrant_store::upsert_chunks(
            state,
            &source_file,
            &file_name,
            &document.document_type,
            &chunks,
        )
        .await?;
        insert_document_row(state, path, &document.document_type, chunks.len()).await?;
        files += 1;
        chunks_total += chunks.len();
    }

    info!(files, chunks = chunks_total, "reindexed documents");
    Ok(IndexReport {
        files,
        chunks: chunks_total,
    })
}

async fn insert_document_row(
    state: &AppState,
    path: &Path,
    document_type: &str,
    chunk_count: usize,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO source_documents (source_file, file_name, document_type, chunk_count)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(path.to_string_lossy().to_string())
    .bind(
        path.file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("document"),
    )
    .bind(document_type)
    .bind(chunk_count as i32)
    .execute(&state.db)
    .await?;
    Ok(())
}
