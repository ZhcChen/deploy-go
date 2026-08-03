use std::borrow::Cow;

use sqlx::{
    SqlitePool,
    migrate::{MigrateError, Migration, Migrator},
};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();
const FOREIGN_KEY_ISOLATED_MIGRATIONS: &[i64] = &[3, 5];

pub async fn migrate(pool: &SqlitePool) -> Result<(), MigrateError> {
    migrate_with(pool, &MIGRATOR).await
}

pub async fn migrate_with(pool: &SqlitePool, migrator: &Migrator) -> Result<(), MigrateError> {
    let mut connection = pool.acquire().await?;
    set_foreign_keys(&mut connection, true).await?;

    let versions = migrator
        .iter()
        .map(|migration| migration.version)
        .collect::<Vec<_>>();
    for version in versions {
        let migration = filtered_migrator(migrator, |candidate| candidate == version);
        if FOREIGN_KEY_ISOLATED_MIGRATIONS.contains(&version) {
            set_foreign_keys(&mut connection, false).await?;
            let migration_result = migration.run_direct(&mut *connection).await;
            let restore_result = set_foreign_keys(&mut connection, true).await;
            migration_result?;
            restore_result?;
        } else {
            migration.run_direct(&mut *connection).await?;
        }
    }
    Ok(())
}

fn filtered_migrator(migrator: &Migrator, include: impl Fn(i64) -> bool) -> Migrator {
    let migrations = migrator
        .iter()
        .filter(|migration| include(migration.version))
        .cloned()
        .collect::<Vec<Migration>>();

    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: true,
        locking: true,
        no_tx: false,
    }
}

async fn set_foreign_keys(
    connection: &mut sqlx::SqliteConnection,
    enabled: bool,
) -> Result<(), MigrateError> {
    let statement = if enabled {
        "PRAGMA foreign_keys = ON"
    } else {
        "PRAGMA foreign_keys = OFF"
    };
    sqlx::query(statement)
        .execute(connection)
        .await
        .map_err(MigrateError::Execute)?;
    Ok(())
}
