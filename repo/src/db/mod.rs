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

const MIGRATIONS: &[(i32, &str)] = &[
    (1, include_str!("../../migrations/001_initial_schema.sql")),
    (2, include_str!("../../migrations/002_knowledge_base.sql")),
    (3, include_str!("../../migrations/003_inspection_tasks.sql")),
    (4, include_str!("../../migrations/004_review_scorecards.sql")),
    (5, include_str!("../../migrations/005_admin_dashboard.sql")),
    (6, include_str!("../../migrations/006_messaging.sql")),
    (7, include_str!("../../migrations/007_bulk_data.sql")),
    (8, include_str!("../../migrations/008_disputed_classifications.sql")),
    (9, include_str!("../../migrations/009_encrypted_fields.sql")),
    (10, include_str!("../../migrations/010_device_fingerprint_hash.sql")),
];

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS _civicsort_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(pool)
    .await?;

    let tracked: i64 =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM _civicsort_migrations")
            .fetch_one(pool)
            .await?;

    if tracked == 0 {
        let legacy_schema: bool = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = 'users'
            ) OR EXISTS (
                SELECT 1 FROM pg_type t
                JOIN pg_namespace n ON n.oid = t.typnamespace
                WHERE n.nspname = 'public' AND t.typname = 'user_role'
            )"#,
        )
        .fetch_one(pool)
        .await?;

        if legacy_schema {
            for v in 1_i32..=10_i32 {
                sqlx::query("INSERT INTO _civicsort_migrations (version) VALUES ($1)")
                    .bind(v)
                    .execute(pool)
                    .await?;
            }
            return Ok(());
        }
    }

    for &(version, sql) in MIGRATIONS {
        let applied: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM _civicsort_migrations WHERE version = $1)",
        )
        .bind(version)
        .fetch_one(pool)
        .await?;

        if applied {
            continue;
        }

        let mut tx = pool.begin().await?;
        sqlx::raw_sql(sql).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO _civicsort_migrations (version) VALUES ($1)")
            .bind(version)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }

    Ok(())
}
