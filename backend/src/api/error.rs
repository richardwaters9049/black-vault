use crate::services::auth::AuthError;
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

    pub fn from_auth_error(error: AuthError) -> Self {
        let status = match error {
            AuthError::EmailTaken => StatusCode::CONFLICT,
            AuthError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AuthError::Unavailable => StatusCode::INTERNAL_SERVER_ERROR,
            AuthError::EmailRequired
            | AuthError::InvalidEmail
            | AuthError::PasswordRequired
            | AuthError::PasswordTooShort
            | AuthError::UppercaseRequired
            | AuthError::LowercaseRequired
            | AuthError::DigitRequired => StatusCode::BAD_REQUEST,
        };
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            eprintln!("[black-vault-api] internal service error (details withheld)");
        }
        Self {
            status,
            error: error.message().into(),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_service_errors_are_not_exposed() {
        let response = ErrorResponse::from_auth_error(AuthError::Unavailable);

        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.error, "Internal server error");
    }

    #[test]
    fn known_service_errors_keep_their_client_message() {
        let response = ErrorResponse::from_auth_error(AuthError::InvalidCredentials);

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
        assert_eq!(response.error, "Invalid email or password");
    }

    #[test]
    fn typed_auth_errors_preserve_http_statuses() {
        for (error, status) in [
            (AuthError::EmailTaken, StatusCode::CONFLICT),
            (AuthError::InvalidCredentials, StatusCode::UNAUTHORIZED),
            (AuthError::Unavailable, StatusCode::INTERNAL_SERVER_ERROR),
            (AuthError::InvalidEmail, StatusCode::BAD_REQUEST),
            (AuthError::PasswordTooShort, StatusCode::BAD_REQUEST),
        ] {
            assert_eq!(ErrorResponse::from_auth_error(error).status, status);
        }
    }
}
