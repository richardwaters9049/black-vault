use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use serde::{
    Deserialize,
    Serialize,
};

use sqlx::PgPool;

use uuid::Uuid;

use crate::config::Config;

use crate::services::auth::{
    login_user,
    register_user,
};

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
    State((pool, config)): State<(PgPool, Config)>,
    Json(payload): Json<RegisterRequest>,
) -> Result<
    (StatusCode, Json<RegisterResponse>),
    (StatusCode, String),
> {
    let user =
        register_user(
            &pool,
            &payload.email,
            &payload.password,
        )
        .await
        .map_err(|error| {
            if error
                == "An account with that email already exists"
            {
                return (
                    StatusCode::CONFLICT,
                    error,
                );
            }

            if error
                == "Password must be at least 12 characters"
                || error
                    == "Email is required"
            {
                return (
                    StatusCode::BAD_REQUEST,
                    error,
                );
            }

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                error,
            )
        })?;

    let _ = config;

    Ok((
        StatusCode::CREATED,
        Json(
            RegisterResponse {
                id: user.id,
                email: user.email,
            }
        ),
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
) -> Result<
    (StatusCode, Json<LoginResponse>),
    (StatusCode, String),
> {
    let (user, token) =
        login_user(
            &pool,
            &payload.email,
            &payload.password,
            &config.jwt_secret,
            config.jwt_expiration,
        )
        .await
        .map_err(|error| {
            if error
                == "Invalid email or password"
            {
                return (
                    StatusCode::UNAUTHORIZED,
                    error,
                );
            }

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                error,
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(
            LoginResponse {
                id: user.id,
                email: user.email,
                token,
            }
        ),
    ))
}