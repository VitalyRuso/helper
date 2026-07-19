use shared_contracts::{AssistantPolicy, AssistantVersionStatus};

pub fn create_policy(
    assistant_profile_id: &str,
    title: &str,
    retrieval_top_k: i32,
    require_sources: bool,
    allow_llm_without_sources: bool,
    allowed_collection: &str,
) -> AssistantPolicy {
    AssistantPolicy {
        id: title.to_lowercase().replace(' ', "-"),
        assistant_profile_id: assistant_profile_id.to_owned(),
        title: title.to_owned(),
        retrieval_top_k: retrieval_top_k.max(1),
        require_sources,
        allow_llm_without_sources,
        allowed_collection: allowed_collection.to_owned(),
        status: AssistantVersionStatus::Draft,
    }
}
