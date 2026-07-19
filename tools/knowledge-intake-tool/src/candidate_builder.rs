use crate::detect_category;
use shared_contracts::{
    CandidateType, ContentCandidate, ExtractedFact, FactType, KnowledgeSourceInput, ReviewStatus,
    RiskLevel, DRAFT_WARNING_RU,
};

pub fn build_candidate(input: &KnowledgeSourceInput, facts: &[ExtractedFact]) -> ContentCandidate {
    let source_text = input.raw_text.as_deref().unwrap_or(&input.title);
    let risk_level = if facts.iter().any(|fact| fact.fact_type == FactType::Warning) {
        RiskLevel::High
    } else if facts.is_empty() {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };

    ContentCandidate {
        candidate_type: CandidateType::Article,
        title_ru: input.title.clone(),
        summary_ru: facts
            .first()
            .map(|fact| fact.text_ru.clone())
            .unwrap_or_else(|| "Черновик для проверки редактором.".to_owned()),
        body_ru_markdown: body(input, facts),
        category_slug: detect_category(source_text),
        risk_level,
        status: ReviewStatus::Draft,
    }
}

fn body(input: &KnowledgeSourceInput, facts: &[ExtractedFact]) -> String {
    let documents = section(facts, FactType::Document);
    let deadlines = section(facts, FactType::Deadline);
    let warnings = section(facts, FactType::Warning);
    let all = if facts.is_empty() {
        "- Подтвержденные факты не найдены автоматически.".to_owned()
    } else {
        facts
            .iter()
            .map(|fact| format!("- {}", fact.text_ru))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let source = input
        .source_url
        .as_deref()
        .or(input.original_path.as_deref())
        .unwrap_or("Источник указан во входных данных.");

    format!(
        "# Кратко\n\n{warning}\n\n# Что найдено в источнике\n\n{all}\n\n# Возможные документы / требования\n\n{documents}\n\n# Сроки / важные даты\n\n{deadlines}\n\n# Риски и что проверить\n\n{warnings}\n\n# Источник\n\n{source}\n",
        warning = DRAFT_WARNING_RU,
    )
}

fn section(facts: &[ExtractedFact], fact_type: FactType) -> String {
    let lines = facts
        .iter()
        .filter(|fact| fact.fact_type == fact_type)
        .map(|fact| format!("- {}", fact.text_ru))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        "- Не найдено автоматически.".to_owned()
    } else {
        lines.join("\n")
    }
}
