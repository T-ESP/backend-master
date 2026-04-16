use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse<T> {
    pub success: bool,
    pub data: T,
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorInfo,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

impl<T> SuccessResponse<T> {
    pub fn new(data: T, message: String) -> Self {
        Self {
            success: true,
            data,
            message,
        }
    }
}

impl ErrorResponse {
    pub fn new(code: String, message: String) -> Self {
        Self {
            success: false,
            error: ErrorInfo { code, message },
        }
    }
}