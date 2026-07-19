use crate::{
    error::{AppError, AppResult},
    rag::citations::SourceChunk,
    state::AppState,
};
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn ensure_collection(state: &AppState) -> AppResult<()> {
    let url = format!(
        "{}/collections/{}",
        state.config.qdrant_url, state.config.qdrant_collection
    );
    let response = state.http.get(&url).send().await.map_err(upstream)?;
    if response.status().is_success() {
        let body: Value = response.json().await.map_err(upstream)?;
        let actual = body["result"]["config"]["params"]["vectors"]["size"].as_u64();
        let expected = state.embeddings.dimensions() as u64;
        if actual == Some(expected) {
            return Ok(());
        }
        return Err(AppError::BadRequest(format!(
            "Qdrant collection '{}' has vector size {:?}, but embedding provider '{}' model '{}' needs {}. Delete the collection or run POST /api/rag/reindex after resetting Qdrant storage.",
            state.config.qdrant_collection,
            actual,
            state.embeddings.provider_name(),
            state.embeddings.model_name(),
            expected
        )));
    }
    if response.status() != reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::Upstream(
            response.text().await.unwrap_or_default(),
        ));
    }

    let body = json!({
        "vectors": {
            "size": state.embeddings.dimensions(),
            "distance": "Cosine"
        }
    });
    let response = state
        .http
        .put(url)
        .json(&body)
        .send()
        .await
        .map_err(upstream)?;
    if response.status().is_success() || response.status() == reqwest::StatusCode::CONFLICT {
        Ok(())
    } else {
        Err(AppError::Upstream(
            response.text().await.unwrap_or_default(),
        ))
    }
}

pub async fn clear_collection(state: &AppState) -> AppResult<()> {
    let url = format!(
        "{}/collections/{}",
        state.config.qdrant_url, state.config.qdrant_collection
    );
    let _ = state.http.delete(url).send().await;
    ensure_collection(state).await
}

pub async fn upsert_chunks(
    state: &AppState,
    source_file: &str,
    file_name: &str,
    document_type: &str,
    chunks: &[String],
) -> AppResult<()> {
    if chunks.is_empty() {
        return Ok(());
    }
    let vectors = state
        .embeddings
        .embed_documents(chunks.to_vec())
        .await
        .map_err(AppError::Anyhow)?;
    let points: Vec<Value> = chunks
        .iter()
        .zip(vectors)
        .enumerate()
        .map(|(chunk_index, (chunk, vector))| {
            json!({
                "id": Uuid::new_v4().to_string(),
                "vector": vector,
                "payload": {
                    "text": chunk,
                    "source_file": source_file,
                    "file_name": file_name,
                    "document_type": document_type,
                    "page_number": null,
                    "chunk_index": chunk_index,
                    "language": "ru"
                }
            })
        })
        .collect();

    let url = format!(
        "{}/collections/{}/points?wait=true",
        state.config.qdrant_url, state.config.qdrant_collection
    );
    let response = state
        .http
        .put(url)
        .json(&json!({ "points": points }))
        .send()
        .await
        .map_err(upstream)?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(AppError::Upstream(
            response.text().await.unwrap_or_default(),
        ))
    }
}

pub async fn search(state: &AppState, query: &str) -> AppResult<Vec<SourceChunk>> {
    let vector = state
        .embeddings
        .embed_query(query.to_owned())
        .await
        .map_err(AppError::Anyhow)?;
    let url = format!(
        "{}/collections/{}/points/search",
        state.config.qdrant_url, state.config.qdrant_collection
    );
    let response = state
        .http
        .post(url)
        .json(&json!({
            "vector": vector,
            "limit": state.config.top_k,
            "with_payload": true
        }))
        .send()
        .await
        .map_err(upstream)?;
    if !response.status().is_success() {
        return Err(AppError::Upstream(
            response.text().await.unwrap_or_default(),
        ));
    }
    let body: Value = response.json().await.map_err(upstream)?;
    let chunks = body["result"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(parse_chunk)
        .collect();
    Ok(chunks)
}

pub async fn count_points(state: &AppState) -> Result<u64, reqwest::Error> {
    let url = format!(
        "{}/collections/{}/points/count",
        state.config.qdrant_url, state.config.qdrant_collection
    );
    let response = state
        .http
        .post(url)
        .json(&json!({ "exact": true }))
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(0);
    }
    let body: Value = response.error_for_status()?.json().await?;
    Ok(body["result"]["count"].as_u64().unwrap_or(0))
}

fn parse_chunk(value: &Value) -> Option<SourceChunk> {
    let payload = value["payload"].as_object()?;
    Some(SourceChunk {
        text: payload.get("text")?.as_str()?.to_owned(),
        score: value["score"].as_f64().unwrap_or(0.0) as f32,
        source_file: payload.get("source_file")?.as_str()?.to_owned(),
        file_name: payload.get("file_name")?.as_str()?.to_owned(),
        document_type: payload.get("document_type")?.as_str()?.to_owned(),
        page_number: payload
            .get("page_number")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        chunk_index: payload
            .get("chunk_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
    })
}

fn upstream(error: reqwest::Error) -> AppError {
    AppError::Upstream(error.to_string())
}
