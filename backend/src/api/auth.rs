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

use crate::services::auth::hash_password;


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
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<
    (StatusCode, Json<RegisterResponse>),
    (StatusCode, String),
> {

    let email = payload
        .email
        .trim()
        .to_lowercase();

    if email.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Email is required".to_string(),
        ));
    }

    if payload.password.len() < 12 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 12 characters".to_string(),
        ));
    }

    let password_hash =
        hash_password(&payload.password)
            .map_err(|_| (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to process password".to_string(),
            ))?;

    let user = sqlx::query!(
        r#"
        INSERT INTO users (
            email,
            password_hash
        )
        VALUES ($1, $2)
        RETURNING id, email
        "#,
        email,
        password_hash
    )
    .fetch_one(&pool)
    .await
    .map_err(|error| {

        if let sqlx::Error::Database(database_error) = &error {

            if database_error.constraint()
                == Some("users_email_key")
            {
                return (
                    StatusCode::CONFLICT,
                    "An account with that email already exists"
                        .to_string(),
                );
            }
        }

        tracing::error!(
            error = ?error,
            "Failed to create user"
        );

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to create account".to_string(),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            id: user.id,
            email: user.email,
        }),
    ))
}