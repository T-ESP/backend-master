//! Database access for chat sessions, messages, and pending actions.
//!
//! Multi-tenant note: all queries run against the per-tenant pool injected by
//! `resolve_tenant_pool`. Session ownership is keyed by `owner_email`
//! (= the authenticated commerce email) since master auth is commerce-level.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::dto::*;

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

pub async fn create_session(
    pool: &PgPool,
    owner_email: &str,
    title: Option<&str>,
    provider: &str,
) -> Result<ChatSession, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO chat_sessions (owner_email, title, provider) VALUES ($1, $2, $3)
         RETURNING session_id, owner_email, title, provider, created_at, updated_at"
    )
    .bind(owner_email)
    .bind(title)
    .bind(provider)
    .fetch_one(pool)
    .await?;

    Ok(ChatSession {
        session_id: row.get("session_id"),
        owner_email: row.get("owner_email"),
        title: row.get("title"),
        provider: row.get("provider"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn list_sessions(
    pool: &PgPool,
    owner_email: &str,
    limit: i32,
    offset: i32,
) -> Result<Vec<ChatSession>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT session_id, owner_email, title, provider, created_at, updated_at
         FROM chat_sessions
         WHERE owner_email = $1
         ORDER BY updated_at DESC
         LIMIT $2 OFFSET $3"
    )
    .bind(owner_email)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| ChatSession {
        session_id: r.get("session_id"),
        owner_email: r.get("owner_email"),
        title: r.get("title"),
        provider: r.get("provider"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }).collect())
}

pub async fn get_session(
    pool: &PgPool,
    session_id: Uuid,
    owner_email: &str,
) -> Result<Option<ChatSession>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT session_id, owner_email, title, provider, created_at, updated_at
         FROM chat_sessions
         WHERE session_id = $1 AND owner_email = $2"
    )
    .bind(session_id)
    .bind(owner_email)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| ChatSession {
        session_id: r.get("session_id"),
        owner_email: r.get("owner_email"),
        title: r.get("title"),
        provider: r.get("provider"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }))
}

pub async fn delete_session(
    pool: &PgPool,
    session_id: Uuid,
    owner_email: &str,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM chat_sessions WHERE session_id = $1 AND owner_email = $2")
        .bind(session_id)
        .bind(owner_email)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn touch_session(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chat_sessions SET updated_at = NOW() WHERE session_id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_session_title_if_empty(
    pool: &PgPool,
    session_id: Uuid,
    new_title: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE chat_sessions
         SET title = $2
         WHERE session_id = $1 AND (title IS NULL OR title = '')"
    )
    .bind(session_id)
    .bind(new_title)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub async fn insert_message(
    pool: &PgPool,
    session_id: Uuid,
    role: &str,
    content: &str,
    tool_calls: Option<&serde_json::Value>,
    tool_name: Option<&str>,
    provider: Option<&str>,
    tokens_in: Option<i32>,
    tokens_out: Option<i32>,
    latency_ms: Option<i32>,
) -> Result<ChatMessage, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO chat_messages
            (session_id, role, content, tool_calls, tool_name, provider, tokens_in, tokens_out, latency_ms)
         VALUES ($1, $2::chat_role, $3, $4, $5, $6, $7, $8, $9)
         RETURNING message_id, session_id, role::text, content, tool_calls, tool_name,
                   provider, tokens_in, tokens_out, latency_ms, created_at"
    )
    .bind(session_id)
    .bind(role)
    .bind(content)
    .bind(tool_calls)
    .bind(tool_name)
    .bind(provider)
    .bind(tokens_in)
    .bind(tokens_out)
    .bind(latency_ms)
    .fetch_one(pool)
    .await?;

    Ok(ChatMessage {
        message_id: row.get("message_id"),
        session_id: row.get("session_id"),
        role: row.get("role"),
        content: row.get("content"),
        tool_calls: row.get("tool_calls"),
        tool_name: row.get("tool_name"),
        provider: row.get("provider"),
        tokens_in: row.get("tokens_in"),
        tokens_out: row.get("tokens_out"),
        latency_ms: row.get("latency_ms"),
        created_at: row.get("created_at"),
    })
}

pub async fn list_messages(
    pool: &PgPool,
    session_id: Uuid,
    limit: i32,
) -> Result<Vec<ChatMessage>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT message_id, session_id, role::text, content, tool_calls, tool_name,
                provider, tokens_in, tokens_out, latency_ms, created_at
         FROM chat_messages
         WHERE session_id = $1
         ORDER BY created_at ASC
         LIMIT $2"
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| ChatMessage {
        message_id: r.get("message_id"),
        session_id: r.get("session_id"),
        role: r.get("role"),
        content: r.get("content"),
        tool_calls: r.get("tool_calls"),
        tool_name: r.get("tool_name"),
        provider: r.get("provider"),
        tokens_in: r.get("tokens_in"),
        tokens_out: r.get("tokens_out"),
        latency_ms: r.get("latency_ms"),
        created_at: r.get("created_at"),
    }).collect())
}

// ---------------------------------------------------------------------------
// Pending actions
// ---------------------------------------------------------------------------

pub async fn create_pending_action(
    pool: &PgPool,
    session_id: Uuid,
    message_id: Option<i64>,
    tool_name: &str,
    tool_args: &serde_json::Value,
) -> Result<PendingAction, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO chat_pending_actions (session_id, message_id, tool_name, tool_args)
         VALUES ($1, $2, $3, $4)
         RETURNING action_id, session_id, message_id, tool_name, tool_args,
                   status::text, created_at"
    )
    .bind(session_id)
    .bind(message_id)
    .bind(tool_name)
    .bind(tool_args)
    .fetch_one(pool)
    .await?;

    Ok(PendingAction {
        action_id: row.get("action_id"),
        session_id: row.get("session_id"),
        message_id: row.get("message_id"),
        tool_name: row.get("tool_name"),
        tool_args: row.get("tool_args"),
        status: row.get("status"),
        created_at: row.get("created_at"),
    })
}

pub async fn get_pending_action(
    pool: &PgPool,
    action_id: Uuid,
    session_id: Uuid,
) -> Result<Option<PendingAction>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT action_id, session_id, message_id, tool_name, tool_args, status::text, created_at
         FROM chat_pending_actions
         WHERE action_id = $1 AND session_id = $2"
    )
    .bind(action_id)
    .bind(session_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| PendingAction {
        action_id: r.get("action_id"),
        session_id: r.get("session_id"),
        message_id: r.get("message_id"),
        tool_name: r.get("tool_name"),
        tool_args: r.get("tool_args"),
        status: r.get("status"),
        created_at: r.get("created_at"),
    }))
}

pub async fn resolve_pending_action(
    pool: &PgPool,
    action_id: Uuid,
    new_status: &str,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE chat_pending_actions
         SET status = $1::pending_action_status, resolved_at = NOW()
         WHERE action_id = $2 AND status = 'pending'"
    )
    .bind(new_status)
    .bind(action_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}
