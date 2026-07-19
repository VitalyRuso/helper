use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use super::{
    diff::{diff_sections, SectionSnapshot, VersionDiffResult},
    models::ReviewTask,
    repository,
};

pub const FIXTURE_V1: &str = r#"
Artículo 1. Objeto.
Esta instrucción regula una prueba local para procedimientos de extranjería.

Artículo 2. Documentación.
La persona interesada deberá aportar pasaporte completo y certificado de empadronamiento.
"#;

pub const FIXTURE_V2: &str = r#"
Artículo 1. Objeto.
Esta instrucción regula una prueba local para procedimientos de extranjería.

Artículo 2. Documentación.
La persona interesada deberá aportar pasaporte completo, certificado de empadronamiento y justificante de medios económicos.

Artículo 3. Revisión humana.
El análisis automatizado deberá ser revisado por un Legal Reviewer antes de publicarse.
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureIngestionResult {
    pub source_id: String,
    pub document_id: String,
    pub version_1_id: String,
    pub version_2_id: String,
    pub diff: VersionDiffResult,
    pub review_task: ReviewTask,
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn simple_fixture_sections(text: &str) -> Vec<SectionSnapshot> {
    let mut sections = Vec::new();
    let mut current_title = String::new();
    let mut current_body = String::new();
    let mut current_key = String::new();
    let mut current_number: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.to_lowercase().starts_with("artículo ") {
            if !current_key.is_empty() {
                let full_text = format!("{}\n{}", current_title, current_body.trim());
                sections.push(SectionSnapshot {
                    stable_section_key: current_key.clone(),
                    section_type: "article".to_string(),
                    section_number: current_number.clone(),
                    title: current_title.clone(),
                    text_content: full_text.clone(),
                    text_hash: sha256_hex(&full_text),
                });
            }

            current_title = trimmed.to_string();
            current_body.clear();

            let number = trimmed
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown")
                .trim_end_matches('.')
                .to_string();

            current_key = format!("article-{number}");
            current_number = Some(number);
        } else {
            if !current_body.is_empty() {
                current_body.push('\n');
            }
            current_body.push_str(trimmed);
        }
    }

    if !current_key.is_empty() {
        let full_text = format!("{}\n{}", current_title, current_body.trim());
        sections.push(SectionSnapshot {
            stable_section_key: current_key,
            section_type: "article".to_string(),
            section_number: current_number,
            title: current_title,
            text_content: full_text.clone(),
            text_hash: sha256_hex(&full_text),
        });
    }

    sections
}

pub async fn run_fixture_ingestion(pool: &PgPool) -> Result<FixtureIngestionResult, sqlx::Error> {
    let source = repository::upsert_fixture_source(pool).await?;
    let document = repository::upsert_fixture_document(pool, source.id).await?;

    let v1_hash = sha256_hex(FIXTURE_V1);
    let v2_hash = sha256_hex(FIXTURE_V2);

    let version_1 = repository::insert_document_version(
        pool,
        document.id,
        "fixture-v1",
        FIXTURE_V1,
        &v1_hash,
        false,
    )
    .await?;

    let version_2 = repository::insert_document_version(
        pool,
        document.id,
        "fixture-v2",
        FIXTURE_V2,
        &v2_hash,
        true,
    )
    .await?;

    let sections_v1 = simple_fixture_sections(FIXTURE_V1);
    let sections_v2 = simple_fixture_sections(FIXTURE_V2);

    for (index, section) in sections_v1.iter().enumerate() {
        repository::insert_document_section(
            pool,
            version_1.id,
            &section.stable_section_key,
            &section.section_type,
            section.section_number.as_deref(),
            &section.title,
            &section.text_content,
            &section.text_hash,
            index as i32,
        )
        .await?;
    }

    for (index, section) in sections_v2.iter().enumerate() {
        repository::insert_document_section(
            pool,
            version_2.id,
            &section.stable_section_key,
            &section.section_type,
            section.section_number.as_deref(),
            &section.title,
            &section.text_content,
            &section.text_hash,
            index as i32,
        )
        .await?;
    }

    let diff = diff_sections(&sections_v1, &sections_v2);
    let diff_json = json!(diff);

    let document_diff = repository::insert_document_diff(
        pool,
        document.id,
        Some(version_1.id),
        version_2.id,
        "version_diff",
        "Fixture v1/v2 comparison detected changed immigration-procedure content.",
        diff_json,
    )
    .await?;

    let affected_sections = json!(diff.changes);

    let legal_change = repository::insert_legal_change(
        pool,
        document_diff.id,
        document.id,
        "modified_wording",
        "medium",
        affected_sections,
        "Fixture change detected: documentation requirements changed and human review article was added.",
    )
    .await?;

    let review_task = repository::insert_review_task(
        pool,
        legal_change.id,
        document.id,
        "Review fixture immigration instruction change",
        "medium",
        "Synthetic fixture detected a changed documentation requirement and a new human-review rule.",
    )
    .await?;

    Ok(FixtureIngestionResult {
        source_id: source.id.to_string(),
        document_id: document.id.to_string(),
        version_1_id: version_1.id.to_string(),
        version_2_id: version_2.id.to_string(),
        diff,
        review_task,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_deterministic_fixture_sections() {
        let first = simple_fixture_sections(FIXTURE_V1);
        let second = simple_fixture_sections(FIXTURE_V1);

        assert_eq!(first.len(), 2);
        assert_eq!(first[0].text_hash, second[0].text_hash);
    }

    #[test]
    fn fixture_diff_detects_added_and_modified_articles() {
        let old_sections = simple_fixture_sections(FIXTURE_V1);
        let new_sections = simple_fixture_sections(FIXTURE_V2);

        let diff = diff_sections(&old_sections, &new_sections);

        assert_eq!(diff.added, 1);
        assert_eq!(diff.modified, 1);
        assert_eq!(diff.removed, 0);
        assert_eq!(diff.unchanged, 1);
    }
}
