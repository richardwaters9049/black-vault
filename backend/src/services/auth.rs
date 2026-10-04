use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use sqlx::PgPool;

use crate::config::Config;

use crate::database::users::{create_user, find_user_by_email};

use crate::models::user::UserCredentials;

use crate::services::jwt::create_token;

// HTTP status mapping uses variants, so changing a message cannot change its meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    EmailRequired,
    InvalidEmail,
    PasswordRequired,
    PasswordTooShort,
    UppercaseRequired,
    LowercaseRequired,
    DigitRequired,
    EmailTaken,
    InvalidCredentials,
    Unavailable,
}

impl AuthError {
    pub fn message(self) -> &'static str {
        match self {
            Self::EmailRequired => "Email is required",
            Self::InvalidEmail => "Invalid email format",
            Self::PasswordRequired => "Password is required",
            Self::PasswordTooShort => "Password must be at least 12 characters",
            Self::UppercaseRequired => "Password must contain at least one uppercase letter",
            Self::LowercaseRequired => "Password must contain at least one lowercase letter",
            Self::DigitRequired => "Password must contain at least one digit",
            Self::EmailTaken => "An account with that email already exists",
            Self::InvalidCredentials => "Invalid email or password",
            Self::Unavailable => "Internal server error",
        }
    }
}

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

pub fn verify_password(password: &str, stored_hash: &str) -> Result<bool, String> {
    let parsed_hash = PasswordHash::new(stored_hash).map_err(|error| error.to_string())?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn validate_email(email: &str) -> Result<(), AuthError> {
    let email = email.trim().to_lowercase();

    if email.is_empty() {
        return Err(AuthError::EmailRequired);
    }

    let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .map_err(|_| AuthError::Unavailable)?;

    if !email_regex.is_match(&email) {
        return Err(AuthError::InvalidEmail);
    }

    Ok(())
}

fn validate_password(password: &str) -> Result<(), AuthError> {
    if password.len() < 12 {
        return Err(AuthError::PasswordTooShort);
    }

    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(AuthError::UppercaseRequired);
    }

    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(AuthError::LowercaseRequired);
    }

    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(AuthError::DigitRequired);
    }

    Ok(())
}

