use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LegalSource {
    pub id: Uuid,
    pub source_key: String,
    pub title: String,
    pub authority: String,
    pub jurisdiction: String,
    pub source_type: String,
    pub base_url: Option<String>,
    pub acquisition_method: String,
    pub trust_level: String,
    pub language: String,
    pub enabled: bool,
    pub terms_or_reuse_notes: String,
    pub parser_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLegalSource {
    pub source_key: String,
    pub title: String,
    pub authority: String,
    pub jurisdiction: String,
    pub source_type: String,
    pub base_url: Option<String>,
    pub acquisition_method: String,
    pub trust_level: String,
    pub language: String,
    pub terms_or_reuse_notes: String,
    pub parser_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LegalDocument {
    pub id: Uuid,
    pub source_id: Uuid,
    pub official_id: Option<String>,
    pub eli_id: Option<String>,
    pub title: String,
    pub document_type: String,
    pub legal_area: String,
    pub procedure_key: Option<String>,
    pub source_url: Option<String>,
    pub status: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentVersion {
    pub id: Uuid,
    pub document_id: Uuid,
    pub version_label: String,
    pub publication_date: Option<NaiveDate>,
    pub effective_date: Option<NaiveDate>,
    pub version_date: Option<NaiveDate>,
    pub retrieved_at: DateTime<Utc>,
    pub source_url: Option<String>,
    pub raw_content_path: Option<String>,
    pub normalized_text: String,
    pub content_hash: String,
    pub parser_version: String,
    pub legal_status: String,
    pub is_current: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentSection {
    pub id: Uuid,
    pub version_id: Uuid,
    pub stable_section_key: String,
    pub section_type: String,
    pub section_number: Option<String>,
    pub title: String,
    pub text_content: String,
    pub text_hash: String,
    pub order_index: i32,
    pub parent_section_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentDiff {
    pub id: Uuid,
    pub document_id: Uuid,
    pub from_version_id: Option<Uuid>,
    pub to_version_id: Uuid,
    pub diff_type: String,
    pub summary: String,
    pub diff_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LegalChange {
    pub id: Uuid,
    pub document_diff_id: Uuid,
    pub document_id: Uuid,
    pub change_type: String,
    pub priority: String,
    pub affected_sections: Value,
    pub detected_summary: String,
    pub requires_analysis: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReviewTask {
    pub id: Uuid,
    pub legal_change_id: Option<Uuid>,
    pub document_id: Option<Uuid>,
    pub task_type: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub ai_summary: String,
    pub reviewer_note: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
