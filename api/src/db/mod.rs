use sqlx::{SqlitePool, migrate::MigrateError};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

pub async fn migrate(pool: &SqlitePool) -> Result<(), MigrateError> {
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await
        .map_err(MigrateError::Execute)?;
    MIGRATOR.run(pool).await
}
