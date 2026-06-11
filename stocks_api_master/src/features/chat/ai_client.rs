//! Thin HTTP client to the Python ai-service.
//!
//! All calls go over the internal Docker network. The user's JWT is forwarded
//! so tools called by the LLM act with the user's permissions. In the
//! multi-tenant deployment we additionally forward `commerce_id` + `slug` so the
//! ai-service can build tenant-scoped tool URLs (`/api/{commerce_id}/...`).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;

const DEFAULT_AI_URL: &str = "http://ai-service:8001";
// Long timeout: first-time chat may include a one-time ~1 GB local-LLM download
// before the response can be produced. Subsequent calls are sub-second.
const DEFAULT_TIMEOUT_SECS: u64 = 900;

fn ai_base() -> String {
    env::var("AI_SERVICE_URL").unwrap_or_else(|_| DEFAULT_AI_URL.to_string())
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .build()
        .expect("failed to build reqwest client")
}

#[derive(Debug, Serialize)]
pub struct TurnHistoryEntry<'a> {
    pub role: &'a str,
    pub content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct TurnRequest<'a> {
    pub user_message: &'a str,
    pub history: Vec<TurnHistoryEntry<'a>>,
    pub user_jwt: &'a str,
    /// Owner identity (commerce email). The ai-service does not need a numeric id.
    pub user_email: &'a str,
    /// Tenant context so the ai-service builds `/api/{commerce_id}/...` tool URLs.
    pub commerce_id: &'a str,
    pub slug: &'a str,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proactive_summary: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct TurnUsage {
    #[serde(default)] pub tokens_in: i32,
    #[serde(default)] pub tokens_out: i32,
    #[serde(default)] pub latency_ms: i32,
}

#[derive(Debug, Deserialize)]
pub struct TurnPendingAction {
    pub tool_name: String,
    pub tool_args: Value,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TurnCitation {
    pub source_path: String,
    #[serde(default)] pub heading: String,
    #[serde(default)] pub similarity: f64,
}

#[derive(Debug, Deserialize)]
pub struct TurnResponse {
    #[serde(default)] pub content: String,
    #[serde(default)] pub intent: String,
    #[serde(default)] pub provider_used: String,
    #[serde(default)] pub tool_calls: Vec<Value>,
    pub pending_action: Option<TurnPendingAction>,
    #[serde(default)] pub citations: Vec<TurnCitation>,
    #[serde(default)] pub cached: bool,
    pub shortcut_used: Option<String>,
    #[serde(default = "default_true")] pub numbers_verified: bool,
    #[serde(default)] pub suggestions: Vec<Value>,
    #[serde(default)] pub usage: Option<TurnUsage>,
}

fn default_true() -> bool { true }

pub async fn run_turn(req: &TurnRequest<'_>) -> Result<TurnResponse, reqwest::Error> {
    let url = format!("{}/chat/turn", ai_base());
    let resp = client().post(url).json(req).send().await?;
    let resp = resp.error_for_status()?;
    resp.json::<TurnResponse>().await
}

#[derive(Debug, Deserialize)]
pub struct ProviderHealthEntry {
    pub name: String,
    pub available: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderHealth {
    pub default: String,
    pub providers: Vec<ProviderHealthEntry>,
}

pub async fn provider_health() -> Result<ProviderHealth, reqwest::Error> {
    let url = format!("{}/llm/health", ai_base());
    client().get(url).send().await?.error_for_status()?.json().await
}

pub async fn briefing(
    user_jwt: &str,
    user_email: &str,
    commerce_id: &str,
    slug: &str,
    session_id: &str,
) -> Result<Value, reqwest::Error> {
    let url = format!("{}/chat/briefing", ai_base());
    let body = serde_json::json!({
        "user_jwt": user_jwt,
        "user_email": user_email,
        "commerce_id": commerce_id,
        "slug": slug,
        "session_id": session_id,
    });
    client().post(url).json(&body).send().await?.error_for_status()?.json().await
}

/// Exécute un outil par son nom côté ai-service (utilisé pour les actions
/// d'écriture confirmées). Renvoie le JSON {ok, data, error}.
pub async fn execute_tool(
    tool_name: &str,
    tool_args: &Value,
    user_jwt: &str,
    commerce_id: &str,
    slug: &str,
    session_id: &str,
) -> Result<Value, reqwest::Error> {
    let url = format!("{}/chat/execute-tool", ai_base());
    let body = serde_json::json!({
        "tool_name": tool_name,
        "tool_args": tool_args,
        "user_jwt": user_jwt,
        "commerce_id": commerce_id,
        "slug": slug,
        "session_id": session_id,
    });
    client().post(url).json(&body).send().await?.error_for_status()?.json().await
}

#[derive(Debug, Deserialize)]
pub struct ReindexMetrics {
    #[serde(default)] pub files_seen: i32,
    #[serde(default)] pub files_embedded: i32,
    #[serde(default)] pub chunks_written: i32,
    #[serde(default)] pub skipped: i32,
}

pub async fn reindex(force: bool) -> Result<ReindexMetrics, reqwest::Error> {
    let url = format!("{}/rag/reindex", ai_base());
    client()
        .post(url)
        .json(&serde_json::json!({"force": force}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}
