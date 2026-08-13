use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash,
        PasswordHasher,
        PasswordVerifier,
        SaltString,
    },
    Argon2,
};

use sqlx::PgPool;

use crate::database::users::{
    create_user,
    find_user_by_email,
};

use crate::models::user::UserCredentials;


pub fn hash_password(
    password: &str,
) -> Result<String, String> {

    let salt =
        SaltString::generate(
            &mut OsRng
        );

    let argon2 =
        Argon2::default();

    argon2
        .hash_password(
            password.as_bytes(),
            &salt,
        )
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}


pub fn verify_password(
    password: &str,
    stored_hash: &str,
) -> Result<bool, String> {

    let parsed_hash =
        PasswordHash::new(
            stored_hash
        )
        .map_err(|error| error.to_string())?;

    Ok(
        Argon2::default()
            .verify_password(
                password.as_bytes(),
                &parsed_hash,
            )
            .is_ok()
    )
}


pub async fn register_user(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<UserCredentials, String> {

    if password.len() < 12 {
        return Err(
            "Password must be at least 12 characters"
                .to_string()
        );
    }

    let email =
        email
            .trim()
            .to_lowercase();

    if email.is_empty() {
        return Err(
            "Email is required"
                .to_string()
        );
    }

    let password_hash =
        hash_password(password)?;

    let user =
        create_user(
            pool,
            &email,
            &password_hash,
        )
        .await
        .map_err(|error| {

            if let sqlx::Error::Database(
                database_error
            ) = &error {

                if database_error.constraint()
                    == Some("users_email_key")
                {
                    return "An account with that email already exists"
                        .to_string();
                }
            }

            tracing::error!(
                error = ?error,
                "Failed to create user"
            );

            "Unable to create account"
                .to_string()
        })?;

    Ok(
        UserCredentials {
            id: user.id,
            email: user.email,
            password_hash: user.password_hash,
        }
    )
}


pub async fn login_user(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<UserCredentials, String> {

    let email =
        email
            .trim()
            .to_lowercase();

    if email.is_empty() {
        return Err(
            "Email is required"
                .to_string()
        );
    }

    if password.is_empty() {
        return Err(
            "Password is required"
                .to_string()
        );
    }

    let user =
        find_user_by_email(
            pool,
            &email,
        )
        .await
        .map_err(|error| {

            tracing::error!(
                error = ?error,
                "Failed to find user during login"
            );

            "Unable to authenticate".to_string()
        })?;

    let user =
        user.ok_or_else(|| {
            "Invalid email or password"
                .to_string()
        })?;

    let password_valid =
        verify_password(
            password,
            &user.password_hash,
        )
        .map_err(|error| {

            tracing::error!(
                error = %error,
                "Failed to verify password"
            );

            "Unable to authenticate".to_string()
        })?;

    if !password_valid {
        return Err(
            "Invalid email or password"
                .to_string()
        );
    }

    Ok(
        UserCredentials {
            id: user.id,
            email: user.email,
            password_hash: user.password_hash,
        }
    )
}


#[cfg(test)]
mod tests {

    use super::*;

    use uuid::Uuid;


    async fn test_pool() -> PgPool {

        dotenvy::dotenv().ok();

        let database_url =
            std::env::var("DATABASE_URL")
                .expect(
                    "DATABASE_URL must be set for authentication tests"
                );

        PgPool::connect(&database_url)
            .await
            .expect(
                "Failed to connect to test database"
            )
    }


    #[test]
    fn password_hash_works() {

        let password =
            "super_secret_password";

        let hash =
            hash_password(password)
                .unwrap();

        assert!(
            verify_password(
                password,
                &hash,
            )
            .unwrap()
        );
    }


    #[test]
    fn password_hash_rejects_wrong_password() {

        let password =
            "super_secret_password";

        let wrong_password =
            "incorrect_password";

        let hash =
            hash_password(password)
                .unwrap();

        assert!(
            !verify_password(
                wrong_password,
                &hash,
            )
            .unwrap()
        );
    }


    #[tokio::test]
    async fn register_user_creates_hashed_user() {

        let pool =
            test_pool().await;

        let email =
            format!(
                "register-{}@blackvault.local",
                Uuid::new_v4()
            );

        let password =
            "super_secure_test_password";

        let user =
            register_user(
                &pool,
                &email,
                password,
            )
            .await
            .expect(
                "Failed to register test user"
            );

        assert_eq!(
            user.email,
            email
        );

        assert!(
            !user.id.is_nil()
        );

        assert_ne!(
            user.password_hash,
            password
        );

        assert!(
            verify_password(
                password,
                &user.password_hash,
            )
            .expect(
                "Failed to verify password"
            )
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
    async fn register_user_normalises_email() {

        let pool =
            test_pool().await;

        let email =
            format!(
                "NORMALISE-{}@BLACKVAULT.LOCAL",
                Uuid::new_v4()
            );

        let expected_email =
            email
                .trim()
                .to_lowercase();

        let user =
            register_user(
                &pool,
                &email,
                "super_secure_test_password",
            )
            .await
            .expect(
                "Failed to register test user"
            );

        assert_eq!(
            user.email,
            expected_email
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
    async fn register_user_rejects_short_password() {

        let pool =
            test_pool().await;

        let result =
            register_user(
                &pool,
                "short-password@blackvault.local",
                "short",
            )
            .await;

        match result {
            Err(error) => {
                assert_eq!(
                    error,
                    "Password must be at least 12 characters"
                );
            }
            Ok(_) => {
                panic!(
                    "Expected short password to be rejected"
                );
            }
        }
    }


    #[tokio::test]
    async fn login_user_accepts_correct_password() {

        let pool =
            test_pool().await;

        let email =
            format!(
                "login-success-{}@blackvault.local",
                Uuid::new_v4()
            );

        let password =
            "super_secure_login_password";

        let registered_user =
            register_user(
                &pool,
                &email,
                password,
            )
            .await
            .expect(
                "Failed to register login test user"
            );

        let logged_in_user =
            login_user(
                &pool,
                &email,
                password,
            )
            .await
            .expect(
                "Expected login to succeed"
            );

        assert_eq!(
            logged_in_user.id,
            registered_user.id
        );

        assert_eq!(
            logged_in_user.email,
            email
        );

        sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            registered_user.id
        )
        .execute(&pool)
        .await
        .expect(
            "Failed to clean up login test user"
        );
    }


    #[tokio::test]
    async fn login_user_rejects_wrong_password() {

        let pool =
            test_pool().await;

        let email =
            format!(
                "login-failure-{}@blackvault.local",
                Uuid::new_v4()
            );

        let password =
            "super_secure_login_password";

        let registered_user =
            register_user(
                &pool,
                &email,
                password,
            )
            .await
            .expect(
                "Failed to register login test user"
            );

        let result =
            login_user(
                &pool,
                &email,
                "definitely-wrong-password",
            )
            .await;

        match result {
            Err(error) => {
                assert_eq!(
                    error,
                    "Invalid email or password"
                );
            }
            Ok(_) => {
                panic!(
                    "Expected login with wrong password to fail"
                );
            }
        }

        sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            registered_user.id
        )
        .execute(&pool)
        .await
        .expect(
            "Failed to clean up login test user"
        );
    }


    #[tokio::test]
    async fn login_user_rejects_unknown_email() {

        let pool =
            test_pool().await;

        let email =
            format!(
                "does-not-exist-{}@blackvault.local",
                Uuid::new_v4()
            );

        let result =
            login_user(
                &pool,
                &email,
                "some_password",
            )
            .await;

        match result {
            Err(error) => {
                assert_eq!(
                    error,
                    "Invalid email or password"
                );
            }
            Ok(_) => {
                panic!(
                    "Expected login with unknown email to fail"
                );
            }
        }
    }


    #[tokio::test]
    async fn login_user_normalises_email() {

        let pool =
            test_pool().await;

        let email =
            format!(
                "login-normalise-{}@blackvault.local",
                Uuid::new_v4()
            );

        let password =
            "super_secure_login_password";

        let registered_user =
            register_user(
                &pool,
                &email,
                password,
            )
            .await
            .expect(
                "Failed to register login test user"
            );

        let input_email =
            format!(
                "  {}  ",
                email.to_uppercase()
            );

        let logged_in_user =
            login_user(
                &pool,
                &input_email,
                password,
            )
            .await
            .expect(
                "Expected normalised login to succeed"
            );

        assert_eq!(
            logged_in_user.id,
            registered_user.id
        );

        sqlx::query!(
            "DELETE FROM users WHERE id = $1",
            registered_user.id
        )
        .execute(&pool)
        .await
        .expect(
            "Failed to clean up login test user"
        );
    }
}