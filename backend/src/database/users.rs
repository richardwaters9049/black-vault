use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug)]
pub struct UserRecord {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
}

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
) -> Result<UserRecord, sqlx::Error> {
    let user = sqlx::query_as!(
        UserRecord,
        r#"
            INSERT INTO users (
                email,
                password_hash
            )
            VALUES ($1, $2)
            RETURNING
                id,
                email,
                password_hash
        "#,
        email,
        password_hash
    )
    .fetch_one(pool)
    .await?;

    Ok(user)
}

pub async fn find_user_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<UserRecord>, sqlx::Error> {
    let user = sqlx::query_as!(
        UserRecord,
        r#"
            SELECT
                id,
                email,
                password_hash
            FROM users
            WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn find_user_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<UserRecord>, sqlx::Error> {
    let user = sqlx::query_as!(
        UserRecord,
        r#"
            SELECT
                id,
                email,
                password_hash
            FROM users
            WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> PgPool {
        dotenvy::dotenv().ok();

        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for database tests");

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn create_user_inserts_user() {
        let pool = test_pool().await;

        let email = format!("database-test-{}@blackvault.local", Uuid::new_v4());

        let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$test$test";

        let user = create_user(&pool, &email, password_hash)
            .await
            .expect("Failed to create test user");

        assert_eq!(user.email, email);
        assert_eq!(user.password_hash, password_hash);
        assert!(!user.id.is_nil());

        sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
            .execute(&pool)
            .await
            .expect("Failed to clean up test user");
    }

    #[tokio::test]
    async fn duplicate_email_is_rejected() {
        let pool = test_pool().await;

        let email = format!("duplicate-{}@blackvault.local", Uuid::new_v4());

        let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$test$test";

        let first_user = create_user(&pool, &email, password_hash)
            .await
            .expect("Failed to create first test user");

        let duplicate = create_user(&pool, &email, password_hash).await;

        assert!(duplicate.is_err());

        sqlx::query!("DELETE FROM users WHERE id = $1", first_user.id)
            .execute(&pool)
            .await
            .expect("Failed to clean up test user");
    }

    #[tokio::test]
    async fn find_user_by_email_returns_user() {
        let pool = test_pool().await;

        let email = format!("lookup-{}@blackvault.local", Uuid::new_v4());

        let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$test$test";

        let created_user = create_user(&pool, &email, password_hash)
            .await
            .expect("Failed to create lookup test user");

        let found_user = find_user_by_email(&pool, &email)
            .await
            .expect("Failed to find test user")
            .expect("Expected test user to exist");

        assert_eq!(found_user.id, created_user.id);
        assert_eq!(found_user.email, email);
        assert_eq!(found_user.password_hash, password_hash);

        sqlx::query!("DELETE FROM users WHERE id = $1", created_user.id)
            .execute(&pool)
            .await
            .expect("Failed to clean up test user");
    }

    #[tokio::test]
    async fn find_user_by_email_returns_none_for_unknown_email() {
        let pool = test_pool().await;

        let email = format!("missing-{}@blackvault.local", Uuid::new_v4());

        let user = find_user_by_email(&pool, &email)
            .await
            .expect("Failed to query test user");

        assert!(user.is_none());
    }
}