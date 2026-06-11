use utoipa::OpenApi;

use crate::features::auth::dto::{
    LoginRequest,
    LoginResponse,
    AdminRegisterRequest,
};

use crate::common::responses::{ErrorResponse, ErrorInfo};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::features::auth::handlers::login,
        crate::features::auth::handlers::register,
        crate::features::chat::handlers::create_session,
        crate::features::chat::handlers::list_sessions,
        crate::features::chat::handlers::get_session,
        crate::features::chat::handlers::delete_session,
        crate::features::chat::handlers::send_message,
        crate::features::chat::handlers::send_message_stream,
        crate::features::chat::handlers::confirm_action,
        crate::features::chat::handlers::export_session,
        crate::features::chat::handlers::get_briefing,
        crate::features::chat::handlers::admin_provider_health,
        crate::features::chat::handlers::admin_rag_reindex,
    ),
    components(
        schemas(
            ErrorResponse,
            ErrorInfo,
            LoginRequest,
            LoginResponse,
            AdminRegisterRequest,
            crate::features::chat::dto::ChatSession,
            crate::features::chat::dto::ChatSessionWithMessages,
            crate::features::chat::dto::ChatMessage,
            crate::features::chat::dto::CreateSessionRequest,
            crate::features::chat::dto::SendMessageRequest,
            crate::features::chat::dto::SendMessageResponse,
            crate::features::chat::dto::Citation,
            crate::features::chat::dto::ChatTurnUsage,
            crate::features::chat::dto::PendingAction,
            crate::features::chat::dto::ConfirmActionRequest,
            crate::features::chat::dto::ConfirmActionResponse,
            crate::features::chat::dto::ProviderHealth,
            crate::features::chat::dto::ProviderHealthEntry,
            crate::features::chat::dto::ReindexResponse,
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "chat", description = "AI chatbot endpoints"),
    ),
    info(
        title = "Stock-S API",
        version = "1.0.0",
        description = "Multi-tenant stock management API",
        contact(
            name = "API Support",
            email = "support@example.com"
        ),
        license(
            name = "MIT",
        )
    )
)]
pub struct ApiDoc;
