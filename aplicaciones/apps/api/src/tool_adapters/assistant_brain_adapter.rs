use assistant_brain_tool::{reject_direct_activation, AssistantRuntimeConfig};
use sqlx::{PgPool, Row};

pub fn reject_direct_prompt_or_policy_activation() -> anyhow::Result<()> {
    reject_direct_activation()?;
    Ok(())
}

pub async fn active_runtime_config(
    pool: &PgPool,
    fallback_top_k: i32,
    fallback_collection: &str,
) -> Result<Option<AssistantRuntimeConfig>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
          p.slug,
          prompt.system_prompt,
          prompt.answer_format,
          COALESCE(policy.retrieval_top_k, $1) AS retrieval_top_k,
          COALESCE(policy.require_sources, true) AS require_sources,
          COALESCE(policy.allow_llm_without_sources, false) AS allow_llm_without_sources,
          COALESCE(policy.allowed_collection, $2) AS allowed_collection
        FROM assistant_profiles p
        JOIN assistant_prompt_versions prompt ON prompt.id = p.active_prompt_version_id
        LEFT JOIN assistant_policies policy ON policy.id = p.active_policy_version_id
        WHERE p.is_active = true
        ORDER BY p.created_at ASC
        LIMIT 1
        "#,
    )
    .bind(fallback_top_k)
    .bind(fallback_collection)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| AssistantRuntimeConfig {
        profile_slug: row.get("slug"),
        system_prompt: row.get("system_prompt"),
        answer_format: row.get("answer_format"),
        retrieval_top_k: row.get("retrieval_top_k"),
        require_sources: row.get("require_sources"),
        allow_llm_without_sources: row.get("allow_llm_without_sources"),
        allowed_collection: row.get("allowed_collection"),
    }))
}
