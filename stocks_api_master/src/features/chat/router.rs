//! Chat routes.
//!
//! Mounted under `/api/:commerce_id/chat` in `server.rs`, so they inherit the
//! tenant-pool resolution middleware (`Extension<PgPool>`) and `require_auth`
//! (`Extension<Claims>`). No router-level state is used — the tenant pool is
//! injected per-request by `resolve_tenant_pool`, mirroring `ai_predictions`.

use axum::{
    routing::{get, post},
    Router,
};

use super::handlers;

pub fn chat_routes() -> Router {
    Router::new()
        .route("/sessions", post(handlers::create_session).get(handlers::list_sessions))
        .route("/sessions/:id", get(handlers::get_session).delete(handlers::delete_session))
        .route("/sessions/:id/messages", post(handlers::send_message))
        .route("/sessions/:id/messages/stream", post(handlers::send_message_stream))
        .route("/sessions/:id/confirm-action", post(handlers::confirm_action))
        .route("/sessions/:id/export", get(handlers::export_session))
        .route("/briefing", get(handlers::get_briefing))
        // Admin (platform/commerce) — folded in so everything stays under the
        // tenant path. RAG reindex + provider health operate via the ai-service.
        .route("/providers", get(handlers::admin_provider_health))
        .route("/rag/reindex", post(handlers::admin_rag_reindex))
}
