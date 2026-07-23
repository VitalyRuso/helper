use anyhow::{Context, Result};
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub app_name: String,
    pub app_env: String,
    pub api_host: String,
    pub api_port: u16,
    pub database_url: String,
    pub qdrant_url: String,
    pub qdrant_collection: String,
    pub docs_dir: String,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub top_k: usize,
    pub llm_provider: String,
    pub ollama_base_url: String,
    pub ollama_model: String,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_fallback: String,
    pub access_keys: Vec<String>,
    pub guest_question_limit: i32,
    pub admin_username: String,
    pub admin_password: String,
    pub log_level: String,
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            app_name: get("APP_NAME", "Spain Helper AI"),
            app_env: get("APP_ENV", "development"),
            api_host: get("API_HOST", "0.0.0.0"),
            api_port: get("API_PORT", "8000")
                .parse()
                .context("API_PORT must be a number")?,
            database_url: env::var("DATABASE_URL").context("DATABASE_URL is required")?,
            qdrant_url: get("QDRANT_URL", "http://localhost:6333"),
            qdrant_collection: get("QDRANT_COLLECTION", "spain_helper_main"),
            docs_dir: get("DOCS_DIR", "./docs"),
            chunk_size: get("CHUNK_SIZE", "1200")
                .parse()
                .context("CHUNK_SIZE must be a number")?,
            chunk_overlap: get("CHUNK_OVERLAP", "200")
                .parse()
                .context("CHUNK_OVERLAP must be a number")?,
            top_k: get("TOP_K", "5")
                .parse()
                .context("TOP_K must be a number")?,
            llm_provider: get("LLM_PROVIDER", "ollama"),
            ollama_base_url: get("OLLAMA_BASE_URL", "http://localhost:11434"),
            ollama_model: get("OLLAMA_MODEL", "llama3.1"),
            openai_api_key: env::var("OPENAI_API_KEY").ok().filter(|v| !v.is_empty()),
            openai_model: get("OPENAI_MODEL", "gpt-4o-mini"),
            embedding_provider: get("EMBEDDING_PROVIDER", "fastembed"),
            embedding_model: get("EMBEDDING_MODEL", "intfloat/multilingual-e5-small"),
            embedding_fallback: get("EMBEDDING_FALLBACK", "disabled"),
            access_keys: get("ACCESS_KEYS", "")
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            guest_question_limit: get("GUEST_QUESTION_LIMIT", "3")
                .parse()
                .context("GUEST_QUESTION_LIMIT must be a number")?,
            admin_username: get("ADMIN_USERNAME", "admin"),
            admin_password: get("ADMIN_PASSWORD", "change-me"),
            log_level: get("LOG_LEVEL", "info"),
            cors_origins: get(
                "CORS_ORIGINS",
                "http://localhost:3000,http://127.0.0.1:3000",
            )
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        })
    }
}

fn get(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
