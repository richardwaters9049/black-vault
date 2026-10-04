use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{engine::general_purpose, Engine as _};
use black_vault_api::{app::create_app, config::Config, services::jwt::create_token};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const AES_GCM_TAG_LENGTH: usize = 16;

async fn test_pool() -> PgPool {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for API tests");

    PgPool::connect(&database_url)
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

fn test_token(user_id: Uuid) -> String {
    let config = test_config();

    create_token(user_id, &config.jwt_secret, config.jwt_expiration)
        .expect("Failed to create test JWT")
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");

    serde_json::from_slice(&body).expect("Response body was not valid JSON")
}

async fn create_test_user(pool: &PgPool) -> Uuid {
    let email = format!("api-file-{}@blackvault.local", Uuid::new_v4());

    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$test$test";

    // Test setup uses typed runtime binding; production queries keep their offline metadata.
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
    )
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
    .expect("Failed to create test user")
}

async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .expect("Failed to clean up test user");
}

fn valid_file_payload() -> Value {
    json!({
        "encrypted_filename": general_purpose::STANDARD.encode(b"encrypted-test-file"),
        "encrypted_file_key": general_purpose::STANDARD.encode([7u8; 32]),
        "encryption_nonce": general_purpose::STANDARD.encode([5u8; 12]),
        "file_size": 12345
    })
}

async fn create_file(pool: &PgPool, user_id: Uuid) -> Uuid {
    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/files")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(valid_file_payload().to_string()))
        .expect("Failed to build file creation request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute file creation request");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;

    Uuid::parse_str(
        body.get("id")
            .and_then(Value::as_str)
            .expect("File response did not contain an id"),
    )
    .expect("File response contained an invalid id")
}

fn encrypted_data_for_file_size(file_size: usize) -> Vec<u8> {
    vec![42u8; file_size + AES_GCM_TAG_LENGTH]
}

#[tokio::test]
async fn create_file_requires_authentication() {
    let pool = test_pool().await;

    let app = test_app(pool);

    let request = Request::builder()
        .method("POST")
        .uri("/files")
        .header("content-type", "application/json")
        .body(Body::from(valid_file_payload().to_string()))
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_file_creates_encrypted_metadata() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/files")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(valid_file_payload().to_string()))
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;

    assert!(body.get("id").and_then(Value::as_str).is_some());

    assert_eq!(
        body.get("encrypted_filename").and_then(Value::as_str),
        Some(
            general_purpose::STANDARD
                .encode(b"encrypted-test-file")
                .as_str()
        )
    );

    assert_eq!(
        body.get("encrypted_file_key").and_then(Value::as_str),
        Some(general_purpose::STANDARD.encode([7u8; 32]).as_str())
    );

    assert_eq!(
        body.get("encryption_nonce").and_then(Value::as_str),
        Some(general_purpose::STANDARD.encode([5u8; 12]).as_str())
    );

    assert_eq!(
        body.get("encryption_version").and_then(Value::as_str),
        Some("1")
    );

    assert_eq!(body.get("file_size").and_then(Value::as_i64), Some(12345));

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn create_file_rejects_negative_file_size() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;

    let mut payload = valid_file_payload();
    payload["file_size"] = json!(-1);

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/files")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(payload.to_string()))
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("File size cannot be negative")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn create_file_rejects_invalid_key() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;

    let mut payload = valid_file_payload();
    payload["encrypted_file_key"] = json!("not-valid-base64");

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/files")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(payload.to_string()))
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Encrypted file key must be valid Base64")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn create_file_rejects_invalid_key_length() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;

    let mut payload = valid_file_payload();
    payload["encrypted_file_key"] = json!(general_purpose::STANDARD.encode([7u8; 31]));

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/files")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(payload.to_string()))
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Encrypted file key has invalid length")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn create_file_rejects_invalid_nonce_length() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;

    let mut payload = valid_file_payload();
    payload["encryption_nonce"] = json!(general_purpose::STANDARD.encode([5u8; 11]));

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/files")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(payload.to_string()))
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Encryption nonce has invalid length")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn list_files_returns_only_authenticated_users_files() {
    let pool = test_pool().await;

    let user_a = create_test_user(&pool).await;
    let user_b = create_test_user(&pool).await;

    create_file(&pool, user_a).await;
    create_file(&pool, user_b).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/files")
        .header("authorization", format!("Bearer {}", test_token(user_a)))
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;

    let files = body
        .get("files")
        .and_then(Value::as_array)
        .expect("Response did not contain files");

    assert_eq!(files.len(), 1);

    cleanup_user(&pool, user_a).await;
    cleanup_user(&pool, user_b).await;
}

