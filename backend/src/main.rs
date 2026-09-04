use dotenvy::dotenv;

use black_vault_api::app::create_app;
use black_vault_api::config::Config;
use black_vault_api::database;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let config = Config::from_env().expect("Failed to load application configuration");

    tracing::info!("Connecting to database...");

    let pool = database::connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Database connected");

    tracing::info!("Running database migrations...");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database migrations complete");

    let app = create_app(pool, config.clone());

    let address = format!("{}:{}", config.host, config.port);

    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("Failed to bind server");

    tracing::info!(
        "Black Vault API running on http://{}:{}",
        config.host,
        config.port
    );

    axum::serve(listener, app)
        .await
        .expect("Server error");
}