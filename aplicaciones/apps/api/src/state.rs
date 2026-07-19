use crate::{config::Config, rag::embeddings::EmbeddingService};
use anyhow::Result;
use reqwest::Client;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub http: Client,
    pub admin_token: Arc<String>,
    pub embeddings: Arc<EmbeddingService>,
}

impl AppState {
    pub fn new(config: Config, db: PgPool, admin_token: String) -> Result<Self> {
        let embeddings = Arc::new(EmbeddingService::from_config(&config)?);
        Ok(Self {
            config: Arc::new(config),
            db,
            http: Client::new(),
            admin_token: Arc::new(admin_token),
            embeddings,
        })
    }
}
