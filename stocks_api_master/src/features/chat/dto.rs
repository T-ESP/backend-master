use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatSession {
    pub session_id: Uuid,
    /// Owner identity = the authenticated commerce email (multi-tenant: master
    /// auth is commerce-level, so sessions are keyed by email, not a user id).
    pub owner_email: String,
    pub title: Option<String>,
    pub provider: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatSessionWithMessages {
    pub session_id: Uuid,
    pub owner_email: String,
    pub title: Option<String>,
    pub provider: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSessionRequest {
    /// Optional title; if absent, auto-generated from first message.
    pub title: Option<String>,
    /// Provider preference: "auto" | "mistral" | "groq" | "local". Default "auto".
    pub provider: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSessionsQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct ChatMessage {
    pub message_id: i64,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_name: Option<String>,
    pub provider: Option<String>,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub latency_ms: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    pub content: String,
    /// Override session provider for this turn only.
    pub provider: Option<String>,
}

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct Citation {
    pub source_path: String,
    pub heading: String,
    pub similarity: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SendMessageResponse {
    pub assistant_message: ChatMessage,
    pub pending_action: Option<PendingAction>,
    pub provider_used: String,
    pub intent: String,
    pub citations: Vec<Citation>,
    pub cached: bool,
    pub shortcut_used: Option<String>,
    /// False si la réponse cite des chiffres non vérifiables dans les données.
    pub numbers_verified: bool,
    /// Questions de suivi suggérées (pré-résolues, appelables via /execute-tool).
    pub suggestions: Vec<serde_json::Value>,
    pub usage: ChatTurnUsage,
}

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct ChatTurnUsage {
    pub tokens_in: i32,
    pub tokens_out: i32,
    pub latency_ms: i32,
}

// ---------------------------------------------------------------------------
// Pending actions
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct PendingAction {
    pub action_id: Uuid,
    pub session_id: Uuid,
    pub message_id: Option<i64>,
    pub tool_name: String,
    pub tool_args: serde_json::Value,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfirmActionRequest {
    pub action_id: Uuid,
    /// "confirm" or "cancel".
    pub decision: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfirmActionResponse {
    pub action_id: Uuid,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderHealthEntry {
    pub name: String,
    pub available: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderHealth {
    pub default: String,
    pub providers: Vec<ProviderHealthEntry>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReindexResponse {
    pub files_seen: i32,
    pub files_embedded: i32,
    pub chunks_written: i32,
    pub skipped: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_session_request_optional_fields() {
        let json = r#"{}"#;
        let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
        assert!(req.title.is_none());
        assert!(req.provider.is_none());

        let json = r#"{"title": "Test", "provider": "groq"}"#;
        let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title.unwrap(), "Test");
        assert_eq!(req.provider.unwrap(), "groq");
    }

    #[test]
    fn send_message_request_minimum() {
        let json = r#"{"content": "Bonjour"}"#;
        let req: SendMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "Bonjour");
        assert!(req.provider.is_none());
    }

    #[test]
    fn confirm_action_request_parses() {
        let json = r#"{"action_id": "550e8400-e29b-41d4-a716-446655440000", "decision": "confirm"}"#;
        let req: ConfirmActionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.decision, "confirm");
    }

    #[test]
    fn chat_message_serializes_with_optional_fields_null() {
        let now = chrono::Utc::now();
        let m = ChatMessage {
            message_id: 1,
            session_id: Uuid::nil(),
            role: "user".to_string(),
            content: "hi".to_string(),
            tool_calls: None,
            tool_name: None,
            provider: None,
            tokens_in: None,
            tokens_out: None,
            latency_ms: None,
            created_at: now,
        };
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("\"tool_calls\":null"));
        assert!(s.contains("\"role\":\"user\""));
    }
}
