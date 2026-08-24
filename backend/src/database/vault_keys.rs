use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug)]
pub struct VaultKeyRecord {
    pub user_id: Uuid,
    pub encrypted_vault_key: String,
    pub key_derivation_salt: String,
    pub key_derivation_params: serde_json::Value,
    pub encryption_nonce: String,
    pub encryption_version: String,
}

pub async fn create_vault_key(
    pool: &PgPool,
    user_id: Uuid,
    encrypted_vault_key: &str,
    key_derivation_salt: &str,
    key_derivation_params: &serde_json::Value,
    encryption_nonce: &str,
) -> Result<VaultKeyRecord, sqlx::Error> {
    let vault_key = sqlx::query_as!(
        VaultKeyRecord,
        r#"
        INSERT INTO vault_keys (
            user_id,
            encrypted_vault_key,
            key_derivation_salt,
            key_derivation_params,
            encryption_nonce
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            user_id,
            encrypted_vault_key,
            key_derivation_salt,
            key_derivation_params,
            encryption_nonce,
            encryption_version
        "#,
        user_id,
        encrypted_vault_key,
        key_derivation_salt,
        key_derivation_params,
        encryption_nonce
    )
    .fetch_one(pool)
    .await?;

    Ok(vault_key)
}

pub async fn get_vault_key(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<VaultKeyRecord>, sqlx::Error> {
    let vault_key = sqlx::query_as!(
        VaultKeyRecord,
        r#"
        SELECT
            user_id,
            encrypted_vault_key,
            key_derivation_salt,
            key_derivation_params,
            encryption_nonce,
            encryption_version
        FROM vault_keys
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(vault_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::users::create_user;

 async fn test_pool() -> PgPool {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for vault key tests");

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

    #[tokio::test]
    async fn create_and_retrieve_vault_key() {
        let pool = test_pool().await;

        let email = format!(
            "vault-key-test-{}@blackvault.local",
            Uuid::new_v4()
        );

        let password_hash =
            "$argon2id$v=19$m=19456,t=2,p=1$test$test";

        let user = create_user(
            &pool,
            &email,
            password_hash,
        )
        .await
        .expect("Failed to create test user");

        let user_id = user.id;

        let vault_key = create_vault_key(
            &pool,
            user_id,
            "test_encrypted_key",
            "test_salt",
            &serde_json::json!({
                "t_cost": 2,
                "m_cost": 19456,
                "p_cost": 1
            }),
            "test_nonce",
        )
        .await
        .expect("Failed to create vault key");

        assert_eq!(vault_key.user_id, user_id);
        assert_eq!(
            vault_key.encrypted_vault_key,
            "test_encrypted_key"
        );

        let retrieved = get_vault_key(&pool, user_id)
            .await
            .expect("Failed to retrieve vault key")
            .expect("Vault key not found");

        assert_eq!(retrieved.user_id, user_id);
        assert_eq!(
            retrieved.encrypted_vault_key,
            "test_encrypted_key"
        );

        sqlx::query!(
            "DELETE FROM vault_keys WHERE user_id = $1",
            user_id
        )
        .execute(&pool)
        .await
        .expect("Failed to clean up test vault key");

        sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            user_id
        )
        .execute(&pool)
        .await
        .expect("Failed to clean up test user");
    }

    #[tokio::test]
    async fn get_nonexistent_vault_key_returns_none() {
        let pool = test_pool().await;
        let user_id = Uuid::new_v4();

        let result = get_vault_key(&pool, user_id)
            .await
            .expect("Failed to query vault key");

        assert!(result.is_none());
    }
}