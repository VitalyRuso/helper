use crate::{build_candidate, extract_facts, intake::source_text, KnowledgeToolResult};
use shared_contracts::{KnowledgeAnalysisOutput, KnowledgeSourceInput};

pub fn analyze_source(input: KnowledgeSourceInput) -> KnowledgeToolResult<KnowledgeAnalysisOutput> {
    let text = source_text(&input)?;
    let facts = extract_facts(&text);
    let candidate = build_candidate(
        &KnowledgeSourceInput {
            raw_text: Some(text),
            ..input.clone()
        },
        &facts,
    );
    Ok(KnowledgeAnalysisOutput {
        source: input,
        facts,
        candidate,
    })
}
