use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{engine::general_purpose, Engine as _};
use black_vault_api::{
    app::create_app, config::Config, crypto::derivation::DerivationParams, database,
    services::jwt::create_token,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn test_pool() -> PgPool {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for API tests");

    database::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

fn test_config() -> Config {
    Config {
        database_url: "postgres://test@localhost/test".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8080,
        jwt_secret: "black-vault-api-test-secret".to_string(),
        jwt_expiration: 3600,
    }
}

fn test_app(pool: PgPool) -> axum::Router {
    create_app(pool, test_config())
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    serde_json::from_slice(&body).expect("Response body was not valid JSON")
}

async fn register_test_user(pool: &PgPool, email: &str, password: &str) -> Uuid {
    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": password
            })
            .to_string(),
        ))
        .expect("Failed to build registration request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute registration request");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;

    Uuid::parse_str(
        body.get("id")
            .and_then(Value::as_str)
            .expect("Registration response did not contain an id"),
    )
    .expect("Registration response contained an invalid user id")
}

fn test_token(user_id: Uuid) -> String {
    let config = test_config();

    create_token(user_id, &config.jwt_secret, config.jwt_expiration)
        .expect("Failed to create test JWT")
}

async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .expect("Failed to clean up test user");
}

fn valid_vault_payload() -> Value {
    let salt = general_purpose::STANDARD.encode([9u8; 16]);

    json!({
        "encrypted_vault_key": general_purpose::STANDARD.encode([7u8; 48]),
        "key_derivation_salt": salt,
        "key_derivation_params": {
            "salt": salt,
            "t_cost": 2,
            "m_cost": 19456,
            "p_cost": 1,
            "output_len": 32
        },
        "encryption_nonce": general_purpose::STANDARD.encode([5u8; 12])
    })
}

async fn setup_test_vault(pool: &PgPool, user_id: Uuid) {
    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/vault/setup")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(valid_vault_payload().to_string()))
        .expect("Failed to build vault setup request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute vault setup request");

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn register_endpoint_creates_user() {
    let pool = test_pool().await;

    let email = format!("api-register-{}@blackvault.local", Uuid::new_v4());
    let password = "SuperSecure123Register";

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": password
            })
            .to_string(),
        ))
        .expect("Failed to build registration request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute registration request");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;

    assert_eq!(
        body.get("email").and_then(Value::as_str),
        Some(email.as_str())
    );

    assert!(body.get("id").and_then(Value::as_str).is_some());
    assert!(body.get("password").is_none());
    assert!(body.get("password_hash").is_none());

    let user_id = Uuid::parse_str(
        body.get("id")
            .and_then(Value::as_str)
            .expect("Registration response did not contain an id"),
    )
    .expect("Registration response contained an invalid user id");

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn register_endpoint_normalises_email() {
    let pool = test_pool().await;

    let email = format!("  API-NORMALISE-{}@BLACKVAULT.LOCAL  ", Uuid::new_v4());
    let expected_email = email.trim().to_lowercase();

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": "SuperSecure123Register"
            })
            .to_string(),
        ))
        .expect("Failed to build registration request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute registration request");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;

    assert_eq!(
        body.get("email").and_then(Value::as_str),
        Some(expected_email.as_str())
    );

    let user_id = Uuid::parse_str(
        body.get("id")
            .and_then(Value::as_str)
            .expect("Registration response did not contain an id"),
    )
    .expect("Registration response contained an invalid user id");

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn register_endpoint_rejects_weak_password() {
    let pool = test_pool().await;

    let email = format!("api-weak-password-{}@blackvault.local", Uuid::new_v4());

    let app = test_app(pool);

    let request = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": "weak"
            })
            .to_string(),
        ))
        .expect("Failed to build registration request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute registration request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Password must be at least 12 characters")
    );
}

