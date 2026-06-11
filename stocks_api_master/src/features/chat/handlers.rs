//! Chat HTTP handlers (multi-tenant).
//!
//! Mounted under `/api/:commerce_id/chat`. The tenant `PgPool` is injected by
//! `resolve_tenant_pool` as an `Extension`; `Claims` by `require_auth`. Session
//! ownership is keyed by `claims.email` (master auth is commerce-level). The
//! `commerce_id` + `slug` from the token are forwarded to the ai-service so its
//! tools call tenant-scoped routes (`/api/{commerce_id}/...`).
//!
//! Flow for sending a message:
//! 1. Verify the session belongs to the caller.
//! 2. Persist the user message.
//! 3. Load recent history (capped).
//! 4. Forward to ai-service /chat/turn with the user's JWT + tenant context.
//! 5. Persist the assistant turn (and any pending action).
//! 6. Auto-title the session on the first user message.

use axum::{
    extract::{Extension, Path, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{error_codes, responses::{ErrorResponse, SuccessResponse}, security::Claims};

use super::{ai_client, dto::*, services};

const HISTORY_LIMIT: i32 = 30;
const TITLE_MAX_LEN: usize = 60;

fn tenant_ctx(claims: &Claims) -> (String, String, String) {
    let owner = claims.email.clone();
    let commerce_id = claims.commerce_id.map(|c| c.to_string()).unwrap_or_default();
    let slug = claims.slug.clone().unwrap_or_default();
    (owner, commerce_id, slug)
}

fn extract_jwt(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/chat/sessions",
    tag = "chat",
    request_body = CreateSessionRequest,
    responses(
        (status = 201, description = "Session created", body = inline(SuccessResponse<ChatSession>)),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
)]
pub async fn create_session(
    Extension(pool): Extension<PgPool>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateSessionRequest>,
) -> Response {
    let owner = claims.email.clone();
    let provider = req.provider.unwrap_or_else(|| "auto".to_string());
    match services::create_session(&pool, &owner, req.title.as_deref(), &provider).await {
        Ok(s) => (
            StatusCode::CREATED,
            Json(SuccessResponse::new(s, "Session created".to_string())),
        ).into_response(),
        Err(e) => {
            eprintln!("create_session error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to create session".to_string(),
                )),
            ).into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/chat/sessions",
    tag = "chat",
    params(ListSessionsQuery),
    responses(
        (status = 200, description = "Sessions listed", body = inline(SuccessResponse<Vec<ChatSession>>)),
    ),
)]
pub async fn list_sessions(
    Extension(pool): Extension<PgPool>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<ListSessionsQuery>,
) -> Response {
    let owner = claims.email.clone();
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let offset = q.offset.unwrap_or(0).max(0);
    match services::list_sessions(&pool, &owner, limit, offset).await {
        Ok(list) => (
            StatusCode::OK,
            Json(SuccessResponse::new(list, "Sessions retrieved".to_string())),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to list sessions".to_string(),
            )),
        ).into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/chat/sessions/{id}",
    tag = "chat",
    params(("id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Session + messages", body = inline(SuccessResponse<ChatSessionWithMessages>)),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
)]
pub async fn get_session(
    Extension(pool): Extension<PgPool>,
    Extension(claims): Extension<Claims>,
    Path((_commerce_id, id)): Path<(String, Uuid)>,
) -> Response {
    let owner = claims.email.clone();
    let session = match services::get_session(&pool, id, &owner).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    "Session not found".to_string(),
                )),
            ).into_response()
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to load session".to_string(),
                )),
            ).into_response()
        }
    };
    let messages = services::list_messages(&pool, id, 200).await.unwrap_or_default();
    let payload = ChatSessionWithMessages {
        session_id: session.session_id,
        owner_email: session.owner_email,
        title: session.title,
        provider: session.provider,
        created_at: session.created_at,
        updated_at: session.updated_at,
        messages,
    };
    (
        StatusCode::OK,
        Json(SuccessResponse::new(payload, "Session retrieved".to_string())),
    ).into_response()
}

