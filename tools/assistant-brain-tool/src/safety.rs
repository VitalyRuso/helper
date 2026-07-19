use crate::{AssistantBrainError, AssistantBrainResult};

pub fn validate_safety_rules(safety_rules: &[String]) -> AssistantBrainResult<()> {
    if safety_rules.is_empty() {
        return Err(AssistantBrainError::UnsafeRules(
            "at least one safety rule is required".to_owned(),
        ));
    }
    let joined = safety_rules.join(" ").to_lowercase();
    if !(joined.contains("source") || joined.contains("источник")) {
        return Err(AssistantBrainError::UnsafeRules(
            "rules must mention sources".to_owned(),
        ));
    }
    if joined.contains("ignore safety") || joined.contains("без ограничений") {
        return Err(AssistantBrainError::UnsafeRules(
            "rules cannot disable safety".to_owned(),
        ));
    }
    Ok(())
}
