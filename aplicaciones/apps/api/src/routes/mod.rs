use crate::state::AppState;
use axum::Router;

pub mod access;
pub mod admin;
pub mod admin_assistant;
pub mod admin_knowledge;
pub mod admin_legal;
pub mod articles;
pub mod categories;
pub mod chat;
pub mod documents;
pub mod guides;
pub mod health;
pub mod rag;
pub mod search;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .nest("/api/admin/legal", admin_legal::router())
        .nest("/api/categories", categories::router())
        .nest("/api/articles", articles::router())
        .nest("/api/guides", guides::router())
        .nest("/api/search", search::router())
        .nest("/api/chat", chat::router())
        .nest("/api/documents", documents::router())
        .nest("/api/access", access::router())
        .nest("/api/rag", rag::router())
        .nest("/api/admin", admin::router())
        .nest("/api/admin/knowledge", admin_knowledge::router())
        .nest("/api/admin/assistant", admin_assistant::router())
        .nest("/api/admin/categories", categories::admin_router())
        .nest("/api/admin/articles", articles::admin_router())
        .nest("/api/admin/guides", guides::admin_router())
        .with_state(state)
}