#[utoipa::path(
    delete,
    path = "/chat/sessions/{id}",
    tag = "chat",
    params(("id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Session deleted"),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
)]
pub async fn delete_session(
    Extension(pool): Extension<PgPool>,
    Extension(claims): Extension<Claims>,
    Path((_commerce_id, id)): Path<(String, Uuid)>,
) -> Response {
    let owner = claims.email.clone();
    match services::delete_session(&pool, id, &owner).await {
        Ok(true) => (
            StatusCode::OK,
            Json(SuccessResponse::new(json!({"deleted": true}), "Session deleted".to_string())),
        ).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                "Session not found".to_string(),
            )),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to delete session".to_string(),
            )),
        ).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Messages — non-streaming
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/chat/sessions/{id}/messages",
    tag = "chat",
    params(("id" = Uuid, Path, description = "Session ID")),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Assistant response", body = inline(SuccessResponse<SendMessageResponse>)),
        (status = 404, description = "Session not found", body = ErrorResponse),
        (status = 502, description = "AI service unavailable", body = ErrorResponse),
    ),
)]
pub async fn send_message(
    Extension(pool): Extension<PgPool>,
    Extension(claims): Extension<Claims>,
    headers: HeaderMap,
    Path((_commerce_id, session_id)): Path<(String, Uuid)>,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    let (owner, commerce_id, slug) = tenant_ctx(&claims);
    let user_jwt = match extract_jwt(&headers) {
        Some(j) => j,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    error_codes::UNAUTHORIZED.to_string(),
                    "Missing bearer token".to_string(),
                )),
            ).into_response()
        }
    };

    // Verify ownership
    let session = match services::get_session(&pool, session_id, &owner).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    "Session not found".to_string(),
                )),
            ).into_response()
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to load session".to_string(),
                )),
            ).into_response()
        }
    };

    let trimmed = req.content.trim().to_string();
    if trimmed.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::VALIDATION_ERROR.to_string(),
                "content is required".to_string(),
            )),
        ).into_response();
    }

    // Persist the user message
    if let Err(e) = services::insert_message(
        &pool, session_id, "user", &trimmed, None, None, None, None, None, None,
    ).await {
        eprintln!("insert user msg: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to persist message".to_string(),
            )),
        ).into_response();
    }

    // Auto-title from first user message
    let auto_title = trimmed
        .chars()
        .take(TITLE_MAX_LEN)
        .collect::<String>();
    let _ = services::update_session_title_if_empty(&pool, session_id, &auto_title).await;

    // Load history (excluding the just-inserted user msg, which we pass separately)
    let mut history = services::list_messages(&pool, session_id, HISTORY_LIMIT).await
        .unwrap_or_default();
    // Drop the most recent if it's the same user msg we just inserted.
    if let Some(last) = history.last() {
        if last.role == "user" && last.content == trimmed {
            history.pop();
        }
    }

    // Build the request to ai-service
    let history_payload: Vec<ai_client::TurnHistoryEntry> = history.iter()
        .map(|m| ai_client::TurnHistoryEntry {
            role: m.role.as_str(),
            content: m.content.as_str(),
            tool_calls: m.tool_calls.as_ref(),
            name: m.tool_name.as_deref(),
            tool_call_id: None,
        })
        .collect();

    let provider_pref = req.provider.as_deref().or(Some(session.provider.as_str()));

    let turn_req = ai_client::TurnRequest {
        user_message: &trimmed,
        history: history_payload,
        user_jwt: &user_jwt,
        user_email: &owner,
        commerce_id: &commerce_id,
        slug: &slug,
        session_id: session_id.to_string(),
        provider: provider_pref,
        proactive_summary: None,
    };

    let turn = match ai_client::run_turn(&turn_req).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("ai-service /chat/turn failed: {e}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    error_codes::INTERNAL_SERVER_ERROR.to_string(),
                    format!("AI service unavailable: {e}"),
                )),
            ).into_response();
        }
    };

    let usage = turn.usage.as_ref();
    let assistant_msg = match services::insert_message(
        &pool,
        session_id,
        "assistant",
        &turn.content,
        Some(&serde_json::Value::Array(turn.tool_calls.clone())),
        None,
        Some(turn.provider_used.as_str()),
        usage.map(|u| u.tokens_in),
        usage.map(|u| u.tokens_out),
        usage.map(|u| u.latency_ms),
    ).await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("insert assistant msg: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to persist assistant message".to_string(),
                )),
            ).into_response();
        }
    };

    let _ = services::touch_session(&pool, session_id).await;

    // If a pending action was raised, persist it.
    let pending_dto = if let Some(pa) = turn.pending_action.as_ref() {
        match services::create_pending_action(
            &pool,
            session_id,
            Some(assistant_msg.message_id),
            &pa.tool_name,
            &pa.tool_args,
        ).await {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("create_pending_action: {e}");
                None
            }
        }
    } else {
        None
    };

    let resp = SendMessageResponse {
        assistant_message: assistant_msg,
        pending_action: pending_dto,
        provider_used: turn.provider_used.clone(),
        intent: turn.intent.clone(),
        citations: turn.citations.iter().map(|c| Citation {
            source_path: c.source_path.clone(),
            heading: c.heading.clone(),
            similarity: c.similarity,
        }).collect(),
        cached: turn.cached,
        shortcut_used: turn.shortcut_used.clone(),
        numbers_verified: turn.numbers_verified,
        suggestions: turn.suggestions.clone(),
        usage: ChatTurnUsage {
            tokens_in: usage.map(|u| u.tokens_in).unwrap_or(0),
            tokens_out: usage.map(|u| u.tokens_out).unwrap_or(0),
            latency_ms: usage.map(|u| u.latency_ms).unwrap_or(0),
        },
    };

    (
        StatusCode::OK,
        Json(SuccessResponse::new(resp, "Assistant response".to_string())),
    ).into_response()
}

