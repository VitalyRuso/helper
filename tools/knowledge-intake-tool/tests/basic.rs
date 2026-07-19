use knowledge_intake_tool::{
    analyze_source, build_candidate, detect_category, extract_facts, CandidateType, FactType,
    KnowledgeSourceInput, ReviewStatus, SourceType, TrustLevel, DRAFT_WARNING_RU,
};

fn input(text: &str) -> KnowledgeSourceInput {
    KnowledgeSourceInput {
        title: "NIE cita previa".to_owned(),
        source_type: SourceType::PastedText,
        original_path: None,
        source_url: None,
        raw_text: Some(text.to_owned()),
        trust_level: TrustLevel::Official,
    }
}

#[test]
fn analyzes_pasted_text() {
    let output = analyze_source(input(
        "Para NIE debe presentar pasaporte. El plazo es 10 dias.",
    ))
    .unwrap();
    assert!(!output.facts.is_empty());
}

#[test]
fn extracts_document_facts() {
    let facts = extract_facts("Debe presentar pasaporte y certificado.");
    assert!(facts
        .iter()
        .any(|fact| fact.fact_type == FactType::Document));
}

#[test]
fn detects_category() {
    assert_eq!(
        detect_category("Solicitud de cita previa NIE").as_deref(),
        Some("cita-previa")
    );
}

#[test]
fn creates_draft_candidate() {
    let facts = extract_facts("Debe presentar pasaporte.");
    let candidate = build_candidate(&input("Debe presentar pasaporte."), &facts);
    assert_eq!(candidate.candidate_type, CandidateType::Article);
    assert_eq!(candidate.status, ReviewStatus::Draft);
}

#[test]
fn candidate_includes_draft_warning() {
    let output = analyze_source(input("Debe presentar pasaporte.")).unwrap();
    assert!(output.candidate.body_ru_markdown.contains(DRAFT_WARNING_RU));
}

#[test]
fn output_json_is_stable() {
    let output = analyze_source(input("Debe presentar pasaporte.")).unwrap();
    let json = serde_json::to_string(&output).unwrap();
    assert!(json.contains("\"status\":\"draft\""));
}