pub async fn register_user(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<UserCredentials, AuthError> {
    validate_email(email)?;
    validate_password(password)?;

    let email = email.trim().to_lowercase();

    let password_hash = hash_password(password).map_err(|_| AuthError::Unavailable)?;

    let user = create_user(pool, &email, &password_hash)
        .await
        .map_err(|error| {
            if let sqlx::Error::Database(database_error) = &error {
                if database_error.constraint() == Some("users_email_key") {
                    return AuthError::EmailTaken;
                }
            }

            tracing::error!("Failed to create user");

            AuthError::Unavailable
        })?;

    Ok(UserCredentials {
        id: user.id,
        email: user.email,
        password_hash: user.password_hash,
    })
}

pub async fn login_user(
    pool: &PgPool,
    config: &Config,
    email: &str,
    password: &str,
) -> Result<(UserCredentials, String), AuthError> {
    let email = email.trim().to_lowercase();

    if email.is_empty() {
        return Err(AuthError::EmailRequired);
    }

    if password.is_empty() {
        return Err(AuthError::PasswordRequired);
    }

    let user = find_user_by_email(pool, &email).await.map_err(|_| {
        tracing::error!("Failed to find user during login");

        AuthError::Unavailable
    })?;

    let user = user.ok_or(AuthError::InvalidCredentials)?;

    let password_valid = verify_password(password, &user.password_hash).map_err(|_| {
        tracing::error!("Failed to verify password");

        AuthError::Unavailable
    })?;

    if !password_valid {
        return Err(AuthError::InvalidCredentials);
    }

    let token = create_token(user.id, &config.jwt_secret, config.jwt_expiration).map_err(|_| {
        tracing::error!("Failed to create authentication token");

        AuthError::Unavailable
    })?;

    let credentials = UserCredentials {
        id: user.id,
        email: user.email,
        password_hash: user.password_hash,
    };

    Ok((credentials, token))
}

#[cfg(test)]
mod tests {
    use super::*;

    use uuid::Uuid;

    async fn test_pool() -> PgPool {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for authentication tests");

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    fn test_config() -> Config {
        Config {
            database_url: "postgres://test@localhost/test".to_string(),

            host: "127.0.0.1".to_string(),

            port: 8080,

            jwt_secret: "test-jwt-secret".to_string(),

            jwt_expiration: 3600,
        }
    }

    #[test]
    fn password_hash_works() {
        let password = "super_secret_password";

        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash,).unwrap());
    }

    #[test]
    fn password_hash_rejects_wrong_password() {
        let password = "super_secret_password";

        let wrong_password = "incorrect_password";

        let hash = hash_password(password).unwrap();

        assert!(!verify_password(wrong_password, &hash,).unwrap());
    }

    #[tokio::test]
    async fn register_user_creates_hashed_user() {
        let pool = test_pool().await;

        let email = format!("register-{}@blackvault.local", Uuid::new_v4());

        let password = "SuperSecure123Test";

        let user = register_user(&pool, &email, password)
            .await
            .expect("Failed to register test user");

        assert_eq!(user.email, email);

        assert!(!user.id.is_nil());

        assert_ne!(user.password_hash, password);

        assert!(verify_password(password, &user.password_hash,).expect("Failed to verify password"));

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .expect("Failed to clean up test user");
    }

    #[tokio::test]
    async fn register_user_normalises_email() {
        let pool = test_pool().await;

        let email = format!("NORMALISE-{}@BLACKVAULT.LOCAL", Uuid::new_v4());

        let expected_email = email.trim().to_lowercase();

        let user = register_user(&pool, &email, "SuperSecure123Test")
            .await
            .expect("Failed to register test user");

        assert_eq!(user.email, expected_email);

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .expect("Failed to clean up test user");
    }

    #[tokio::test]
    async fn register_user_rejects_short_password() {
        let pool = test_pool().await;

        let result = register_user(&pool, "short-password@blackvault.local", "short").await;

        match result {
            Err(error) => {
                assert_eq!(error.message(), "Password must be at least 12 characters");
            }

            Ok(_) => {
                panic!("Expected short password to be rejected");
            }
        }
    }

    #[tokio::test]
    async fn login_user_accepts_correct_password() {
        let pool = test_pool().await;

        let config = test_config();

        let email = format!("login-success-{}@blackvault.local", Uuid::new_v4());

        let password = "SuperSecure123Login";

        let registered_user = register_user(&pool, &email, password)
            .await
            .expect("Failed to register login test user");

        let (logged_in_user, token) = login_user(&pool, &config, &email, password)
            .await
            .expect("Expected login to succeed");

        assert_eq!(logged_in_user.id, registered_user.id);

        assert_eq!(logged_in_user.email, email);

        assert!(!token.is_empty());

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(registered_user.id)
            .execute(&pool)
            .await
            .expect("Failed to clean up login test user");
    }

    #[tokio::test]
    async fn login_user_rejects_wrong_password() {
        let pool = test_pool().await;

        let config = test_config();

        let email = format!("login-failure-{}@blackvault.local", Uuid::new_v4());

        let password = "SuperSecure123Login";

        let registered_user = register_user(&pool, &email, password)
            .await
            .expect("Failed to register login test user");

        let result = login_user(&pool, &config, &email, "definitely-wrong-password").await;

        match result {
            Err(error) => {
                assert_eq!(error.message(), "Invalid email or password");
            }

            Ok(_) => {
                panic!("Expected login with wrong password to fail");
            }
        }

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(registered_user.id)
            .execute(&pool)
            .await
            .expect("Failed to clean up login test user");
    }

    #[tokio::test]
    async fn login_user_rejects_unknown_email() {
        let pool = test_pool().await;

        let config = test_config();

        let email = format!("does-not-exist-{}@blackvault.local", Uuid::new_v4());

        let result = login_user(&pool, &config, &email, "some_password").await;

        match result {
            Err(error) => {
                assert_eq!(error.message(), "Invalid email or password");
            }

            Ok(_) => {
                panic!("Expected login with unknown email to fail");
            }
        }
    }

    #[tokio::test]
    async fn login_user_normalises_email() {
        let pool = test_pool().await;

        let config = test_config();

        let email = format!("login-normalise-{}@blackvault.local", Uuid::new_v4());

        let password = "SuperSecure123Login";

        let registered_user = register_user(&pool, &email, password)
            .await
            .expect("Failed to register login test user");

        let input_email = format!("  {}  ", email.to_uppercase());

        let (logged_in_user, token) = login_user(&pool, &config, &input_email, password)
            .await
            .expect("Expected normalised login to succeed");

        assert_eq!(logged_in_user.id, registered_user.id);

        assert_eq!(logged_in_user.email, email);

        assert!(!token.is_empty());

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(registered_user.id)
            .execute(&pool)
            .await
            .expect("Failed to clean up login test user");
    }
}
