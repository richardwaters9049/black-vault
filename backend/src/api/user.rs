use axum::{
    extract::{
        FromRequestParts,
        State,
    },
    http::StatusCode,
    Json,
};

use serde::Serialize;

use sqlx::PgPool;

use uuid::Uuid;

use crate::api::error::ErrorResponse;
use crate::database::users::find_user_by_id;

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
}

pub struct AuthenticatedUser(pub Uuid);

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Uuid>()
            .cloned()
            .map(AuthenticatedUser)
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Unauthorized".to_string(),
                )
            })
    }
}

pub async fn me(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> Result<Json<UserResponse>, ErrorResponse> {
    let user_record = find_user_by_id(&pool, user_id)
        .await
        .map_err(|_| ErrorResponse {
            error: "Failed to fetch user".to_string(),
        })?
        .ok_or_else(|| ErrorResponse {
            error: "User not found".to_string(),
        })?;

    let user_response = UserResponse {
        id: user_record.id,
        email: user_record.email,
    };

    Ok(Json(user_response))
}
