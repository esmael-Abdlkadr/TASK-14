pub mod users;
pub mod sessions;
pub mod audit;
pub mod devices;
pub mod rate_limit;
pub mod knowledge_base;
pub mod inspection;
pub mod review;
pub mod admin;
pub mod messaging;
pub mod bulk_data;
pub mod dispute;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    let migration_001 = include_str!("../../migrations/001_initial_schema.sql");
    sqlx::raw_sql(migration_001).execute(pool).await?;

    let migration_002 = include_str!("../../migrations/002_knowledge_base.sql");
    sqlx::raw_sql(migration_002).execute(pool).await?;

    let migration_003 = include_str!("../../migrations/003_inspection_tasks.sql");
    sqlx::raw_sql(migration_003).execute(pool).await?;

    let migration_004 = include_str!("../../migrations/004_review_scorecards.sql");
    sqlx::raw_sql(migration_004).execute(pool).await?;

    let migration_005 = include_str!("../../migrations/005_admin_dashboard.sql");
    sqlx::raw_sql(migration_005).execute(pool).await?;

    let migration_006 = include_str!("../../migrations/006_messaging.sql");
    sqlx::raw_sql(migration_006).execute(pool).await?;

    let migration_007 = include_str!("../../migrations/007_bulk_data.sql");
    sqlx::raw_sql(migration_007).execute(pool).await?;

    let migration_008 = include_str!("../../migrations/008_disputed_classifications.sql");
    sqlx::raw_sql(migration_008).execute(pool).await?;

    let migration_009 = include_str!("../../migrations/009_encrypted_fields.sql");
    sqlx::raw_sql(migration_009).execute(pool).await?;

    let migration_010 = include_str!("../../migrations/010_device_fingerprint_hash.sql");
    sqlx::raw_sql(migration_010).execute(pool).await?;

    Ok(())
}
