use anyhow::Context;
use axum::{http::HeaderValue, Router};
use spain_helper_api::{config, db, routes, security, seeds, state::AppState, telemetry};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = config::Config::from_env()?;
    telemetry::init(&config.log_level)?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .context("connect to PostgreSQL")?;

    db::migrations::run(&pool).await?;
    seeds::seed_content::seed_defaults(&pool).await?;
    security::seed_access_key_hashes(&pool, &config.access_keys).await?;

    let origins = config
        .cors_origins
        .iter()
        .map(|origin| origin.parse::<HeaderValue>())
        .collect::<Result<Vec<_>, _>>()
        .context("CORS_ORIGINS contains an invalid origin")?;
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers(Any);
    let state = AppState::new(config.clone(), pool, Uuid::new_v4().to_string())?;
    let app = Router::new()
        .merge(routes::router(state.clone()))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", config.api_host, config.api_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(%addr, "Spain Helper AI API listening");
    axum::serve(listener, app).await?;
    Ok(())
}
