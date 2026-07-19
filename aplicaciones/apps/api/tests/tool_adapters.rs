use shared_contracts::{KnowledgeSourceInput, SourceType, TrustLevel};
use spain_helper_api::{
    rag::prompts,
    tool_adapters::{assistant_brain_adapter, knowledge_intake_adapter},
};

#[test]
fn app_can_call_knowledge_intake_tool_adapter() {
    let output = knowledge_intake_adapter::analyze(KnowledgeSourceInput {
        title: "NIE".to_owned(),
        source_type: SourceType::PastedText,
        original_path: None,
        source_url: None,
        raw_text: Some("Debe presentar pasaporte para NIE.".to_owned()),
        trust_level: TrustLevel::Official,
    })
    .unwrap();

    assert_eq!(
        output.candidate.status,
        shared_contracts::ReviewStatus::Draft
    );
}

#[test]
fn app_can_call_assistant_brain_tool_adapter() {
    let error = assistant_brain_adapter::reject_direct_prompt_or_policy_activation().unwrap_err();
    assert!(error.to_string().contains("direct activation"));
}

#[test]
fn existing_chat_prompt_fallback_still_exists() {
    assert!(prompts::SYSTEM_PROMPT.contains("retrieved context"));
}
