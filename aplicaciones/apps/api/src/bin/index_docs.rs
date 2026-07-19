use anyhow::Context;
use spain_helper_api::{config, db, rag, state::AppState, telemetry};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = config::Config::from_env()?;
    telemetry::init(&config.log_level)?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&config.database_url)
        .await
        .context("connect to PostgreSQL")?;
    db::migrations::run(&pool).await?;
    let state = AppState::new(config, pool, "index-docs".to_owned())?;
    let report = rag::indexer::reindex(&state).await?;
    println!("indexed {} files, {} chunks", report.files, report.chunks);
    Ok(())
}
