use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct FileRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub encrypted_filename: String,
    pub encrypted_file_key: String,
    pub encryption_nonce: String,
    pub encryption_version: String,
    pub file_size: i64,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub async fn create_file(
    pool: &PgPool,
    user_id: Uuid,
    encrypted_filename: &str,
    encrypted_file_key: &str,
    encryption_nonce: &str,
    file_size: i64,
) -> Result<FileRecord, sqlx::Error> {
    let file = sqlx::query_as!(
        FileRecord,
        r#"
        INSERT INTO files (
            user_id,
            encrypted_filename,
            encrypted_file_key,
            encryption_nonce,
            file_size
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
            id,
            user_id,
            encrypted_filename,
            encrypted_file_key,
            encryption_nonce,
            encryption_version,
            file_size,
            created_at,
            updated_at
        "#,
        user_id,
        encrypted_filename,
        encrypted_file_key,
        encryption_nonce,
        file_size
    )
    .fetch_one(pool)
    .await?;

    Ok(file)
}

pub async fn get_file(
    pool: &PgPool,
    user_id: Uuid,
    file_id: Uuid,
) -> Result<Option<FileRecord>, sqlx::Error> {
    let file = sqlx::query_as!(
        FileRecord,
        r#"
        SELECT
            id,
            user_id,
            encrypted_filename,
            encrypted_file_key,
            encryption_nonce,
            encryption_version,
            file_size,
            created_at,
            updated_at
        FROM files
        WHERE id = $1
          AND user_id = $2
        "#,
        file_id,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(file)
}

pub async fn list_files(pool: &PgPool, user_id: Uuid) -> Result<Vec<FileRecord>, sqlx::Error> {
    let files = sqlx::query_as!(
        FileRecord,
        r#"
        SELECT
            id,
            user_id,
            encrypted_filename,
            encrypted_file_key,
            encryption_nonce,
            encryption_version,
            file_size,
            created_at,
            updated_at
        FROM files
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(files)
}

pub async fn store_encrypted_data(
    pool: &PgPool,
    user_id: Uuid,
    file_id: Uuid,
    encrypted_data: &[u8],
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE files
        SET encrypted_data = $1,
            updated_at = NOW()
        WHERE id = $2
          AND user_id = $3
        "#,
        encrypted_data,
        file_id,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn get_encrypted_data(
    pool: &PgPool,
    user_id: Uuid,
    file_id: Uuid,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    let file = sqlx::query!(
        r#"
        SELECT encrypted_data
        FROM files
        WHERE id = $1
          AND user_id = $2
        "#,
        file_id,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(file.and_then(|record| record.encrypted_data))
}

pub async fn delete_file(pool: &PgPool, user_id: Uuid, file_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM files
        WHERE id = $1
          AND user_id = $2
        "#,
        file_id,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::users::create_user;

    async fn test_pool() -> PgPool {
        dotenvy::dotenv().ok();

        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for file tests");

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    async fn create_test_user(pool: &PgPool) -> Uuid {
        let email = format!("file-test-{}@blackvault.local", Uuid::new_v4());

        let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$test$test";

        create_user(pool, &email, password_hash)
            .await
            .expect("Failed to create test user")
            .id
    }

    #[tokio::test]
    async fn create_and_retrieve_file() {
        let pool = test_pool().await;

        let user_id = create_test_user(&pool).await;

        let file = create_file(
            &pool,
            user_id,
            "encrypted_filename",
            "encrypted_file_key",
            "encrypted_nonce",
            12345,
        )
        .await
        .expect("Failed to create test file");

        assert_eq!(file.user_id, user_id);
        assert_eq!(file.encrypted_filename, "encrypted_filename");
        assert_eq!(file.encrypted_file_key, "encrypted_file_key");
        assert_eq!(file.encryption_nonce, "encrypted_nonce");
        assert_eq!(file.encryption_version, "1");
        assert_eq!(file.file_size, 12345);
        assert!(!file.id.is_nil());

        let retrieved = get_file(&pool, user_id, file.id)
            .await
            .expect("Failed to retrieve test file")
            .expect("Test file was not found");

        assert_eq!(retrieved.id, file.id);
        assert_eq!(retrieved.user_id, user_id);
        assert_eq!(retrieved.encrypted_filename, "encrypted_filename");
        assert_eq!(retrieved.encrypted_file_key, "encrypted_file_key");
        assert_eq!(retrieved.encryption_nonce, "encrypted_nonce");
        assert_eq!(retrieved.file_size, 12345);

        sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up test user");
    }

    #[tokio::test]
    async fn store_and_retrieve_encrypted_data() {
        let pool = test_pool().await;

        let user_id = create_test_user(&pool).await;

        let file = create_file(
            &pool,
            user_id,
            "encrypted_filename",
            "encrypted_file_key",
            "encrypted_nonce",
            10,
        )
        .await
        .expect("Failed to create test file");

        let encrypted_data = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let stored = store_encrypted_data(&pool, user_id, file.id, &encrypted_data)
            .await
            .expect("Failed to store encrypted data");

        assert!(stored);

        let retrieved = get_encrypted_data(&pool, user_id, file.id)
            .await
            .expect("Failed to retrieve encrypted data")
            .expect("Encrypted data was not found");

        assert_eq!(retrieved, encrypted_data);

        sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up test user");
    }

    #[tokio::test]
    async fn get_encrypted_data_rejects_another_users_file() {
        let pool = test_pool().await;

        let owner_id = create_test_user(&pool).await;
        let attacker_id = create_test_user(&pool).await;

        let file = create_file(
            &pool,
            owner_id,
            "owner_file",
            "owner_key",
            "owner_nonce",
            10,
        )
        .await
        .expect("Failed to create test file");

        store_encrypted_data(&pool, owner_id, file.id, &[1u8, 2, 3, 4])
            .await
            .expect("Failed to store encrypted data");

        let result = get_encrypted_data(&pool, attacker_id, file.id)
            .await
            .expect("Failed to query encrypted data");

        assert!(result.is_none());

        sqlx::query!("DELETE FROM users WHERE id = $1", owner_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up owner");

        sqlx::query!("DELETE FROM users WHERE id = $1", attacker_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up attacker");
    }

    #[tokio::test]
    async fn store_encrypted_data_rejects_another_users_file() {
        let pool = test_pool().await;

        let owner_id = create_test_user(&pool).await;
        let attacker_id = create_test_user(&pool).await;

        let file = create_file(
            &pool,
            owner_id,
            "owner_file",
            "owner_key",
            "owner_nonce",
            10,
        )
        .await
        .expect("Failed to create test file");

        let stored = store_encrypted_data(&pool, attacker_id, file.id, &[1u8, 2, 3, 4])
            .await
            .expect("Failed to attempt unauthorized write");

        assert!(!stored);

        let result = get_encrypted_data(&pool, owner_id, file.id)
            .await
            .expect("Failed to query encrypted data");

        assert!(result.is_none());

        sqlx::query!("DELETE FROM users WHERE id = $1", owner_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up owner");

        sqlx::query!("DELETE FROM users WHERE id = $1", attacker_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up attacker");
    }

    #[tokio::test]
    async fn list_files_returns_only_users_files() {
        let pool = test_pool().await;

        let user_a_id = create_test_user(&pool).await;
        let user_b_id = create_test_user(&pool).await;

        create_file(&pool, user_a_id, "user_a_file", "key_a", "nonce_a", 100)
            .await
            .expect("Failed to create user A file");

        create_file(&pool, user_b_id, "user_b_file", "key_b", "nonce_b", 200)
            .await
            .expect("Failed to create user B file");

        let files = list_files(&pool, user_a_id)
            .await
            .expect("Failed to list user files");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].user_id, user_a_id);
        assert_eq!(files[0].encrypted_filename, "user_a_file");

        sqlx::query!("DELETE FROM users WHERE id = $1", user_a_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up user A");

        sqlx::query!("DELETE FROM users WHERE id = $1", user_b_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up user B");
    }

    #[tokio::test]
    async fn get_file_rejects_access_from_another_user() {
        let pool = test_pool().await;

        let owner_id = create_test_user(&pool).await;
        let attacker_id = create_test_user(&pool).await;

        let file = create_file(
            &pool,
            owner_id,
            "owner_file",
            "owner_key",
            "owner_nonce",
            500,
        )
        .await
        .expect("Failed to create test file");

        let result = get_file(&pool, attacker_id, file.id)
            .await
            .expect("Failed to query test file");

        assert!(result.is_none());

        sqlx::query!("DELETE FROM users WHERE id = $1", owner_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up owner");

        sqlx::query!("DELETE FROM users WHERE id = $1", attacker_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up attacker");
    }

    #[tokio::test]
    async fn delete_file_removes_owned_file() {
        let pool = test_pool().await;

        let user_id = create_test_user(&pool).await;

        let file = create_file(
            &pool,
            user_id,
            "delete_me",
            "delete_key",
            "delete_nonce",
            999,
        )
        .await
        .expect("Failed to create test file");

        let deleted = delete_file(&pool, user_id, file.id)
            .await
            .expect("Failed to delete test file");

        assert!(deleted);

        let retrieved = get_file(&pool, user_id, file.id)
            .await
            .expect("Failed to query deleted file");

        assert!(retrieved.is_none());

        sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up test user");
    }

    #[tokio::test]
    async fn delete_file_rejects_another_users_file() {
        let pool = test_pool().await;

        let owner_id = create_test_user(&pool).await;
        let attacker_id = create_test_user(&pool).await;

        let file = create_file(
            &pool,
            owner_id,
            "protected_file",
            "protected_key",
            "protected_nonce",
            1000,
        )
        .await
        .expect("Failed to create test file");

        let deleted = delete_file(&pool, attacker_id, file.id)
            .await
            .expect("Failed to attempt unauthorized deletion");

        assert!(!deleted);

        let retrieved = get_file(&pool, owner_id, file.id)
            .await
            .expect("Failed to retrieve protected file");

        assert!(retrieved.is_some());

        sqlx::query!("DELETE FROM users WHERE id = $1", owner_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up owner");

        sqlx::query!("DELETE FROM users WHERE id = $1", attacker_id)
            .execute(&pool)
            .await
            .expect("Failed to clean up attacker");
    }
}
