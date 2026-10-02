use dotenvy::dotenv;

use black_vault_api::app::create_app;
use black_vault_api::config::Config;
use black_vault_api::database;
use black_vault_api::database::users::create_user;
use black_vault_api::services::auth::hash_password;

#[tokio::main]
async fn main() -> Result<(), String> {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let config = Config::from_env()
        .map_err(|error| format!("Failed to load application configuration: {error}"))?;

    tracing::info!("Connecting to database...");

    let pool = database::connect(&config.database_url)
        .await
        .map_err(|error| format!("Failed to connect to database: {error}"))?;

    tracing::info!("Database connected");

    tracing::info!("Running database migrations...");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|error| format!("Failed to run database migrations: {error}"))?;

    tracing::info!("Database migrations complete");

    seed_demo_user(&pool).await?;

    let app = create_app(pool, config.clone());

    let address = format!("{}:{}", config.host, config.port);

    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .map_err(|error| format!("Failed to bind server: {error}"))?;

    tracing::info!(
        "Black Vault API running on http://{}:{}",
        config.host,
        config.port
    );

    axum::serve(listener, app)
        .await
        .map_err(|error| format!("Server error: {error}"))?;

    Ok(())
}

async fn seed_demo_user(pool: &sqlx::PgPool) -> Result<(), String> {
    let enabled = std::env::var("DEMO_USER_ENABLED")
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !enabled {
        tracing::debug!("Demo user seeding disabled");

        return Ok(());
    }

    let email =
        std::env::var("DEMO_USER_EMAIL").unwrap_or_else(|_| "demo@blackvault.local".to_string());

    let password =
        std::env::var("DEMO_USER_PASSWORD").unwrap_or_else(|_| "BlackVaultDev123!".to_string());

    match black_vault_api::database::users::find_user_by_email(pool, &email).await {
        Ok(Some(_)) => {
            tracing::info!(
                email = %email,
                "Development demo user already exists"
            );
        }

        Ok(None) => {
            let password_hash = hash_password(&password)
                .map_err(|_| "Failed to hash demo user password".to_string())?;

            create_user(pool, &email, &password_hash)
                .await
                .map_err(|error| format!("Failed to create development demo user: {error}"))?;

            tracing::info!(
                email = %email,
                "Development demo user created"
            );
        }

        Err(error) => {
            tracing::error!(
                error = ?error,
                email = %email,
                "Failed to check for development demo user"
            );

            return Err("Failed to check for development demo user".to_string());
        }
    }

    Ok(())
}
