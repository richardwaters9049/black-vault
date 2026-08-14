mod api;
mod config;
mod database;
mod middleware;
mod models;
mod services;

use axum::{
    routing::{
        get,
        post,
    },
    Router,
};

use dotenvy::dotenv;

use tower_http::cors::{
    Any,
    CorsLayer,
};

use api::auth::{
    login,
    register,
};
use api::user::me;

use config::Config;
use middleware::auth::authenticate;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let config = Config::from_env()
        .expect("Failed to load application configuration");

    let pool = database::connect(
        &config.database_url,
    )
    .await
    .expect("Failed to connect to database");

    tracing::info!(
        "Database connected"
    );

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
        .route_layer(axum::middleware::from_fn_with_state(
            config.clone(),
            authenticate,
        ));

    let app: Router = public_routes
        .merge(protected_routes)
        .layer(cors)
        .with_state((
            pool,
            config.clone(),
        ));

    let address = format!(
        "{}:{}",
        config.host,
        config.port
    );

    let listener =
        tokio::net::TcpListener::bind(&address)
            .await
            .expect("Failed to bind server");

    tracing::info!(
        "Black Vault API running on http://{}:{}",
        config.host,
        config.port
    );

    axum::serve(
        listener,
        app,
    )
    .await
    .expect("Server error");
}

async fn health_check() -> &'static str {
    "OK"
}