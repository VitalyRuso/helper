use shared_contracts::{
    AssistantPolicy, AssistantProfile, AssistantPromptVersion, AssistantRuntimeConfig,
    AssistantVersionStatus,
};

pub fn create_profile(name: &str, slug: &str, description: &str) -> AssistantProfile {
    AssistantProfile {
        id: slug.to_owned(),
        name: name.to_owned(),
        slug: slug.to_owned(),
        description: description.to_owned(),
        active_prompt_version_id: None,
        active_policy_version_id: None,
        is_active: true,
    }
}

pub fn build_runtime_config(
    profile: &AssistantProfile,
    prompt: &AssistantPromptVersion,
    policy: &AssistantPolicy,
) -> Option<AssistantRuntimeConfig> {
    if !profile.is_active
        || prompt.status != AssistantVersionStatus::Active
        || policy.status != AssistantVersionStatus::Active
    {
        return None;
    }

    Some(AssistantRuntimeConfig {
        profile_slug: profile.slug.clone(),
        system_prompt: prompt.system_prompt.clone(),
        answer_format: prompt.answer_format.clone(),
        retrieval_top_k: policy.retrieval_top_k,
        require_sources: policy.require_sources,
        allow_llm_without_sources: policy.allow_llm_without_sources,
        allowed_collection: policy.allowed_collection.clone(),
    })
}
