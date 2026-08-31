use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    api::{error::ErrorResponse, user::AuthenticatedUser},
    database::files::{create_file, delete_file, get_file, list_files},
};

#[derive(Debug, Deserialize)]
pub struct CreateFileRequest {
    pub encrypted_filename: String,
    pub encrypted_file_key: String,
    pub encryption_nonce: String,
    pub file_size: i64,
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
    pub id: Uuid,
    pub encrypted_filename: String,
    pub encrypted_file_key: String,
    pub encryption_nonce: String,
    pub encryption_version: String,
    pub file_size: i64,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct FileListResponse {
    pub files: Vec<FileResponse>,
}

impl From<crate::database::files::FileRecord> for FileResponse {
    fn from(file: crate::database::files::FileRecord) -> Self {
        Self {
            id: file.id,
            encrypted_filename: file.encrypted_filename,
            encrypted_file_key: file.encrypted_file_key,
            encryption_nonce: file.encryption_nonce,
            encryption_version: file.encryption_version,
            file_size: file.file_size,
            created_at: file.created_at,
            updated_at: file.updated_at,
        }
    }
}

pub async fn create(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<CreateFileRequest>,
) -> Result<(StatusCode, Json<FileResponse>), ErrorResponse> {
    if payload.encrypted_filename.trim().is_empty() {
        return Err(ErrorResponse::bad_request(
            "Encrypted filename cannot be empty",
        ));
    }

    if payload.encrypted_file_key.trim().is_empty() {
        return Err(ErrorResponse::bad_request(
            "Encrypted file key cannot be empty",
        ));
    }

    if payload.encryption_nonce.trim().is_empty() {
        return Err(ErrorResponse::bad_request(
            "Encryption nonce cannot be empty",
        ));
    }

    if payload.file_size < 0 {
        return Err(ErrorResponse::bad_request("File size cannot be negative"));
    }

    let file = create_file(
        &pool,
        user_id,
        &payload.encrypted_filename,
        &payload.encrypted_file_key,
        &payload.encryption_nonce,
        payload.file_size,
    )
    .await
    .map_err(|error| {
        tracing::error!(
            error = ?error,
            user_id = %user_id,
            "Failed to create file"
        );

        ErrorResponse::internal_server_error("Failed to create file")
    })?;

    Ok((StatusCode::CREATED, Json(file.into())))
}

pub async fn list(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    AuthenticatedUser(user_id): AuthenticatedUser,
) -> Result<Json<FileListResponse>, ErrorResponse> {
    let files = list_files(&pool, user_id).await.map_err(|error| {
        tracing::error!(
            error = ?error,
            user_id = %user_id,
            "Failed to list files"
        );

        ErrorResponse::internal_server_error("Failed to list files")
    })?;

    Ok(Json(FileListResponse {
        files: files.into_iter().map(FileResponse::from).collect(),
    }))
}

pub async fn get(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(file_id): Path<Uuid>,
) -> Result<Json<FileResponse>, ErrorResponse> {
    let file = get_file(&pool, user_id, file_id)
        .await
        .map_err(|error| {
            tracing::error!(
                error = ?error,
                user_id = %user_id,
                file_id = %file_id,
                "Failed to retrieve file"
            );

            ErrorResponse::internal_server_error("Failed to retrieve file")
        })?
        .ok_or_else(|| ErrorResponse::not_found("File not found"))?;

    Ok(Json(file.into()))
}

pub async fn delete(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(file_id): Path<Uuid>,
) -> Result<StatusCode, ErrorResponse> {
    let deleted = delete_file(&pool, user_id, file_id)
        .await
        .map_err(|error| {
            tracing::error!(
                error = ?error,
                user_id = %user_id,
                file_id = %file_id,
                "Failed to delete file"
            );

            ErrorResponse::internal_server_error("Failed to delete file")
        })?;

    if !deleted {
        return Err(ErrorResponse::not_found("File not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
