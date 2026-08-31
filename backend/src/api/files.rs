use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header::CONTENT_TYPE, StatusCode},
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    api::{error::ErrorResponse, user::AuthenticatedUser},
    database::files::{
        create_file, delete_file, get_encrypted_data, get_file, list_files, store_encrypted_data,
    },
};

const ENCRYPTION_KEY_LENGTH: usize = 32;
const ENCRYPTION_NONCE_LENGTH: usize = 12;
const AES_GCM_TAG_LENGTH: usize = 16;

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

    let encrypted_file_key = general_purpose::STANDARD
        .decode(&payload.encrypted_file_key)
        .map_err(|_| ErrorResponse::bad_request("Encrypted file key must be valid Base64"))?;

    if encrypted_file_key.len() != ENCRYPTION_KEY_LENGTH {
        return Err(ErrorResponse::bad_request(
            "Encrypted file key has invalid length",
        ));
    }

    let encryption_nonce = general_purpose::STANDARD
        .decode(&payload.encryption_nonce)
        .map_err(|_| ErrorResponse::bad_request("Encryption nonce must be valid Base64"))?;

    if encryption_nonce.len() != ENCRYPTION_NONCE_LENGTH {
        return Err(ErrorResponse::bad_request(
            "Encryption nonce has invalid length",
        ));
    }

    let encrypted_filename = general_purpose::STANDARD
        .decode(&payload.encrypted_filename)
        .map_err(|_| ErrorResponse::bad_request("Encrypted filename must be valid Base64"))?;

    if encrypted_filename.is_empty() {
        return Err(ErrorResponse::bad_request(
            "Encrypted filename cannot be empty",
        ));
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

pub async fn upload_data(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(file_id): Path<Uuid>,
    body: Bytes,
) -> Result<StatusCode, ErrorResponse> {
    let file = get_file(&pool, user_id, file_id)
        .await
        .map_err(|error| {
            tracing::error!(
                error = ?error,
                user_id = %user_id,
                file_id = %file_id,
                "Failed to retrieve file before encrypted data upload"
            );

            ErrorResponse::internal_server_error("Failed to retrieve file")
        })?
        .ok_or_else(|| ErrorResponse::not_found("File not found"))?;

    let expected_encrypted_size = usize::try_from(file.file_size)
        .ok()
        .and_then(|size| size.checked_add(AES_GCM_TAG_LENGTH));

    let expected_encrypted_size = expected_encrypted_size
        .ok_or_else(|| ErrorResponse::bad_request("File size is too large to process"))?;

    if body.len() != expected_encrypted_size {
        return Err(ErrorResponse::bad_request(
            "Encrypted data size does not match file size",
        ));
    }

    store_encrypted_data(&pool, user_id, file_id, body.as_ref())
        .await
        .map_err(|error| {
            tracing::error!(
                error = ?error,
                user_id = %user_id,
                file_id = %file_id,
                "Failed to store encrypted file data"
            );

            ErrorResponse::internal_server_error("Failed to store encrypted file data")
        })?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn download_data(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Path(file_id): Path<Uuid>,
) -> Result<([(axum::http::HeaderName, &'static str); 1], Bytes), ErrorResponse> {
    get_file(&pool, user_id, file_id)
        .await
        .map_err(|error| {
            tracing::error!(
                error = ?error,
                user_id = %user_id,
                file_id = %file_id,
                "Failed to retrieve file before encrypted data download"
            );

            ErrorResponse::internal_server_error("Failed to retrieve file")
        })?
        .ok_or_else(|| ErrorResponse::not_found("File not found"))?;

    let encrypted_data = get_encrypted_data(&pool, user_id, file_id)
        .await
        .map_err(|error| {
            tracing::error!(
                error = ?error,
                user_id = %user_id,
                file_id = %file_id,
                "Failed to retrieve encrypted file data"
            );

            ErrorResponse::internal_server_error("Failed to retrieve encrypted file data")
        })?
        .ok_or_else(|| ErrorResponse::not_found("Encrypted file data not found"))?;

    Ok((
        [(CONTENT_TYPE, "application/octet-stream")],
        Bytes::from(encrypted_data),
    ))
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
