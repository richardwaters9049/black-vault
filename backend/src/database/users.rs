use sqlx::PgPool;

use crate::models::user::UserCredentials;


pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
) -> Result<UserCredentials, sqlx::Error> {

    let user = sqlx::query_as!(
        UserCredentials,
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


#[cfg(test)]
mod tests {

    use super::*;

    use uuid::Uuid;


    async fn test_pool() -> PgPool {

        let database_url =
            std::env::var("DATABASE_URL")
                .expect(
                    "DATABASE_URL must be set for database tests"
                );

        PgPool::connect(&database_url)
            .await
            .expect(
                "Failed to connect to test database"
            )
    }


    #[tokio::test]
    async fn create_user_inserts_user() {

        let pool =
            test_pool().await;

        let email =
            format!(
                "test-{}@blackvault.local",
                Uuid::new_v4()
            );

        let password_hash =
            "$argon2id$v=19$m=19456,t=2,p=1$test$test";

        let user =
            create_user(
                &pool,
                &email,
                password_hash,
            )
            .await
            .expect(
                "Failed to create test user"
            );

        assert_eq!(
            user.email,
            email
        );

        assert_eq!(
            user.password_hash,
            password_hash
        );

        assert!(
            !user.id.is_nil()
        );

        sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            user.id
        )
        .execute(&pool)
        .await
        .expect(
            "Failed to clean up test user"
        );
    }


    #[tokio::test]
    async fn duplicate_email_is_rejected() {

        let pool =
            test_pool().await;

        let email =
            format!(
                "duplicate-{}@blackvault.local",
                Uuid::new_v4()
            );

        let password_hash =
            "$argon2id$v=19$m=19456,t=2,p=1$test$test";

        let first =
            create_user(
                &pool,
                &email,
                password_hash,
            )
            .await;

        assert!(
            first.is_ok()
        );

        let second =
            create_user(
                &pool,
                &email,
                password_hash,
            )
            .await;

        assert!(
            second.is_err()
        );

        if let Ok(user) = first {

            sqlx::query!(
                "DELETE FROM users WHERE id = $1",
                user.id
            )
            .execute(&pool)
            .await
            .expect(
                "Failed to clean up test user"
            );
        }
    }
}