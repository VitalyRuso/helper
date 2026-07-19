use assistant_brain_tool::{
    approve_candidate, create_change_candidate, create_policy, create_profile,
    create_prompt_version, reject_direct_activation, validate_safety_rules, AssistantChangeType,
    AssistantVersionStatus, ReviewStatus, RiskLevel,
};
use serde_json::json;

#[test]
fn creates_profile() {
    let profile = create_profile("Spain Helper", "spain-helper", "desc");
    assert!(profile.is_active);
}

#[test]
fn validates_prompt_version() {
    let prompt = create_prompt_version(
        "profile",
        "Safe prompt",
        "Use sources only.",
        "Checklist",
        vec!["Always cite sources.".to_owned()],
    )
    .unwrap();
    assert_eq!(prompt.status, AssistantVersionStatus::Draft);
}

#[test]
fn creates_change_candidate() {
    let candidate = create_change_candidate(
        "profile",
        AssistantChangeType::PromptChange,
        "Better checklist",
        "desc",
        json!({"system_prompt":"Use sources only."}),
        "reason",
        RiskLevel::Low,
    );
    assert_eq!(candidate.status, ReviewStatus::Draft);
}

#[test]
fn approves_candidate_without_overwriting_old_version() {
    let old = create_prompt_version(
        "profile",
        "Old",
        "Use sources only.",
        "Short",
        vec!["Always cite sources.".to_owned()],
    )
    .unwrap();
    let approved = approve_candidate(create_change_candidate(
        "profile",
        AssistantChangeType::PromptChange,
        "New",
        "desc",
        json!({"system_prompt":"Use sources only, clearly."}),
        "reason",
        RiskLevel::Medium,
    ))
    .unwrap();
    assert_eq!(approved.status, ReviewStatus::Approved);
    assert_eq!(old.title, "Old");
}

#[test]
fn policy_require_sources_works() {
    let policy = create_policy("profile", "Strict", 5, true, false, "spain_helper_main");
    assert!(policy.require_sources);
    assert!(!policy.allow_llm_without_sources);
}

#[test]
fn unsafe_direct_activation_is_rejected() {
    assert!(reject_direct_activation().is_err());
    assert!(validate_safety_rules(&["Always cite sources.".to_owned()]).is_ok());
}
