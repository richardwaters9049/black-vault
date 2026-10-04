use axum::{extract::State, http::StatusCode, Json};

use serde::{Deserialize, Serialize};

use sqlx::PgPool;

use uuid::Uuid;

use crate::api::error::ErrorResponse;
use crate::config::Config;

use crate::services::auth::{login_user, register_user};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: Uuid,
    pub email: String,
}

pub async fn register(
    State((pool, _config)): State<(PgPool, Config)>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), ErrorResponse> {
    let user = register_user(&pool, &payload.email, &payload.password)
        .await
        .map_err(ErrorResponse::from_auth_error)?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            id: user.id,
            email: user.email,
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub id: Uuid,
    pub email: String,
    pub token: String,
}

pub async fn login(
    State((pool, config)): State<(PgPool, Config)>,
    Json(payload): Json<LoginRequest>,
) -> Result<(StatusCode, Json<LoginResponse>), ErrorResponse> {
    let (user, token) = login_user(&pool, &config, &payload.email, &payload.password)
        .await
        .map_err(ErrorResponse::from_auth_error)?;

    Ok((
        StatusCode::OK,
        Json(LoginResponse {
            id: user.id,
            email: user.email,
            token,
        }),
    ))
}
