//! Typed CRUD for `tag_definitions` (T5).
//!
//! `starter-tags` defines [`TagDefinition`] and [`TagKind`]; this
//! module is the Postgres bridge. The `kind` CHECK constraint in
//! `0003_tag_definitions.sql` mirrors `TagKind::as_str` exactly —
//! drift is a compile/test failure, not a silent corruption.

use chrono::{DateTime, Utc};
use sqlx::types::Json;
use starter_tags::{TagDefinition, TagKind};

use crate::pool::Pool;

/// Result alias.
pub type Result<T> = std::result::Result<T, sqlx::Error>;

#[derive(sqlx::FromRow)]
struct Row {
    key: String,
    kind: String,
    description: Option<String>,
    enum_values: Option<Json<serde_json::Value>>,
    ref_kind: Option<String>,
    source: String,
    #[allow(dead_code)]
    created_at: DateTime<Utc>,
}

fn parse_kind(s: &str) -> Result<TagKind> {
    Ok(match s {
        "bool" => TagKind::Bool,
        "str" => TagKind::Str,
        "ref" => TagKind::Ref,
        "num_discriminant" => TagKind::NumDiscriminant,
        other => {
            return Err(sqlx::Error::Decode(
                format!("invalid tag kind in db: {other:?}").into(),
            ))
        }
    })
}

fn enum_values_from_json(v: serde_json::Value) -> Option<Vec<String>> {
    match v {
        serde_json::Value::Array(items) => Some(
            items
                .into_iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect(),
        ),
        _ => None,
    }
}

impl Row {
    fn into_definition(self) -> Result<TagDefinition> {
        Ok(TagDefinition {
            key: self.key,
            kind: parse_kind(&self.kind)?,
            description: self.description,
            enum_values: self.enum_values.and_then(|j| enum_values_from_json(j.0)),
            ref_kind: self.ref_kind,
            source: self.source,
        })
    }
}

/// Upsert a definition (T5: advisory — never refuses writes; merely
/// records what the workspace knows about a key).
pub async fn upsert(pool: &Pool, def: &TagDefinition) -> Result<()> {
    let enum_json = def.enum_values.as_ref().map(|v| Json(serde_json::json!(v)));
    sqlx::query(
        "INSERT INTO tag_definitions \
            (key, kind, description, enum_values, ref_kind, source) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (key) DO UPDATE SET \
            kind = EXCLUDED.kind, \
            description = EXCLUDED.description, \
            enum_values = EXCLUDED.enum_values, \
            ref_kind = EXCLUDED.ref_kind, \
            source = EXCLUDED.source",
    )
    .bind(&def.key)
    .bind(def.kind.as_str())
    .bind(def.description.as_ref())
    .bind(enum_json)
    .bind(def.ref_kind.as_ref())
    .bind(&def.source)
    .execute(pool.sqlx())
    .await?;
    Ok(())
}

/// Fetch one definition by key.
pub async fn get(pool: &Pool, key: &str) -> Result<Option<TagDefinition>> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT key, kind, description, enum_values, ref_kind, source, created_at \
         FROM tag_definitions WHERE key = $1",
    )
    .bind(key)
    .fetch_optional(pool.sqlx())
    .await?;
    row.map(Row::into_definition).transpose()
}

/// List every definition (autocomplete + agent surface).
pub async fn list(pool: &Pool) -> Result<Vec<TagDefinition>> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT key, kind, description, enum_values, ref_kind, source, created_at \
         FROM tag_definitions ORDER BY key",
    )
    .fetch_all(pool.sqlx())
    .await?;
    rows.into_iter().map(Row::into_definition).collect()
}
