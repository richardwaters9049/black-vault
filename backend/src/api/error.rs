use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{ser::SerializeStruct, Serialize, Serializer};

#[derive(Debug)]
pub struct ErrorResponse {
    status: StatusCode,
    pub error: String,
}

impl ErrorResponse {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            error: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: message.into(),
        }
    }

    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: message.into(),
        }
    }

    pub fn from_service_error(error: String) -> Self {
        let status = match error.as_str() {
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

        Self { status, error }
    }
}

impl Serialize for ErrorResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ErrorResponse", 1)?;
        state.serialize_field("error", &self.error)?;
        state.end()
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let status = self.status;

        (status, Json(self)).into_response()
    }
}
