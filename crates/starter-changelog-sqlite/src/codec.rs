//! JSON envelope helpers shared by the recorder + query impls.
//!
//! `Change` rows persist with their `before` / `after` / `patch`
//! payloads as opaque JSON TEXT — starter never inspects them
//! (SCOPE §"Non-goals" — no automatic schema diffing).

use chrono::{DateTime, Utc};
use sqlx::Row;
use starter_spi::changelog::{Actor, Change, ChangeId, GroupId, Op, TraceId};
use starter_spi::authz::ResourceRef;
use starter_spi::{Error, Result};

/// Serialize a JSON value to TEXT or return `Error::Internal`.
pub(crate) fn json_to_text(v: &Option<serde_json::Value>) -> Result<Option<String>> {
    v.as_ref()
        .map(|val| {
            serde_json::to_string(val).map_err(|e| Error::Internal {
                source: Box::new(e),
            })
        })
        .transpose()
}

/// Parse a TEXT column back to JSON, mapping failures to `Internal`.
pub(crate) fn text_to_json(s: Option<String>) -> Result<Option<serde_json::Value>> {
    s.map(|raw| {
        serde_json::from_str(&raw).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    })
    .transpose()
}

/// `(actor_kind, actor_id, actor_meta_json, actor_model)` tuple for
/// the recorder's INSERT.
pub(crate) fn actor_columns(actor: &Actor) -> Result<(String, Option<String>, Option<String>, Option<String>)> {
    Ok(match actor {
        Actor::User { subject } => ("user".into(), Some(subject.clone()), None, None),
        Actor::Agent { run_id, model } => {
            let meta = serde_json::json!({ "model": model });
            let meta_text = serde_json::to_string(&meta).map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;
            (
                "agent".into(),
                Some(run_id.clone()),
                Some(meta_text),
                Some(model.clone()),
            )
        }
        Actor::System => ("system".into(), None, None, None),
    })
}

/// Inverse of [`actor_columns`].
pub(crate) fn actor_from_columns(
    kind: &str,
    id: Option<String>,
    meta: Option<String>,
    model: Option<String>,
) -> Result<Actor> {
    Ok(match kind {
        "user" => Actor::User {
            subject: id.unwrap_or_default(),
        },
        "agent" => {
            // Prefer denormalised `actor_model`; fall back to meta.
            let model = model
                .or_else(|| {
                    meta.as_deref()
                        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
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

/// Serialize `Op` for storage.
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
pub(crate) fn row_to_change(row: &sqlx::sqlite::SqliteRow) -> Result<Change> {
    let id: String = row.try_get("id").map_err(internal)?;
    let at: String = row.try_get("at").map_err(internal)?;
    let actor_kind: String = row.try_get("actor_kind").map_err(internal)?;
    let actor_id: Option<String> = row.try_get("actor_id").map_err(internal)?;
    let actor_meta: Option<String> = row.try_get("actor_meta").map_err(internal)?;
    let actor_model: Option<String> = row.try_get("actor_model").map_err(internal)?;
    let resource_kind: String = row.try_get("resource_kind").map_err(internal)?;
    let resource_id: String = row.try_get("resource_id").map_err(internal)?;
    let resource_owner: Option<String> = row.try_get("resource_owner").map_err(internal)?;
    let resource_version: Option<i64> = row.try_get("resource_version").map_err(internal)?;
    let op_text: String = row.try_get("op").map_err(internal)?;
    let before: Option<String> = row.try_get("before").map_err(internal)?;
    let after: Option<String> = row.try_get("after").map_err(internal)?;
    let patch: Option<String> = row.try_get("patch").map_err(internal)?;
    let group_id: String = row.try_get("group_id").map_err(internal)?;
    let correlation: Option<String> = row.try_get("correlation").map_err(internal)?;

    let at: DateTime<Utc> = at.parse().map_err(|e: chrono::ParseError| Error::Internal {
        source: Box::new(e),
    })?;

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
        before: text_to_json(before)?,
        after: text_to_json(after)?,
        patch: text_to_json(patch)?,
        group_id: GroupId(group_id),
        correlation: correlation.map(TraceId),
    })
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
