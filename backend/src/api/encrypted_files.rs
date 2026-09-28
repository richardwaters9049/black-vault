//! Version 2 envelopes: independent nonce-prefixed filename and wrapped-key blobs.
use crate::{
    api::{error::ErrorResponse, files::FileResponse, user::AuthenticatedUser},
    config::Config,
    database::files::FileRecord,
};
use axum::{extract::State, http::StatusCode, Json};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use sqlx::PgPool;

pub const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;
#[derive(Deserialize)]
pub struct UploadRequest {
    pub encrypted_filename: String,
    pub encrypted_file_key: String,
    pub encryption_nonce: String,
    pub encryption_version: String,
    pub file_size: i64,
    pub encrypted_data: String,
}

fn decode(value: &str) -> Result<Vec<u8>, ErrorResponse> {
    STANDARD
        .decode(value)
        .map_err(|_| ErrorResponse::bad_request("Invalid standard Base64"))
}

fn validate(payload: &UploadRequest) -> Result<Vec<u8>, ErrorResponse> {
    if payload.encryption_version != "2"
        || payload.file_size < 0
        || payload.file_size > MAX_FILE_SIZE as i64
    {
        return Err(ErrorResponse::bad_request(
            "Unsupported version or file size (maximum 5 MiB)",
        ));
    }
    let filename = decode(&payload.encrypted_filename)?;
    let wrapped_key = decode(&payload.encrypted_file_key)?;
    let nonce = decode(&payload.encryption_nonce)?;
    if !(29..=1052).contains(&filename.len()) || wrapped_key.len() != 60 || nonce.len() != 12 {
        return Err(ErrorResponse::bad_request("Invalid encryption metadata"));
    }
    // Contents and filename share one file key, so their nonces must differ.
    if filename[..12] == nonce {
        return Err(ErrorResponse::bad_request(
            "File encryption nonces must differ",
        ));
    }
    let data = decode(&payload.encrypted_data)?;
    if data.len() != payload.file_size as usize + 16 {
        return Err(ErrorResponse::bad_request(
            "Encrypted data size does not match file size",
        ));
    }
    Ok(data)
}

pub async fn upload(
    State((pool, _)): State<(PgPool, Config)>,
    AuthenticatedUser(user_id): AuthenticatedUser,
    Json(payload): Json<UploadRequest>,
) -> Result<(StatusCode, Json<FileResponse>), ErrorResponse> {
    let data = validate(&payload)?;
    // One statement stores the complete envelope and bytes, with no partial file.
    let file = sqlx::query_as::<_, FileRecord>(
        "INSERT INTO files (user_id, encrypted_filename, encrypted_file_key, encryption_nonce,
            encryption_version, file_size, encrypted_data)
         SELECT $1, $2, $3, $4, '2', $5, $6
         WHERE EXISTS (SELECT 1 FROM vault_keys WHERE user_id = $1)
         RETURNING id, user_id, encrypted_filename, encrypted_file_key, encryption_nonce,
            encryption_version, file_size, created_at, updated_at",
    )
    .bind(user_id)
    .bind(&payload.encrypted_filename)
    .bind(&payload.encrypted_file_key)
    .bind(&payload.encryption_nonce)
    .bind(payload.file_size)
    .bind(data)
    .fetch_optional(&pool)
    .await
    .map_err(|error| {
        tracing::error!(?error, %user_id, "Encrypted upload failed");
        ErrorResponse::internal_server_error("Unable to store encrypted file")
    })?
    .ok_or_else(|| ErrorResponse::conflict("Create a vault before uploading files"))?;
    tracing::info!(%user_id, file_id = %file.id, event = "encrypted_file_created");
    Ok((StatusCode::CREATED, Json(file.into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn payload() -> UploadRequest {
        UploadRequest {
            encrypted_filename: STANDARD.encode([1; 29]),
            encrypted_file_key: STANDARD.encode([2; 60]),
            encryption_nonce: STANDARD.encode([3; 12]),
            encryption_version: "2".into(),
            file_size: 0,
            encrypted_data: STANDARD.encode([4; 16]),
        }
    }
    #[test]
    fn validates_complete_envelopes_and_rejects_bad_metadata() {
        assert!(validate(&payload()).is_ok());
        let mut p = payload();
        p.encrypted_file_key = STANDARD.encode([2; 32]);
        assert!(validate(&p).is_err());
        let mut p = payload();
        p.file_size = MAX_FILE_SIZE as i64 + 1;
        assert!(validate(&p).is_err());
        let mut p = payload();
        p.encryption_nonce = STANDARD.encode([1; 12]);
        assert!(validate(&p).is_err());
        let mut p = payload();
        p.encrypted_data = STANDARD.encode([4; 15]);
        assert!(validate(&p).is_err());
    }
}
