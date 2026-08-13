mod api;
mod config;
mod database;
mod models;
mod services;

use axum::{
    routing::post,
    Router,
};

use dotenvy::dotenv;

use api::auth::{
    login,
    register,
};

use config::Config;

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

    let app = Router::new()
        .route(
            "/auth/register",
            post(register),
        )
        .route(
            "/auth/login",
            post(login),
        )
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