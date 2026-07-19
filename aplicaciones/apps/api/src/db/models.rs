use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: Uuid,
    pub title_ru: String,
    pub slug: String,
    pub description_ru: String,
    pub icon: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Article {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub title_ru: String,
    pub slug: String,
    pub summary_ru: String,
    pub body_ru_markdown: String,
    pub tags: Vec<String>,
    pub source_references: Value,
    pub legal_risk_level: String,
    pub is_published: bool,
    pub include_in_ai: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Guide {
    pub id: Uuid,
    pub title_ru: String,
    pub slug: String,
    pub summary_ru: String,
    pub target_audience: String,
    pub required_documents: Value,
    pub steps: Value,
    pub deadlines: Value,
    pub fees: Value,
    pub where_to_submit: String,
    pub common_mistakes: Value,
    pub risks: Value,
    pub official_sources: Value,
    pub related_article_ids: Vec<Uuid>,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