#[tokio::test]
async fn register_endpoint_rejects_duplicate_email() {
    let pool = test_pool().await;

    let email = format!("api-duplicate-{}@blackvault.local", Uuid::new_v4());
    let password = "SuperSecure123Register";

    let user_id = register_test_user(&pool, &email, password).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": password
            })
            .to_string(),
        ))
        .expect("Failed to build registration request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute registration request");

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("An account with that email already exists")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn login_endpoint_returns_jwt_for_valid_credentials() {
    let pool = test_pool().await;

    let email = format!("api-login-{}@blackvault.local", Uuid::new_v4());
    let password = "SuperSecure123Login";

    let user_id = register_test_user(&pool, &email, password).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": password
            })
            .to_string(),
        ))
        .expect("Failed to build login request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute login request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;

    assert_eq!(
        body.get("id").and_then(Value::as_str),
        Some(user_id.to_string().as_str())
    );

    assert_eq!(
        body.get("email").and_then(Value::as_str),
        Some(email.as_str())
    );

    let token = body
        .get("token")
        .and_then(Value::as_str)
        .expect("Login response did not contain a token");

    assert!(!token.is_empty());

    let config = test_config();

    let claims = black_vault_api::services::jwt::verify_token(token, &config.jwt_secret)
        .expect("Returned token could not be verified");

    assert_eq!(claims.sub, user_id.to_string());

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn login_endpoint_rejects_wrong_password() {
    let pool = test_pool().await;

    let email = format!("api-login-failure-{}@blackvault.local", Uuid::new_v4());
    let password = "SuperSecure123Login";

    let user_id = register_test_user(&pool, &email, password).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": "DefinitelyWrong123"
            })
            .to_string(),
        ))
        .expect("Failed to build login request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute login request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Invalid email or password")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn protected_endpoint_rejects_missing_authorization() {
    let pool = test_pool().await;

    let app = test_app(pool);

    let request = Request::builder()
        .method("GET")
        .uri("/user/me")
        .body(Body::empty())
        .expect("Failed to build protected request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute protected request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_endpoint_rejects_invalid_jwt() {
    let pool = test_pool().await;

    let app = test_app(pool);

    let request = Request::builder()
        .method("GET")
        .uri("/user/me")
        .header("authorization", "Bearer invalid.jwt.token")
        .body(Body::empty())
        .expect("Failed to build protected request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute protected request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_endpoint_accepts_valid_jwt() {
    let pool = test_pool().await;

    let email = format!("api-protected-{}@blackvault.local", Uuid::new_v4());
    let password = "SuperSecure123Protected";

    let user_id = register_test_user(&pool, &email, password).await;

    let token = test_token(user_id);

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/user/me")
        .header("authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .expect("Failed to build protected request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute protected request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;

    assert_eq!(
        body.get("id").and_then(Value::as_str),
        Some(user_id.to_string().as_str())
    );

    assert_eq!(
        body.get("email").and_then(Value::as_str),
        Some(email.as_str())
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn vault_key_endpoint_rejects_missing_authorization() {
    let pool = test_pool().await;

    let app = test_app(pool);

    let request = Request::builder()
        .method("GET")
        .uri("/vault/key")
        .body(Body::empty())
        .expect("Failed to build vault key request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute vault key request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn vault_key_endpoint_returns_not_found_when_vault_is_not_set_up() {
    let pool = test_pool().await;

    let email = format!("api-no-vault-{}@blackvault.local", Uuid::new_v4());
    let password = "SuperSecure123Vault";

    let user_id = register_test_user(&pool, &email, password).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/vault/key")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::empty())
        .expect("Failed to build vault key request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute vault key request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Vault not set up")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn vault_setup_creates_vault_for_authenticated_user() {
    let pool = test_pool().await;

    let email = format!("api-vault-setup-{}@blackvault.local", Uuid::new_v4());
    let password = "SuperSecure123Vault";

    let user_id = register_test_user(&pool, &email, password).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/vault/setup")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(valid_vault_payload().to_string()))
        .expect("Failed to build vault setup request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute vault setup request");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;

    assert_eq!(body.get("success").and_then(Value::as_bool), Some(true));

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn vault_key_endpoint_returns_authenticated_users_vault() {
    let pool = test_pool().await;

    let email = format!("api-vault-retrieve-{}@blackvault.local", Uuid::new_v4());
    let password = "SuperSecure123Vault";

    let user_id = register_test_user(&pool, &email, password).await;

    setup_test_vault(&pool, user_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/vault/key")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::empty())
        .expect("Failed to build vault key request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute vault key request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;

    assert_eq!(
        body.get("encrypted_vault_key").and_then(Value::as_str),
        Some(general_purpose::STANDARD.encode([7u8; 48]).as_str())
    );

    assert_eq!(
        body.get("key_derivation_salt").and_then(Value::as_str),
        Some(general_purpose::STANDARD.encode([9u8; 16]).as_str())
    );

    assert_eq!(
        body.get("encryption_nonce").and_then(Value::as_str),
        Some(general_purpose::STANDARD.encode([5u8; 12]).as_str())
    );

    assert_eq!(
        body.get("encryption_version").and_then(Value::as_str),
        Some("1")
    );

    let params = body
        .get("key_derivation_params")
        .expect("Response did not contain key derivation parameters");

    let expected_params = DerivationParams {
        salt: general_purpose::STANDARD.encode([9u8; 16]),
        t_cost: 2,
        m_cost: 19456,
        p_cost: 1,
        output_len: 32,
    };

    let actual_params: DerivationParams =
        serde_json::from_value(params.clone()).expect("Invalid derivation parameters");

    assert_eq!(actual_params.salt, expected_params.salt);
    assert_eq!(actual_params.t_cost, expected_params.t_cost);
    assert_eq!(actual_params.m_cost, expected_params.m_cost);
    assert_eq!(actual_params.p_cost, expected_params.p_cost);
    assert_eq!(actual_params.output_len, expected_params.output_len);

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn vault_setup_rejects_duplicate_setup() {
    let pool = test_pool().await;

    let email = format!("api-vault-duplicate-{}@blackvault.local", Uuid::new_v4());
    let password = "SuperSecure123Vault";

    let user_id = register_test_user(&pool, &email, password).await;

    setup_test_vault(&pool, user_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/vault/setup")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(valid_vault_payload().to_string()))
        .expect("Failed to build duplicate vault setup request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute duplicate vault setup request");

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Vault is already set up")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn vault_key_cannot_be_accessed_by_another_user() {
    let pool = test_pool().await;

    let user_a_email = format!("api-vault-owner-{}@blackvault.local", Uuid::new_v4());
    let user_b_email = format!("api-vault-attacker-{}@blackvault.local", Uuid::new_v4());

    let password = "SuperSecure123Vault";

    let user_a_id = register_test_user(&pool, &user_a_email, password).await;
    let user_b_id = register_test_user(&pool, &user_b_email, password).await;

    setup_test_vault(&pool, user_a_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/vault/key")
        .header("authorization", format!("Bearer {}", test_token(user_b_id)))
        .body(Body::empty())
        .expect("Failed to build cross-user vault request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute cross-user vault request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Vault not set up")
    );

    cleanup_user(&pool, user_a_id).await;
    cleanup_user(&pool, user_b_id).await;
}

#[tokio::test]
async fn vault_setup_rejects_invalid_encryption_nonce() {
    let pool = test_pool().await;

    let email = format!(
        "api-vault-invalid-nonce-{}@blackvault.local",
        Uuid::new_v4()
    );
    let password = "SuperSecure123Vault";

    let user_id = register_test_user(&pool, &email, password).await;

    let mut payload = valid_vault_payload();
    payload["encryption_nonce"] = json!(general_purpose::STANDARD.encode([5u8; 11]));

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/vault/setup")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(payload.to_string()))
        .expect("Failed to build invalid nonce request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute invalid nonce request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("encryption_nonce has invalid length")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn vault_setup_rejects_mismatched_derivation_salt() {
    let pool = test_pool().await;

    let email = format!(
        "api-vault-salt-mismatch-{}@blackvault.local",
        Uuid::new_v4()
    );
    let password = "SuperSecure123Vault";

    let user_id = register_test_user(&pool, &email, password).await;

    let mut payload = valid_vault_payload();

    payload["key_derivation_params"]["salt"] = json!(general_purpose::STANDARD.encode([8u8; 16]));

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/vault/setup")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(payload.to_string()))
        .expect("Failed to build salt mismatch request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute salt mismatch request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Key derivation salt does not match parameters")
    );

    cleanup_user(&pool, user_id).await;
}