// ---------------------------------------------------------------------------
// Streaming (SSE)
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/chat/sessions/{id}/messages/stream",
    tag = "chat",
    params(("id" = Uuid, Path, description = "Session ID")),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Server-Sent Events stream"),
    ),
)]
pub async fn send_message_stream(
    Extension(pool): Extension<PgPool>,
    Extension(claims): Extension<Claims>,
    headers: HeaderMap,
    Path((_commerce_id, session_id)): Path<(String, Uuid)>,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    let (owner, commerce_id, slug) = tenant_ctx(&claims);
    let user_jwt = match extract_jwt(&headers) {
        Some(j) => j,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    error_codes::UNAUTHORIZED.to_string(),
                    "Missing bearer token".to_string(),
                )),
            ).into_response();
        }
    };

    let session = match services::get_session(&pool, session_id, &owner).await {
        Ok(Some(s)) => s,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    "Session not found".to_string(),
                )),
            ).into_response();
        }
    };

    let trimmed = req.content.trim().to_string();
    if trimmed.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::VALIDATION_ERROR.to_string(),
                "content is required".to_string(),
            )),
        ).into_response();
    }

    // Persist the user message and load history (same as the non-streaming path).
    if let Err(e) = services::insert_message(
        &pool, session_id, "user", &trimmed, None, None, None, None, None, None,
    ).await {
        eprintln!("insert user msg: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                error_codes::DATABASE_ERROR.to_string(),
                "Failed to persist message".to_string(),
            )),
        ).into_response();
    }

    let auto_title: String = trimmed.chars().take(TITLE_MAX_LEN).collect();
    let _ = services::update_session_title_if_empty(&pool, session_id, &auto_title).await;

    let mut history = services::list_messages(&pool, session_id, HISTORY_LIMIT).await
        .unwrap_or_default();
    if let Some(last) = history.last() {
        if last.role == "user" && last.content == trimmed {
            history.pop();
        }
    }

    let history_payload: Vec<serde_json::Value> = history.iter().map(|m| {
        let mut obj = serde_json::json!({
            "role": m.role,
            "content": m.content,
        });
        if let Some(tc) = &m.tool_calls {
            obj["tool_calls"] = tc.clone();
        }
        if let Some(n) = &m.tool_name {
            obj["name"] = serde_json::Value::String(n.clone());
        }
        obj
    }).collect();

    let provider_pref = req.provider.clone().unwrap_or_else(|| session.provider.clone());

    let body = serde_json::json!({
        "user_message": trimmed,
        "history": history_payload,
        "user_jwt": user_jwt,
        "user_email": owner,
        "commerce_id": commerce_id,
        "slug": slug,
        "session_id": session_id.to_string(),
        "provider": provider_pref,
    });

    let ai_url = std::env::var("AI_SERVICE_URL")
        .unwrap_or_else(|_| "http://ai-service:8001".to_string());
    let url = format!("{}/chat/turn/stream", ai_url.trim_end_matches('/'));

    let upstream = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(900))
        .build()
        .unwrap()
        .post(&url)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ai-service stream connect failed: {e}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    error_codes::INTERNAL_SERVER_ERROR.to_string(),
                    format!("AI stream unavailable: {e}"),
                )),
            ).into_response();
        }
    };

    if !upstream.status().is_success() {
        let status = upstream.status();
        let body = upstream.text().await.unwrap_or_default();
        return (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new(
                error_codes::INTERNAL_SERVER_ERROR.to_string(),
                format!("AI stream HTTP {}: {}", status, body),
            )),
        ).into_response();
    }

    // Pass-through SSE bytes to the client. Persist the final assistant
    // message after the stream ends by buffering deltas inline.
    use axum::body::Body;
    use futures_util::stream::StreamExt;

    let pool_for_persist = pool.clone();
    let prov_for_persist = provider_pref.clone();
    let stream = async_stream::stream! {
        let mut stream = upstream.bytes_stream();
        let mut buffered_delta = String::new();
        let mut last_provider = prov_for_persist;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        // Best-effort sniff: collect "delta" content for persistence.
                        for line in text.lines() {
                            if let Some(rest) = line.strip_prefix("data: ") {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(rest) {
                                    if let Some(c) = v.get("content").and_then(|x| x.as_str()) {
                                        buffered_delta.push_str(c);
                                    }
                                    if let Some(p) = v.get("provider_used").and_then(|x| x.as_str()) {
                                        last_provider = p.to_string();
                                    }
                                }
                            }
                        }
                    }
                    yield Ok::<_, std::io::Error>(bytes);
                }
                Err(e) => {
                    let err_event = format!("event: error\ndata: {{\"message\":\"{}\"}}\n\n", e);
                    yield Ok(err_event.into_bytes().into());
                    break;
                }
            }
        }
        // Persist final assistant text + touch session.
        if !buffered_delta.is_empty() {
            let _ = services::insert_message(
                &pool_for_persist, session_id, "assistant",
                &buffered_delta, None, None, Some(last_provider.as_str()),
                None, None, None,
            ).await;
            let _ = services::touch_session(&pool_for_persist, session_id).await;
        }
    };

    let body = Body::from_stream(stream);
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::CONTENT_TYPE,
                   "text/event-stream".parse().unwrap());
    headers.insert(axum::http::header::CACHE_CONTROL, "no-cache".parse().unwrap());
    headers.insert("X-Accel-Buffering", "no".parse().unwrap());
    (StatusCode::OK, headers, body).into_response()
}

