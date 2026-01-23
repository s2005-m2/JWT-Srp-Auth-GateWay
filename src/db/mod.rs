use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;

use crate::config::DatabaseConfig;

pub async fn create_pool(config: &DatabaseConfig) -> anyhow::Result<Arc<PgPool>> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&config.url)
        .await?;

    Ok(Arc::new(pool))
}

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
