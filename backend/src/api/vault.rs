use axum::{extract::State, http::StatusCode, Json};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api::error::ErrorResponse;
use crate::api::user::AuthenticatedUser;
use crate::crypto::derivation::DerivationParams;
use crate::database::vault_keys::{create_vault_key, get_vault_key as get_vault_key_from_db};

const AES_GCM_NONCE_SIZE: usize = 12;
const VAULT_KEY_SIZE: usize = 32;
const VAULT_KEY_CIPHERTEXT_SIZE: usize = 48;
const SALT_SIZE: usize = 16;

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

fn decode_base64_field(value: &str, field_name: &str) -> Result<Vec<u8>, ErrorResponse> {
    general_purpose::STANDARD
        .decode(value)
        .map_err(|_| ErrorResponse::bad_request(format!("{} must be valid base64", field_name)))
}

fn validate_decoded_len(
    value: &str,
    field_name: &str,
    expected_len: usize,
) -> Result<(), ErrorResponse> {
    let decoded = decode_base64_field(value, field_name)?;

    if decoded.len() != expected_len {
        return Err(ErrorResponse::bad_request(format!(
            "{} has invalid length",
            field_name
        )));
    }

    Ok(())
}

fn validate_derivation_params(params: &DerivationParams) -> Result<(), ErrorResponse> {
    validate_decoded_len(&params.salt, "key_derivation_params.salt", SALT_SIZE)?;

    if params.t_cost != 2
        || params.m_cost != 19456
        || params.p_cost != 1
        || params.output_len != VAULT_KEY_SIZE
    {
        return Err(ErrorResponse::bad_request(
            "Unsupported key derivation parameters",
        ));
    }

    Ok(())
}

fn validate_setup_vault_request(payload: &SetupVaultRequest) -> Result<(), ErrorResponse> {
    validate_decoded_len(
        &payload.encrypted_vault_key,
        "encrypted_vault_key",
        VAULT_KEY_CIPHERTEXT_SIZE,
    )?;

    validate_decoded_len(
        &payload.key_derivation_salt,
        "key_derivation_salt",
        SALT_SIZE,
    )?;

    validate_decoded_len(
        &payload.encryption_nonce,
        "encryption_nonce",
        AES_GCM_NONCE_SIZE,
    )?;

    validate_derivation_params(&payload.key_derivation_params)?;

    if payload.key_derivation_salt != payload.key_derivation_params.salt {
        return Err(ErrorResponse::bad_request(
            "Key derivation salt does not match parameters",
        ));
    }

    Ok(())
}

pub async fn setup_vault(
    State((pool, _config)): State<(PgPool, crate::config::Config)>,
    authenticated_user: AuthenticatedUser,
    Json(payload): Json<SetupVaultRequest>,
) -> Result<(StatusCode, Json<SetupVaultResponse>), ErrorResponse> {
    validate_setup_vault_request(&payload)?;

    let params_json = serde_json::to_value(&payload.key_derivation_params).map_err(|error| {
        tracing::error!(
            error = ?error,
            "Failed to serialize vault key derivation params"
        );

        ErrorResponse::internal_server_error("Failed to setup vault")
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

        if let sqlx::Error::Database(database_error) = &e {
            if database_error.constraint() == Some("vault_keys_pkey") {
                return ErrorResponse::conflict("Vault is already set up");
            }
        }

        ErrorResponse::internal_server_error("Failed to setup vault")
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
            ErrorResponse::internal_server_error("Failed to retrieve vault key")
        })?;

    let vault_key = vault_key.ok_or_else(|| ErrorResponse::not_found("Vault not set up"))?;

    let derivation_params: DerivationParams =
        serde_json::from_value(vault_key.key_derivation_params).map_err(|error| {
            tracing::error!(
                error = ?error,
                "Failed to parse stored vault key derivation params"
            );

            ErrorResponse::internal_server_error("Failed to retrieve vault key")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_request() -> SetupVaultRequest {
        SetupVaultRequest {
            encrypted_vault_key: general_purpose::STANDARD.encode([7u8; VAULT_KEY_CIPHERTEXT_SIZE]),
            key_derivation_salt: general_purpose::STANDARD.encode([9u8; SALT_SIZE]),
            key_derivation_params: DerivationParams {
                salt: general_purpose::STANDARD.encode([9u8; SALT_SIZE]),
                t_cost: 2,
                m_cost: 19456,
                p_cost: 1,
                output_len: VAULT_KEY_SIZE,
            },
            encryption_nonce: general_purpose::STANDARD.encode([5u8; AES_GCM_NONCE_SIZE]),
        }
    }

    #[test]
    fn setup_vault_validation_accepts_supported_metadata() {
        let request = valid_request();

        assert!(validate_setup_vault_request(&request).is_ok());
    }

    #[test]
    fn setup_vault_validation_rejects_invalid_nonce_length() {
        let mut request = valid_request();
        request.encryption_nonce = general_purpose::STANDARD.encode([5u8; AES_GCM_NONCE_SIZE - 1]);

        assert!(validate_setup_vault_request(&request).is_err());
    }

    #[test]
    fn setup_vault_validation_rejects_salt_mismatch() {
        let mut request = valid_request();
        request.key_derivation_params.salt = general_purpose::STANDARD.encode([8u8; SALT_SIZE]);

        assert!(validate_setup_vault_request(&request).is_err());
    }

    #[test]
    fn setup_vault_validation_rejects_unsupported_params() {
        let mut request = valid_request();
        request.key_derivation_params.output_len = 16;

        assert!(validate_setup_vault_request(&request).is_err());
    }
}