// ---------------------------------------------------------------------------
// Briefing proactif
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/chat/briefing",
    tag = "chat",
    responses(
        (status = 200, description = "Proactive store briefing"),
        (status = 502, description = "AI service unavailable", body = ErrorResponse),
    ),
)]
pub async fn get_briefing(
    Extension(claims): Extension<Claims>,
    headers: HeaderMap,
) -> Response {
    let (owner, commerce_id, slug) = tenant_ctx(&claims);
    let user_jwt = match extract_jwt(&headers) {
        Some(j) => j,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    error_codes::UNAUTHORIZED.to_string(),
                    "Missing bearer token".to_string(),
                )),
            ).into_response()
        }
    };
    match ai_client::briefing(&user_jwt, &owner, &commerce_id, &slug, "").await {
        Ok(b) => (
            StatusCode::OK,
            Json(SuccessResponse::new(b, "Briefing".to_string())),
        ).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new(
                error_codes::INTERNAL_SERVER_ERROR.to_string(),
                format!("AI service unavailable: {e}"),
            )),
        ).into_response(),
    }
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/chat/sessions/{id}/export",
    tag = "chat",
    params(
        ("id" = Uuid, Path, description = "Session ID"),
        ("format" = Option<String>, Query, description = "markdown (default) or json"),
    ),
    responses(
        (status = 200, description = "Conversation export"),
        (status = 404, description = "Not found", body = ErrorResponse),
    ),
)]
pub async fn export_session(
    Extension(pool): Extension<PgPool>,
    Extension(claims): Extension<Claims>,
    Path((_commerce_id, id)): Path<(String, Uuid)>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let owner = claims.email.clone();
    let session = match services::get_session(&pool, id, &owner).await {
        Ok(Some(s)) => s,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    "Session not found".to_string(),
                )),
            ).into_response();
        }
    };
    let messages = services::list_messages(&pool, id, 1000).await.unwrap_or_default();

    let format = q.get("format").map(|s| s.as_str()).unwrap_or("markdown");
    if format == "json" {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "session": session,
                "messages": messages,
            })),
        ).into_response();
    }

    // Markdown format
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", session.title.as_deref().unwrap_or("Conversation")));
    out.push_str(&format!("_Provider: {} — Created: {}_\n\n---\n\n",
        session.provider, session.created_at.to_rfc3339()));
    for m in &messages {
        let who = match m.role.as_str() {
            "user" => "**Vous**",
            "assistant" => "**Assistant**",
            "tool" => &format!("_Outil: {}_", m.tool_name.as_deref().unwrap_or("?")),
            "system" => "_Système_",
            other => other,
        };
        out.push_str(&format!("{}\n\n{}\n\n", who, m.content));
    }
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::CONTENT_TYPE,
                   "text/markdown; charset=utf-8".parse().unwrap());
    headers.insert(axum::http::header::CONTENT_DISPOSITION,
                   format!("attachment; filename=\"chat-{}.md\"", id).parse().unwrap());
    (StatusCode::OK, headers, out).into_response()
}

