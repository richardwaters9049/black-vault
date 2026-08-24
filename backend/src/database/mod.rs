pub mod users;
pub mod vault_keys;

use sqlx::{
    postgres::PgPoolOptions,
    PgPool,
};

pub async fn connect(
    database_url: &str,
) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}