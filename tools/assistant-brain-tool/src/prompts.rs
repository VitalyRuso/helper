use crate::{safety::validate_safety_rules, AssistantBrainError, AssistantBrainResult};
use shared_contracts::{AssistantPromptVersion, AssistantVersionStatus};

pub fn create_prompt_version(
    assistant_profile_id: &str,
    title: &str,
    system_prompt: &str,
    answer_format: &str,
    safety_rules: Vec<String>,
) -> AssistantBrainResult<AssistantPromptVersion> {
    if system_prompt.trim().is_empty() {
        return Err(AssistantBrainError::EmptyPrompt);
    }
    validate_safety_rules(&safety_rules)?;
    Ok(AssistantPromptVersion {
        id: title.to_lowercase().replace(' ', "-"),
        assistant_profile_id: assistant_profile_id.to_owned(),
        title: title.to_owned(),
        system_prompt: system_prompt.to_owned(),
        answer_format: answer_format.to_owned(),
        safety_rules,
        status: AssistantVersionStatus::Draft,
    })
}

pub fn reject_direct_activation() -> AssistantBrainResult<()> {
    Err(AssistantBrainError::DirectActivationRejected)
}
