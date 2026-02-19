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
    ),
    components(
        schemas(
            ErrorResponse,
            ErrorInfo,
            LoginRequest,
            LoginResponse,
            AdminRegisterRequest,
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
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