#[tokio::test]
async fn get_file_returns_owned_file() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;
    let file_id = create_file(&pool, user_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/files/{}", file_id))
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;

    assert_eq!(
        body.get("id").and_then(Value::as_str),
        Some(file_id.to_string().as_str())
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn get_file_rejects_another_users_file() {
    let pool = test_pool().await;

    let owner_id = create_test_user(&pool).await;
    let attacker_id = create_test_user(&pool).await;

    let file_id = create_file(&pool, owner_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/files/{}", file_id))
        .header(
            "authorization",
            format!("Bearer {}", test_token(attacker_id)),
        )
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    cleanup_user(&pool, owner_id).await;
    cleanup_user(&pool, attacker_id).await;
}

#[tokio::test]
async fn upload_encrypted_data_requires_authentication() {
    let pool = test_pool().await;

    let app = test_app(pool);

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/files/{}/data", Uuid::new_v4()))
        .body(Body::from(vec![1u8; 16]))
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn upload_encrypted_data_stores_payload() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;

    let mut payload = valid_file_payload();
    payload["file_size"] = json!(32);

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/files")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(payload.to_string()))
        .expect("Failed to build request");

    let response = app.oneshot(request).await.expect("Failed to create file");

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;

    let file_id = Uuid::parse_str(
        body.get("id")
            .and_then(Value::as_str)
            .expect("Missing file id"),
    )
    .expect("Invalid file id");

    let encrypted_data = encrypted_data_for_file_size(32);

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/files/{}/data", file_id))
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(encrypted_data.clone()))
        .expect("Failed to build upload request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to upload encrypted data");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/files/{}/data", file_id))
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::empty())
        .expect("Failed to build download request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to download encrypted data");

    assert_eq!(response.status(), StatusCode::OK);

    let returned_data = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read encrypted data");

    assert_eq!(returned_data.as_ref(), encrypted_data.as_slice());

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn upload_encrypted_data_rejects_wrong_size() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;

    let file_id = create_file(&pool, user_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/files/{}/data", file_id))
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(vec![1u8; 16]))
        .expect("Failed to build upload request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute upload request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response_json(response).await;

    assert_eq!(
        body.get("error").and_then(Value::as_str),
        Some("Encrypted data size does not match file size")
    );

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn upload_encrypted_data_rejects_another_users_file() {
    let pool = test_pool().await;

    let owner_id = create_test_user(&pool).await;
    let attacker_id = create_test_user(&pool).await;

    let file_id = create_file(&pool, owner_id).await;

    let encrypted_data = encrypted_data_for_file_size(12345);

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/files/{}/data", file_id))
        .header(
            "authorization",
            format!("Bearer {}", test_token(attacker_id)),
        )
        .body(Body::from(encrypted_data))
        .expect("Failed to build upload request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute upload request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    cleanup_user(&pool, owner_id).await;
    cleanup_user(&pool, attacker_id).await;
}

#[tokio::test]
async fn download_encrypted_data_requires_authentication() {
    let pool = test_pool().await;

    let app = test_app(pool);

    let request = Request::builder()
        .method("GET")
        .uri(format!("/files/{}/data", Uuid::new_v4()))
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn download_encrypted_data_rejects_another_users_file() {
    let pool = test_pool().await;

    let owner_id = create_test_user(&pool).await;
    let attacker_id = create_test_user(&pool).await;

    let file_id = create_file(&pool, owner_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/files/{}/data", file_id))
        .header(
            "authorization",
            format!("Bearer {}", test_token(attacker_id)),
        )
        .body(Body::empty())
        .expect("Failed to build download request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute download request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    cleanup_user(&pool, owner_id).await;
    cleanup_user(&pool, attacker_id).await;
}

#[tokio::test]
async fn download_encrypted_data_returns_not_found_when_payload_missing() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;
    let file_id = create_file(&pool, user_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/files/{}/data", file_id))
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::empty())
        .expect("Failed to build download request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute download request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn delete_file_removes_owned_file() {
    let pool = test_pool().await;

    let user_id = create_test_user(&pool).await;
    let file_id = create_file(&pool, user_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/files/{}", file_id))
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/files/{}", file_id))
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    cleanup_user(&pool, user_id).await;
}

#[tokio::test]
async fn delete_file_rejects_another_users_file() {
    let pool = test_pool().await;

    let owner_id = create_test_user(&pool).await;
    let attacker_id = create_test_user(&pool).await;

    let file_id = create_file(&pool, owner_id).await;

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/files/{}", file_id))
        .header(
            "authorization",
            format!("Bearer {}", test_token(attacker_id)),
        )
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let app = test_app(pool.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/files/{}", file_id))
        .header("authorization", format!("Bearer {}", test_token(owner_id)))
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    cleanup_user(&pool, owner_id).await;
    cleanup_user(&pool, attacker_id).await;
}

fn v2_payload() -> Value {
    json!({
        "encrypted_filename": general_purpose::STANDARD.encode([1u8; 32]),
        "encrypted_file_key": general_purpose::STANDARD.encode([2u8; 60]),
        "encryption_nonce": general_purpose::STANDARD.encode([3u8; 12]),
        "encryption_version": "2", "file_size": 4,
        "encrypted_data": general_purpose::STANDARD.encode([4u8; 20]),
    })
}

#[tokio::test]
async fn v2_upload_is_atomic_owned_and_immutable() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let other_id = create_test_user(&pool).await;
    black_vault_api::database::vault_keys::create_vault_key(
        &pool,
        user_id,
        "test",
        "test",
        &json!({}),
        "test",
    )
    .await
    .unwrap();
    let upload = Request::builder()
        .method("POST")
        .uri("/files/encrypted")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(v2_payload().to_string()))
        .unwrap();
    let response = test_app(pool.clone()).oneshot(upload).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = response_json(response).await;
    assert_eq!(record["encryption_version"], "2");
    let id = record["id"].as_str().unwrap();
    for (actor, expected) in [(user_id, StatusCode::OK), (other_id, StatusCode::NOT_FOUND)] {
        let request = Request::builder()
            .uri(format!("/files/{id}/data"))
            .header("authorization", format!("Bearer {}", test_token(actor)))
            .body(Body::empty())
            .unwrap();
        let response = test_app(pool.clone()).oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected);
        if actor == user_id {
            assert_eq!(
                axum::body::to_bytes(response.into_body(), 100)
                    .await
                    .unwrap()
                    .as_ref(),
                &[4u8; 20]
            );
        }
    }
    let overwrite = Request::builder()
        .method("PUT")
        .uri(format!("/files/{id}/data"))
        .header("authorization", format!("Bearer {}", test_token(user_id)))
        .body(Body::from(vec![5u8; 20]))
        .unwrap();
    assert_eq!(
        test_app(pool.clone())
            .oneshot(overwrite)
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    cleanup_user(&pool, user_id).await;
    cleanup_user(&pool, other_id).await;
}

#[tokio::test]
async fn v2_upload_requires_authentication_vault_and_valid_metadata() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    for (token, payload, expected) in [
        (None, v2_payload(), StatusCode::UNAUTHORIZED),
        (
            Some(test_token(user_id)),
            v2_payload(),
            StatusCode::CONFLICT,
        ),
        (
            Some(test_token(user_id)),
            {
                let mut p = v2_payload();
                p["file_size"] = json!(5);
                p
            },
            StatusCode::BAD_REQUEST,
        ),
        (
            Some(test_token(user_id)),
            {
                let mut p = v2_payload();
                p["encrypted_file_key"] = json!("invalid");
                p
            },
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/files/encrypted")
            .header("content-type", "application/json");
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let response = test_app(pool.clone())
            .oneshot(builder.body(Body::from(payload.to_string())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    assert!(black_vault_api::database::files::list_files(&pool, user_id)
        .await
        .unwrap()
        .is_empty());
    cleanup_user(&pool, user_id).await;
}
