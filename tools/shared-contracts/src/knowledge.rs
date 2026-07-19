use crate::{ReviewStatus, RiskLevel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    File,
    Url,
    PastedText,
    ManualNote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Official,
    SemiOfficial,
    UserProvided,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactType {
    Requirement,
    Deadline,
    Fee,
    Document,
    ProcedureStep,
    Warning,
    Definition,
    Contact,
    SourceReference,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateType {
    Article,
    Guide,
    Checklist,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeSourceInput {
    pub title: String,
    pub source_type: SourceType,
    pub original_path: Option<String>,
    pub source_url: Option<String>,
    pub raw_text: Option<String>,
    pub trust_level: TrustLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedFact {
    pub fact_type: FactType,
    pub title_ru: String,
    pub text_ru: String,
    pub original_text: String,
    pub confidence: f32,
    pub source_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentCandidate {
    pub candidate_type: CandidateType,
    pub title_ru: String,
    pub summary_ru: String,
    pub body_ru_markdown: String,
    pub category_slug: Option<String>,
    pub risk_level: RiskLevel,
    pub status: ReviewStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeAnalysisOutput {
    pub source: KnowledgeSourceInput,
    pub facts: Vec<ExtractedFact>,
    pub candidate: ContentCandidate,
}
