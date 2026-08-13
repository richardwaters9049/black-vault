mod api;
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

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let pool = database::connect()
        .await
        .expect("Failed to connect to database");

    println!("Database connected");

    let app = Router::new()
        .route(
            "/auth/register",
            post(register),
        )
        .route(
            "/auth/login",
            post(login),
        )
        .with_state(pool);

    let listener =
        tokio::net::TcpListener::bind(
            "0.0.0.0:8080"
        )
        .await
        .expect("Failed to bind server");

    println!(
        "Black Vault API running on http://localhost:8080"
    );

    axum::serve(
        listener,
        app,
    )
    .await
    .expect("Server error");
}