// ---------------------------------------------------------------------------
// Confirm action
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/chat/sessions/{id}/confirm-action",
    tag = "chat",
    params(("id" = Uuid, Path, description = "Session ID")),
    request_body = ConfirmActionRequest,
    responses(
        (status = 200, description = "Action resolved", body = inline(SuccessResponse<ConfirmActionResponse>)),
        (status = 404, description = "Action not found", body = ErrorResponse),
    ),
)]
pub async fn confirm_action(
    Extension(pool): Extension<PgPool>,
    Extension(claims): Extension<Claims>,
    headers: HeaderMap,
    Path((_commerce_id, session_id)): Path<(String, Uuid)>,
    Json(req): Json<ConfirmActionRequest>,
) -> Response {
    let (owner, commerce_id, slug) = tenant_ctx(&claims);
    let user_jwt = match extract_jwt(&headers) {
        Some(j) => j,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    error_codes::UNAUTHORIZED.to_string(),
                    "Missing bearer token".to_string(),
                )),
            ).into_response()
        }
    };

    if !matches!(req.decision.as_str(), "confirm" | "cancel") {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                error_codes::VALIDATION_ERROR.to_string(),
                "decision must be 'confirm' or 'cancel'".to_string(),
            )),
        ).into_response();
    }

    // Ownership check via session
    if let Ok(None) = services::get_session(&pool, session_id, &owner).await {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                error_codes::NOT_FOUND.to_string(),
                "Session not found".to_string(),
            )),
        ).into_response();
    }

    let action = match services::get_pending_action(&pool, req.action_id, session_id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    error_codes::NOT_FOUND.to_string(),
                    "Action not found".to_string(),
                )),
            ).into_response()
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    error_codes::DATABASE_ERROR.to_string(),
                    "Failed to load action".to_string(),
                )),
            ).into_response()
        }
    };

    if action.status != "pending" {
        return (
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                error_codes::ALREADY_EXISTS.to_string(),
                format!("Action already resolved ({})", action.status),
            )),
        ).into_response();
    }

    if req.decision == "cancel" {
        let _ = services::resolve_pending_action(&pool, req.action_id, "cancelled").await;
        return (
            StatusCode::OK,
            Json(SuccessResponse::new(
                ConfirmActionResponse {
                    action_id: req.action_id,
                    status: "cancelled".to_string(),
                    result: None,
                    message: "Action annulée".to_string(),
                },
                "Cancelled".to_string(),
            )),
        ).into_response();
    }

    // Confirm: dispatch the action.
    let exec_result = dispatch_confirmed_action(&action, &user_jwt, &commerce_id, &slug).await;
    let _ = services::resolve_pending_action(&pool, req.action_id, "confirmed").await;

    let (status_str, result_json, message) = match exec_result {
        Ok(v) => ("confirmed".to_string(), Some(v), "Action exécutée".to_string()),
        Err(e) => ("confirmed".to_string(), None, format!("Action déclenchée mais erreur: {e}")),
    };

    (
        StatusCode::OK,
        Json(SuccessResponse::new(
            ConfirmActionResponse {
                action_id: req.action_id,
                status: status_str,
                result: result_json,
                message,
            },
            "Resolved".to_string(),
        )),
    ).into_response()
}

