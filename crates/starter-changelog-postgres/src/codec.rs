//! JSON / row codec for the Postgres backend.
//!
//! Mirrors the SQLite backend's codec; the difference is that
//! payload columns are `JSONB` (sqlx natively round-trips
//! `serde_json::Value`) and `at` is `TIMESTAMPTZ`.

use chrono::{DateTime, Utc};
use sqlx::Row;

use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Actor, Change, ChangeId, GroupId, Op, TraceId};
use starter_spi::{Error, Result};

/// Build the `(kind, id, meta, model)` tuple for INSERT. The
/// recorder writes `actor_model` explicitly so the contract for
/// every column lives in one place (resolves SCOPE open
/// question #3 — same shape as the SQLite backend's
/// `actor_columns`).
pub(crate) fn actor_columns(
    actor: &Actor,
) -> Result<(
    String,
    Option<String>,
    Option<serde_json::Value>,
    Option<String>,
)> {
    Ok(match actor {
        Actor::User { subject } => ("user".into(), Some(subject.clone()), None, None),
        Actor::Agent { run_id, model } => (
            "agent".into(),
            Some(run_id.clone()),
            Some(serde_json::json!({ "model": model })),
            Some(model.clone()),
        ),
        Actor::System => ("system".into(), None, None, None),
    })
}

/// Inverse: build an [`Actor`] from row columns.
pub(crate) fn actor_from_columns(
    kind: &str,
    id: Option<String>,
    meta: Option<serde_json::Value>,
    model: Option<String>,
) -> Result<Actor> {
    Ok(match kind {
        "user" => Actor::User {
            subject: id.unwrap_or_default(),
        },
        "agent" => {
            let model = model
                .or_else(|| {
                    meta.as_ref()
                        .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(str::to_owned))
                })
                .unwrap_or_default();
            Actor::Agent {
                run_id: id.unwrap_or_default(),
                model,
            }
        }
        "system" => Actor::System,
        other => {
            return Err(Error::Internal {
                source: format!("unknown actor_kind {other:?} in starter_changes").into(),
            })
        }
    })
}

/// Serialize [`Op`] for storage.
pub(crate) fn op_to_text(op: &Op) -> String {
    match op {
        Op::Create => "create".into(),
        Op::Update => "update".into(),
        Op::Delete => "delete".into(),
        Op::Custom(s) => format!("custom:{s}"),
    }
}

/// Inverse of [`op_to_text`].
pub(crate) fn op_from_text(s: &str) -> Op {
    match s {
        "create" => Op::Create,
        "update" => Op::Update,
        "delete" => Op::Delete,
        other => Op::Custom(other.strip_prefix("custom:").unwrap_or(other).to_owned()),
    }
}

/// Decode one `starter_changes` row into a [`Change`].
pub(crate) fn row_to_change(row: &sqlx::postgres::PgRow) -> Result<Change> {
    let id: String = row.try_get("id").map_err(internal)?;
    let at: DateTime<Utc> = row.try_get("at").map_err(internal)?;
    let actor_kind: String = row.try_get("actor_kind").map_err(internal)?;
    let actor_id: Option<String> = row.try_get("actor_id").map_err(internal)?;
    let actor_meta: Option<serde_json::Value> = row.try_get("actor_meta").map_err(internal)?;
    let actor_model: Option<String> = row.try_get("actor_model").map_err(internal)?;
    let resource_kind: String = row.try_get("resource_kind").map_err(internal)?;
    let resource_id: String = row.try_get("resource_id").map_err(internal)?;
    let resource_owner: Option<String> = row.try_get("resource_owner").map_err(internal)?;
    let resource_version: Option<i64> = row.try_get("resource_version").map_err(internal)?;
    let op_text: String = row.try_get("op").map_err(internal)?;
    let before: Option<serde_json::Value> = row.try_get("before").map_err(internal)?;
    let after: Option<serde_json::Value> = row.try_get("after").map_err(internal)?;
    let patch: Option<serde_json::Value> = row.try_get("patch").map_err(internal)?;
    let group_id: String = row.try_get("group_id").map_err(internal)?;
    let correlation: Option<String> = row.try_get("correlation").map_err(internal)?;

    Ok(Change {
        id: ChangeId(id),
        at,
        actor: actor_from_columns(&actor_kind, actor_id, actor_meta, actor_model)?,
        resource: ResourceRef {
            kind: resource_kind,
            id: Some(resource_id),
            owner: resource_owner,
        },
        resource_version: resource_version.map(|v| v as u64),
        op: op_from_text(&op_text),
        before,
        after,
        patch,
        group_id: GroupId(group_id),
        correlation: correlation.map(TraceId),
    })
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
