use crate::{
    error::AppResult,
    rag::{citations::SourceChunk, llm, prompts, qdrant_store},
    state::AppState,
    tool_adapters::assistant_brain_adapter,
};
use serde::{Deserialize, Serialize};

pub type Citation = SourceChunk;

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentAnswer {
    pub answer: String,
    pub sources: Vec<Citation>,
}

pub async fn answer(
    state: &AppState,
    question: &str,
    page_context: Option<&str>,
) -> AppResult<AgentAnswer> {
    qdrant_store::ensure_collection(state).await?;
    if qdrant_store::count_points(state)
        .await
        .map_err(|error| crate::error::AppError::Upstream(error.to_string()))?
        == 0
    {
        return Ok(AgentAnswer {
            answer: "База документов пока не проиндексирована. Добавьте документы в ./docs и запустите индексацию.".to_owned(),
            sources: vec![],
        });
    }

    let chunks = qdrant_store::search(state, question).await?;
    if chunks.is_empty() {
        return Ok(insufficient(vec![]));
    }

    let relevant: Vec<_> = chunks
        .into_iter()
        .filter(|chunk| chunk.score > 0.05)
        .collect();
    if relevant.is_empty() {
        return Ok(insufficient(vec![]));
    }

    let context = relevant
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            format!(
                "[{}] {} chunk {}\n{}",
                i + 1,
                chunk.file_name,
                chunk.chunk_index,
                chunk.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let runtime = assistant_brain_adapter::active_runtime_config(
        &state.db,
        state.config.top_k as i32,
        &state.config.qdrant_collection,
    )
    .await?;
    let system_prompt = runtime
        .as_ref()
        .map(|config| config.system_prompt.as_str())
        .unwrap_or(prompts::SYSTEM_PROMPT);

    let prompt = format!(
        "{}\n\nContext:\n{}\n\nPage context:\n{}\n\nUser question:\n{}\n\nAnswer only from context.",
        system_prompt,
        context,
        page_context.unwrap_or(""),
        question
    );
    let answer = llm::complete(state, &prompt).await?;
    Ok(AgentAnswer {
        answer,
        sources: relevant,
    })
}

fn insufficient(sources: Vec<Citation>) -> AgentAnswer {
    AgentAnswer {
        answer: format!(
            "Короткий ответ: в базе недостаточно данных.\n\nЧто удалось найти\n{}\n\nЧего не хватает\nПодтвержденных фрагментов из проиндексированных официальных документов.\n\nГде проверить официально\nПроверьте сайт Extranjería, соответствующего органа или официальный источник процедуры.",
            prompts::INSUFFICIENT_RU
        ),
        sources,
    }
}
