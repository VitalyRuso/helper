use crate::{
    error::AppResult,
    legal::answer::{self as legal_answer, LegalMetadata, PreparedLegalAnswer, QuestionRoute},
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
    pub legal: Option<LegalMetadata>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerationPlan {
    pub prompt: String,
    pub sources: Vec<Citation>,
    pub legal: Option<LegalMetadata>,
}

#[derive(Debug)]
pub enum PreparedAnswer {
    Immediate(AgentAnswer),
    Generate(GenerationPlan),
}

pub async fn answer(
    state: &AppState,
    question: &str,
    page_context: Option<&str>,
) -> AppResult<AgentAnswer> {
    match prepare(state, question, page_context).await? {
        PreparedAnswer::Immediate(answer) => Ok(answer),
        PreparedAnswer::Generate(plan) => {
            let generated = llm::complete(state, &plan.prompt).await?;
            finish(plan, generated)
        }
    }
}

pub async fn prepare(
    state: &AppState,
    question: &str,
    page_context: Option<&str>,
) -> AppResult<PreparedAnswer> {
    if let QuestionRoute::Legal(route) = legal_answer::route_question(question) {
        return match legal_answer::prepare(&state.db, question, &route).await? {
            PreparedLegalAnswer::Immediate(legal) => Ok(PreparedAnswer::Immediate(AgentAnswer {
                answer: legal.answer,
                sources: vec![],
                legal: Some(legal.metadata),
            })),
            PreparedLegalAnswer::Generate { prompt, metadata } => {
                Ok(PreparedAnswer::Generate(GenerationPlan {
                    prompt,
                    sources: vec![],
                    legal: Some(metadata),
                }))
            }
        };
    }

    qdrant_store::ensure_collection(state).await?;
    if qdrant_store::count_points(state)
        .await
        .map_err(|error| crate::error::AppError::Upstream(error.to_string()))?
        == 0
    {
        return Ok(PreparedAnswer::Immediate(AgentAnswer {
            answer: "База документов пока не проиндексирована. Добавьте документы в ./docs и запустите индексацию.".to_owned(),
            sources: vec![],
            legal: None,
        }));
    }

    let chunks = qdrant_store::search(state, question).await?;
    if chunks.is_empty() {
        return Ok(PreparedAnswer::Immediate(insufficient(vec![])));
    }

    let relevant: Vec<_> = chunks
        .into_iter()
        .filter(|chunk| chunk.score > 0.05)
        .collect();
    if relevant.is_empty() {
        return Ok(PreparedAnswer::Immediate(insufficient(vec![])));
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
    Ok(PreparedAnswer::Generate(GenerationPlan {
        prompt,
        sources: relevant,
        legal: None,
    }))
}

pub fn finish(plan: GenerationPlan, generated: String) -> AppResult<AgentAnswer> {
    if let Some(metadata) = plan.legal {
        let legal = legal_answer::finish(generated, metadata)?;
        Ok(AgentAnswer {
            answer: legal.answer,
            sources: vec![],
            legal: Some(legal.metadata),
        })
    } else {
        Ok(AgentAnswer {
            answer: generated,
            sources: plan.sources,
            legal: None,
        })
    }
}

fn insufficient(sources: Vec<Citation>) -> AgentAnswer {
    AgentAnswer {
        answer: format!(
            "Короткий ответ: в базе недостаточно данных.\n\nЧто удалось найти\n{}\n\nЧего не хватает\nПодтвержденных фрагментов из проиндексированных официальных документов.\n\nГде проверить официально\nПроверьте сайт Extranjería, соответствующего органа или официальный источник процедуры.",
            prompts::INSUFFICIENT_RU
        ),
        sources,
        legal: None,
    }
}