async fn dispatch_confirmed_action(
    action: &PendingAction,
    user_jwt: &str,
    commerce_id: &str,
    slug: &str,
) -> Result<serde_json::Value, String> {
    match action.tool_name.as_str() {
        "trigger_ai_run" => {
            let url = std::env::var("AI_SERVICE_URL")
                .unwrap_or_else(|_| "http://ai-service:8001".to_string());
            let resp = reqwest::Client::new()
                .post(format!("{}/ai/run", url.trim_end_matches('/')))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let body = resp
                .json::<serde_json::Value>()
                .await
                .unwrap_or_else(|_| json!({"ok": true}));
            Ok(body)
        }
        other => {
            // Toute autre action d'écriture : on délègue à la couche d'outils
            // Python via /chat/execute-tool. Le JWT user est transmis pour que
            // la RBAC s'applique.
            let session = action.session_id.to_string();
            match ai_client::execute_tool(other, &action.tool_args, user_jwt,
                                          commerce_id, slug, &session).await {
                Ok(v) => {
                    if v.get("ok").and_then(|x| x.as_bool()) == Some(false) {
                        let err = v.get("error").and_then(|x| x.as_str())
                            .unwrap_or("échec inconnu");
                        Err(err.to_string())
                    } else {
                        Ok(v.get("data").cloned().unwrap_or(v))
                    }
                }
                Err(e) => Err(e.to_string()),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/admin/chat/providers",
    tag = "chat",
    responses(
        (status = 200, description = "Provider health", body = inline(SuccessResponse<ProviderHealth>)),
        (status = 502, description = "AI service unavailable", body = ErrorResponse),
    ),
)]
pub async fn admin_provider_health() -> Response {
    match ai_client::provider_health().await {
        Ok(h) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                ProviderHealth {
                    default: h.default,
                    providers: h.providers.into_iter().map(|p| ProviderHealthEntry {
                        name: p.name,
                        available: p.available,
                        error: p.error,
                    }).collect(),
                },
                "Providers".to_string(),
            )),
        ).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new(
                error_codes::INTERNAL_SERVER_ERROR.to_string(),
                format!("AI service unavailable: {e}"),
            )),
        ).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/admin/rag/reindex",
    tag = "chat",
    responses(
        (status = 200, description = "Reindex metrics", body = inline(SuccessResponse<ReindexResponse>)),
        (status = 502, description = "AI service unavailable", body = ErrorResponse),
    ),
)]
pub async fn admin_rag_reindex() -> Response {
    match ai_client::reindex(false).await {
        Ok(m) => (
            StatusCode::OK,
            Json(SuccessResponse::new(
                ReindexResponse {
                    files_seen: m.files_seen,
                    files_embedded: m.files_embedded,
                    chunks_written: m.chunks_written,
                    skipped: m.skipped,
                },
                "Reindex complete".to_string(),
            )),
        ).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new(
                error_codes::INTERNAL_SERVER_ERROR.to_string(),
                format!("AI service unavailable: {e}"),
            )),
        ).into_response(),
    }
}
