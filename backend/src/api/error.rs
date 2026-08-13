use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.as_str() {
            "An account with that email already exists" => StatusCode::CONFLICT,
            "Password must be at least 12 characters"
            | "Password must contain at least one uppercase letter"
            | "Password must contain at least one lowercase letter"
            | "Password must contain at least one digit"
            | "Invalid email format"
            | "Email is required"
            | "Password is required" => StatusCode::BAD_REQUEST,
            "Invalid email or password" => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}
