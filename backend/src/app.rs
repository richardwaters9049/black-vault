use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::api::auth::{login, register};
use crate::api::files::{
    create, delete as delete_file, download_data, get as get_file, list, upload_data,
};
use crate::api::user::me;
use crate::api::vault::{get_vault_key, setup_vault};
use crate::config::Config;
use crate::middleware::auth::authenticate;

pub fn create_app(pool: sqlx::PgPool, config: Config) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login));

    let protected_routes = Router::new()
        .route("/user/me", get(me))
        .route("/vault/setup", post(setup_vault))
        .route("/vault/key", get(get_vault_key))
        .route("/files", post(create).get(list))
        .route("/files/{file_id}", get(get_file).delete(delete_file))
        .route("/files/{file_id}/data", put(upload_data).get(download_data))
        .route_layer(axum::middleware::from_fn_with_state(
            config.clone(),
            authenticate,
        ));

    public_routes
        .merge(protected_routes)
        .layer(cors)
        .with_state((pool, config))
}

async fn health_check() -> &'static str {
    "OK"
}
