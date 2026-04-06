use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    AliasInput, KbAlias, KbCategory, KbEntry, KbEntryVersion, KbEntryVersionImage,
    KbImage, KbSearchConfig, KbSearchQuery, KbSearchResult, KbSearchResultImage,
};

// ── Categories ──────────────────────────────────────────────

pub async fn create_category(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
    parent_id: Option<Uuid>,
    sort_order: i32,
) -> Result<KbCategory, sqlx::Error> {
    sqlx::query_as::<_, KbCategory>(
        r#"
        INSERT INTO kb_categories (name, description, parent_id, sort_order)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(parent_id)
    .bind(sort_order)
    .fetch_one(pool)
    .await
}

pub async fn list_categories(pool: &PgPool) -> Result<Vec<KbCategory>, sqlx::Error> {
    sqlx::query_as::<_, KbCategory>(
        "SELECT * FROM kb_categories ORDER BY sort_order, name",
    )
    .fetch_all(pool)
    .await
}

// ── Entries ─────────────────────────────────────────────────

pub async fn create_entry(
    pool: &PgPool,
    item_name: &str,
    category_id: Option<Uuid>,
    region: &str,
    created_by: Option<Uuid>,
) -> Result<KbEntry, sqlx::Error> {
    sqlx::query_as::<_, KbEntry>(
        r#"
        INSERT INTO kb_entries (item_name, category_id, region, created_by)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(item_name)
    .bind(category_id)
    .bind(region)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn get_entry(pool: &PgPool, entry_id: Uuid) -> Result<Option<KbEntry>, sqlx::Error> {
    sqlx::query_as::<_, KbEntry>("SELECT * FROM kb_entries WHERE id = $1")
        .bind(entry_id)
        .fetch_optional(pool)
        .await
}

pub async fn update_entry_head(
    pool: &PgPool,
    entry_id: Uuid,
    item_name: &str,
    region: &str,
    new_version: i32,
) -> Result<KbEntry, sqlx::Error> {
    sqlx::query_as::<_, KbEntry>(
        r#"
        UPDATE kb_entries
        SET item_name = $2, region = $3, current_version = $4, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(entry_id)
    .bind(item_name)
    .bind(region)
    .bind(new_version)
    .fetch_one(pool)
    .await
}

pub async fn deactivate_entry(pool: &PgPool, entry_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE kb_entries SET is_active = FALSE, updated_at = NOW() WHERE id = $1")
        .bind(entry_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Versions ────────────────────────────────────────────────

pub async fn create_version(
    pool: &PgPool,
    entry_id: Uuid,
    version_number: i32,
    item_name: &str,
    disposal_category: &str,
    disposal_instructions: &str,
    special_handling: Option<&str>,
    contamination_notes: Option<&str>,
    region: &str,
    rule_source: Option<&str>,
    effective_date: NaiveDate,
    change_summary: Option<&str>,
    created_by: Option<Uuid>,
) -> Result<KbEntryVersion, sqlx::Error> {
    sqlx::query_as::<_, KbEntryVersion>(
        r#"
        INSERT INTO kb_entry_versions (
            entry_id, version_number, item_name, disposal_category,
            disposal_instructions, special_handling, contamination_notes,
            region, rule_source, effective_date, change_summary, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING *
        "#,
    )
    .bind(entry_id)
    .bind(version_number)
    .bind(item_name)
    .bind(disposal_category)
    .bind(disposal_instructions)
    .bind(special_handling)
    .bind(contamination_notes)
    .bind(region)
    .bind(rule_source)
    .bind(effective_date)
    .bind(change_summary)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

pub async fn get_current_version(
    pool: &PgPool,
    entry_id: Uuid,
) -> Result<Option<KbEntryVersion>, sqlx::Error> {
    sqlx::query_as::<_, KbEntryVersion>(
        r#"
        SELECT v.* FROM kb_entry_versions v
        JOIN kb_entries e ON e.id = v.entry_id
        WHERE v.entry_id = $1 AND v.version_number = e.current_version
        "#,
    )
    .bind(entry_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_version_history(
    pool: &PgPool,
    entry_id: Uuid,
) -> Result<Vec<KbEntryVersion>, sqlx::Error> {
    sqlx::query_as::<_, KbEntryVersion>(
        "SELECT * FROM kb_entry_versions WHERE entry_id = $1 ORDER BY version_number DESC",
    )
    .bind(entry_id)
    .fetch_all(pool)
    .await
}

pub async fn get_version_by_number(
    pool: &PgPool,
    entry_id: Uuid,
    version_number: i32,
) -> Result<Option<KbEntryVersion>, sqlx::Error> {
    sqlx::query_as::<_, KbEntryVersion>(
        "SELECT * FROM kb_entry_versions WHERE entry_id = $1 AND version_number = $2",
    )
    .bind(entry_id)
    .bind(version_number)
    .fetch_optional(pool)
    .await
}

// ── Aliases ─────────────────────────────────────────────────

pub async fn set_aliases(
    pool: &PgPool,
    entry_id: Uuid,
    aliases: &[AliasInput],
) -> Result<Vec<KbAlias>, sqlx::Error> {
    // Remove existing aliases
    sqlx::query("DELETE FROM kb_aliases WHERE entry_id = $1")
        .bind(entry_id)
        .execute(pool)
        .await?;

    let mut result = Vec::new();
    for a in aliases {
        let alias_type = a.alias_type.as_deref().unwrap_or("alias");
        let row = sqlx::query_as::<_, KbAlias>(
            r#"
            INSERT INTO kb_aliases (entry_id, alias, alias_type)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(entry_id)
        .bind(&a.alias)
        .bind(alias_type)
        .fetch_one(pool)
        .await?;
        result.push(row);
    }

    Ok(result)
}

pub async fn get_aliases(pool: &PgPool, entry_id: Uuid) -> Result<Vec<KbAlias>, sqlx::Error> {
    sqlx::query_as::<_, KbAlias>(
        "SELECT * FROM kb_aliases WHERE entry_id = $1 ORDER BY alias",
    )
    .bind(entry_id)
    .fetch_all(pool)
    .await
}

// ── Images ──────────────────────────────────────────────────

pub async fn find_image_by_hash(
    pool: &PgPool,
    sha256_hash: &str,
) -> Result<Option<KbImage>, sqlx::Error> {
    sqlx::query_as::<_, KbImage>("SELECT * FROM kb_images WHERE sha256_hash = $1")
        .bind(sha256_hash)
        .fetch_optional(pool)
        .await
}

pub async fn insert_image(
    pool: &PgPool,
    file_name: &str,
    file_path: &str,
    file_size: i64,
    mime_type: &str,
    sha256_hash: &str,
    width: Option<i32>,
    height: Option<i32>,
    uploaded_by: Option<Uuid>,
) -> Result<KbImage, sqlx::Error> {
    sqlx::query_as::<_, KbImage>(
        r#"
        INSERT INTO kb_images (file_name, file_path, file_size, mime_type, sha256_hash, width, height, uploaded_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(file_name)
    .bind(file_path)
    .bind(file_size)
    .bind(mime_type)
    .bind(sha256_hash)
    .bind(width)
    .bind(height)
    .bind(uploaded_by)
    .fetch_one(pool)
    .await
}

pub async fn get_image(pool: &PgPool, image_id: Uuid) -> Result<Option<KbImage>, sqlx::Error> {
    sqlx::query_as::<_, KbImage>("SELECT * FROM kb_images WHERE id = $1")
        .bind(image_id)
        .fetch_optional(pool)
        .await
}

pub async fn link_images_to_version(
    pool: &PgPool,
    version_id: Uuid,
    image_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    for (i, image_id) in image_ids.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO kb_entry_version_images (version_id, image_id, sort_order)
            VALUES ($1, $2, $3)
            ON CONFLICT (version_id, image_id) DO NOTHING
            "#,
        )
        .bind(version_id)
        .bind(image_id)
        .bind(i as i32)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Joined row for image + version-image junction data
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct VersionImageRow {
    pub id: Uuid,
    pub file_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub sha256_hash: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub uploaded_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub caption: Option<String>,
    pub sort_order: i32,
}

pub async fn get_version_images(
    pool: &PgPool,
    version_id: Uuid,
) -> Result<Vec<VersionImageRow>, sqlx::Error> {
    sqlx::query_as::<_, VersionImageRow>(
        r#"
        SELECT i.id, i.file_name, i.file_path, i.file_size, i.mime_type,
               i.sha256_hash, i.width, i.height, i.uploaded_by, i.created_at,
               vi.caption, vi.sort_order
        FROM kb_images i
        JOIN kb_entry_version_images vi ON vi.image_id = i.id
        WHERE vi.version_id = $1
        ORDER BY vi.sort_order
        "#,
    )
    .bind(version_id)
    .fetch_all(pool)
    .await
}

// ── Search config ───────────────────────────────────────────

pub async fn get_search_config(pool: &PgPool) -> Result<KbSearchConfig, sqlx::Error> {
    sqlx::query_as::<_, KbSearchConfig>(
        "SELECT * FROM kb_search_config ORDER BY updated_at DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await
}

pub async fn update_search_config(
    pool: &PgPool,
    config_id: Uuid,
    name_exact_weight: f32,
    name_prefix_weight: f32,
    name_fuzzy_weight: f32,
    alias_exact_weight: f32,
    alias_fuzzy_weight: f32,
    category_boost: f32,
    region_boost: f32,
    recency_boost: f32,
    fuzzy_threshold: f32,
    max_results: i32,
    updated_by: Option<Uuid>,
) -> Result<KbSearchConfig, sqlx::Error> {
    sqlx::query_as::<_, KbSearchConfig>(
        r#"
        UPDATE kb_search_config SET
            name_exact_weight = $2, name_prefix_weight = $3, name_fuzzy_weight = $4,
            alias_exact_weight = $5, alias_fuzzy_weight = $6,
            category_boost = $7, region_boost = $8, recency_boost = $9,
            fuzzy_threshold = $10, max_results = $11,
            updated_by = $12, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(config_id)
    .bind(name_exact_weight)
    .bind(name_prefix_weight)
    .bind(name_fuzzy_weight)
    .bind(alias_exact_weight)
    .bind(alias_fuzzy_weight)
    .bind(category_boost)
    .bind(region_boost)
    .bind(recency_boost)
    .bind(fuzzy_threshold)
    .bind(max_results)
    .bind(updated_by)
    .fetch_one(pool)
    .await
}

// ── Fuzzy search ────────────────────────────────────────────

/// Perform fuzzy search across item names and aliases, ranked by configurable weights.
/// Uses PostgreSQL pg_trgm for trigram similarity and ts_rank for full-text.
pub async fn fuzzy_search(
    pool: &PgPool,
    query: &KbSearchQuery,
    config: &KbSearchConfig,
) -> Result<(Vec<KbSearchResultRow>, i64), sqlx::Error> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(config.max_results as i64).min(100);
    let offset = (page - 1) * page_size;
    let search_term = &query.q;
    let threshold = config.fuzzy_threshold;

    // Build the search query using UNION ALL for different match types,
    // then rank by configurable weights
    let sql = r#"
        WITH search_matches AS (
            -- Exact name match
            SELECT
                e.id AS entry_id,
                e.item_name,
                NULL::text AS matched_alias,
                'exact' AS match_type,
                $8::real AS base_score
            FROM kb_entries e
            WHERE e.is_active = TRUE
              AND LOWER(e.item_name) = LOWER($1)

            UNION ALL

            -- Prefix name match
            SELECT
                e.id AS entry_id,
                e.item_name,
                NULL::text AS matched_alias,
                'prefix' AS match_type,
                $9::real AS base_score
            FROM kb_entries e
            WHERE e.is_active = TRUE
              AND LOWER(e.item_name) LIKE LOWER($1) || '%'
              AND LOWER(e.item_name) != LOWER($1)

            UNION ALL

            -- Fuzzy name match (trigram)
            SELECT
                e.id AS entry_id,
                e.item_name,
                NULL::text AS matched_alias,
                'fuzzy' AS match_type,
                ($10::real * similarity(e.item_name, $1)) AS base_score
            FROM kb_entries e
            WHERE e.is_active = TRUE
              AND similarity(e.item_name, $1) > $7
              AND LOWER(e.item_name) != LOWER($1)
              AND NOT (LOWER(e.item_name) LIKE LOWER($1) || '%')

            UNION ALL

            -- Exact alias match
            SELECT
                a.entry_id,
                e.item_name,
                a.alias AS matched_alias,
                'alias_exact' AS match_type,
                $11::real AS base_score
            FROM kb_aliases a
            JOIN kb_entries e ON e.id = a.entry_id
            WHERE e.is_active = TRUE
              AND LOWER(a.alias) = LOWER($1)

            UNION ALL

            -- Fuzzy alias match (trigram)
            SELECT
                a.entry_id,
                e.item_name,
                a.alias AS matched_alias,
                'alias_fuzzy' AS match_type,
                ($12::real * similarity(a.alias, $1)) AS base_score
            FROM kb_aliases a
            JOIN kb_entries e ON e.id = a.entry_id
            WHERE e.is_active = TRUE
              AND similarity(a.alias, $1) > $7
              AND LOWER(a.alias) != LOWER($1)
        ),
        -- Deduplicate: pick best match per entry
        ranked AS (
            SELECT DISTINCT ON (entry_id)
                sm.entry_id,
                sm.item_name,
                sm.matched_alias,
                sm.match_type,
                sm.base_score
                    + CASE WHEN $2::uuid IS NOT NULL AND e.category_id = $2 THEN $13::real ELSE 0 END
                    + CASE WHEN $3::text IS NOT NULL AND e.region = $3 THEN $14::real ELSE 0 END
                    AS score,
                e.region,
                e.category_id,
                e.current_version
            FROM search_matches sm
            JOIN kb_entries e ON e.id = sm.entry_id
            ORDER BY entry_id, sm.base_score DESC
        )
        SELECT
            r.entry_id,
            r.item_name,
            r.matched_alias,
            r.match_type,
            r.score,
            r.region,
            r.current_version,
            c.name AS category_name,
            v.disposal_category,
            v.disposal_instructions,
            v.special_handling,
            v.contamination_notes,
            v.rule_source,
            v.effective_date
        FROM ranked r
        LEFT JOIN kb_categories c ON c.id = r.category_id
        JOIN kb_entry_versions v ON v.entry_id = r.entry_id AND v.version_number = r.current_version
        ORDER BY r.score DESC
        LIMIT $4 OFFSET $5
    "#;

    let count_sql = r#"
        SELECT COUNT(DISTINCT e.id)
        FROM kb_entries e
        LEFT JOIN kb_aliases a ON a.entry_id = e.id
        WHERE e.is_active = TRUE
          AND (
            similarity(e.item_name, $1) > $2
            OR LOWER(e.item_name) LIKE LOWER($1) || '%'
            OR similarity(a.alias, $1) > $2
            OR LOWER(a.alias) = LOWER($1)
          )
    "#;

    let total: i64 = sqlx::query_scalar(count_sql)
        .bind(search_term)
        .bind(threshold)
        .fetch_one(pool)
        .await?;

    let rows = sqlx::query_as::<_, KbSearchResultRow>(sql)
        .bind(search_term)                           // $1
        .bind(query.category_id)                     // $2
        .bind(query.region.as_deref())               // $3
        .bind(page_size)                             // $4
        .bind(offset)                                // $5
        .bind(6_i32)                                 // $6 placeholder (unused)
        .bind(threshold)                             // $7
        .bind(config.name_exact_weight)              // $8
        .bind(config.name_prefix_weight)             // $9
        .bind(config.name_fuzzy_weight)              // $10
        .bind(config.alias_exact_weight)             // $11
        .bind(config.alias_fuzzy_weight)             // $12
        .bind(config.category_boost)                 // $13
        .bind(config.region_boost)                   // $14
        .fetch_all(pool)
        .await?;

    Ok((rows, total))
}

/// Intermediate row from the search query
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct KbSearchResultRow {
    pub entry_id: Uuid,
    pub item_name: String,
    pub matched_alias: Option<String>,
    pub match_type: String,
    pub score: f32,
    pub region: String,
    pub current_version: i32,
    pub category_name: Option<String>,
    pub disposal_category: String,
    pub disposal_instructions: String,
    pub special_handling: Option<String>,
    pub contamination_notes: Option<String>,
    pub rule_source: Option<String>,
    pub effective_date: NaiveDate,
}
