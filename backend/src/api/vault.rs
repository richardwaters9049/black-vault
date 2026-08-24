use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api::error::ErrorResponse;
use crate::api::user::AuthenticatedUser;
use crate::crypto::derivation::DerivationParams;
use crate::database::vault_keys::{create_vault_key, get_vault_key as get_vault_key_from_db};

#[derive(Debug, Deserialize)]
pub struct SetupVaultRequest {
    pub encrypted_vault_key: String,
    pub key_derivation_salt: String,
    pub key_derivation_params: DerivationParams,
    pub encryption_nonce: String,
}

#[derive(Debug, Serialize)]
pub struct SetupVaultResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct GetVaultKeyResponse {
    pub encrypted_vault_key: String,
    pub key_derivation_salt: String,
    pub key_derivation_params: DerivationParams,
    pub encryption_nonce: String,
    pub encryption_version: String,
}

pub async fn setup_vault(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    authenticated_user: AuthenticatedUser,
    Json(payload): Json<SetupVaultRequest>,
) -> Result<(StatusCode, Json<SetupVaultResponse>), ErrorResponse> {
    let params_json = serde_json::to_value(&payload.key_derivation_params)
        .map_err(|e| ErrorResponse {
            error: format!("Failed to serialize derivation params: {}", e),
        })?;

    create_vault_key(
        &pool,
        authenticated_user.0,
        &payload.encrypted_vault_key,
        &payload.key_derivation_salt,
        &params_json,
        &payload.encryption_nonce,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "Failed to create vault key");
        ErrorResponse {
            error: "Failed to setup vault".to_string(),
        }
    })?;

    Ok((
        StatusCode::CREATED,
        Json(SetupVaultResponse { success: true }),
    ))
}

pub async fn get_vault_key(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    authenticated_user: AuthenticatedUser,
) -> Result<(StatusCode, Json<GetVaultKeyResponse>), ErrorResponse> {
    let vault_key = get_vault_key_from_db(&pool, authenticated_user.0)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to retrieve vault key");
            ErrorResponse {
                error: "Failed to retrieve vault key".to_string(),
            }
        })?;

    let vault_key = vault_key.ok_or_else(|| ErrorResponse {
        error: "Vault not set up".to_string(),
    })?;

    let derivation_params: DerivationParams = serde_json::from_value(vault_key.key_derivation_params)
        .map_err(|e| ErrorResponse {
            error: format!("Failed to parse derivation params: {}", e),
        })?;

    Ok((
        StatusCode::OK,
        Json(GetVaultKeyResponse {
            encrypted_vault_key: vault_key.encrypted_vault_key,
            key_derivation_salt: vault_key.key_derivation_salt,
            key_derivation_params: derivation_params,
            encryption_nonce: vault_key.encryption_nonce,
            encryption_version: vault_key.encryption_version,
        }),
    ))
}
