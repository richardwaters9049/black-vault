use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub jwt_expiration: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL is not configured".to_string())?;

        if database_url.trim().is_empty() {
            return Err("DATABASE_URL cannot be empty".to_string());
        }

        let host = env::var("HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| "PORT must be a valid number".to_string())?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET is not configured".to_string())?;

        if jwt_secret.trim().is_empty() {
            return Err("JWT_SECRET cannot be empty".to_string());
        }

        let jwt_expiration = env::var("JWT_EXPIRATION")
            .unwrap_or_else(|_| "3600".to_string())
            .parse::<u64>()
            .map_err(|_| {
                "JWT_EXPIRATION must be a valid number".to_string()
            })?;

        if jwt_expiration == 0 {
            return Err(
                "JWT_EXPIRATION must be greater than zero".to_string()
            );
        }

        Ok(Self {
            database_url,
            host,
            port,
            jwt_secret,
            jwt_expiration,
        })
    }
}