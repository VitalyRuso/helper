use crate::{ReviewStatus, RiskLevel};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssistantVersionStatus {
    Draft,
    Active,
    Archived,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssistantChangeType {
    PromptChange,
    PolicyChange,
    ToolChange,
    AnswerFormatChange,
    DataSourceChange,
    ArchitectureNote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantProfile {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub active_prompt_version_id: Option<String>,
    pub active_policy_version_id: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantPromptVersion {
    pub id: String,
    pub assistant_profile_id: String,
    pub title: String,
    pub system_prompt: String,
    pub answer_format: String,
    pub safety_rules: Vec<String>,
    pub status: AssistantVersionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantPolicy {
    pub id: String,
    pub assistant_profile_id: String,
    pub title: String,
    pub retrieval_top_k: i32,
    pub require_sources: bool,
    pub allow_llm_without_sources: bool,
    pub allowed_collection: String,
    pub status: AssistantVersionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantChangeCandidate {
    pub id: String,
    pub assistant_profile_id: String,
    pub candidate_type: AssistantChangeType,
    pub title: String,
    pub description: String,
    pub proposed_payload_json: Value,
    pub reason: String,
    pub risk_level: RiskLevel,
    pub status: ReviewStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantTestRun {
    pub id: String,
    pub assistant_profile_id: String,
    pub candidate_id: Option<String>,
    pub test_question: String,
    pub response_preview: String,
    pub sources_json: Value,
    pub passed: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantRuntimeConfig {
    pub profile_slug: String,
    pub system_prompt: String,
    pub answer_format: String,
    pub retrieval_top_k: i32,
    pub require_sources: bool,
    pub allow_llm_without_sources: bool,
    pub allowed_collection: String,
}
