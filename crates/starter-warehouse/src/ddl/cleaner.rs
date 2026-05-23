//! Cleaner DDL generator. A cleaner is an L1→L2 (or sandbox→L2)
//! materialized view that promotes raw rows into `samples` /
//! `events` / `documents`. The generated MV is a plain
//! `CREATE MATERIALIZED VIEW IF NOT EXISTS …` that writes into the
//! declared target table; no `POPULATE` (W9: backfill is an
//! explicit `INSERT … SELECT` orchestrated by `cleaner.define`).
//!
//! Idempotency (W9 rewrite): a cleaner whose target is
//! `samples`/`events`/`documents` (which are plain MergeTree, not
//! Replacing) must produce *deterministic keys* — that is, given
//! the same source row, the cleaner inserts the same key into the
//! target. We enforce the contract at define-time: the projection
//! must declare `deterministic_key: true`. A cleaner that produces
//! non-deterministic keys (e.g. `now()` in the SELECT) declares
//! `backfill: 'none'` and `deterministic_key: false`; sync/async
//! backfill is then rejected.

use serde::{Deserialize, Serialize};

use super::{validate_ident, IdentError};

#[derive(Debug, thiserror::Error)]
pub enum DdlError {
    #[error(transparent)]
    Ident(#[from] IdentError),
    #[error("cleaner backfill={backfill:?} requires deterministic_key=true or a ReplacingMergeTree target")]
    NonDeterministicBackfill { backfill: String },
    #[error("unsupported target table: {0:?}")]
    UnsupportedTarget(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CleanerSpec {
    pub name: String,
    pub source_table: String,
    pub target_table: String,
    /// Raw projection SELECT clause minus the `SELECT` keyword and
    /// minus the `FROM`. Caller-validated; identifiers used here
    /// must come from a trusted catalog row, not user free-text.
    pub projection: String,
    /// `none` | `sync` | `async`.
    pub backfill: String,
    /// True when the projection emits a deterministic key (e.g.
    /// `entity_id, ts` columns). Required for `sync`/`async`.
    pub deterministic_key: bool,
}

pub struct CleanerDdl {
    pub view_name: String,
    pub create_view: String,
    pub drop_view: String,
    /// `INSERT INTO target SELECT projection FROM source WHERE
    /// ts < now()` — only emitted for non-`none` backfill modes.
    pub backfill_insert: Option<String>,
}

pub fn build(spec: &CleanerSpec) -> Result<CleanerDdl, DdlError> {
    let name = validate_ident(spec.name.strip_prefix("cleaner_").unwrap_or(&spec.name))?;
    let view = format!("cleaner_{name}");
    validate_ident(&spec.source_table)?;
    match spec.target_table.as_str() {
        "samples" | "events" | "documents" => {}
        other => return Err(DdlError::UnsupportedTarget(other.to_string())),
    }

    if spec.backfill != "none" && !spec.deterministic_key {
        return Err(DdlError::NonDeterministicBackfill {
            backfill: spec.backfill.clone(),
        });
    }

    let create_view = format!(
        "CREATE MATERIALIZED VIEW IF NOT EXISTS {view}\nTO {target} AS\nSELECT {proj} FROM {src}",
        target = spec.target_table,
        proj = spec.projection,
        src = spec.source_table,
    );
    let backfill_insert = (spec.backfill != "none").then(|| {
        format!(
            "INSERT INTO {target} SELECT {proj} FROM {src} WHERE ts < now()",
            target = spec.target_table,
            proj = spec.projection,
            src = spec.source_table,
        )
    });
    Ok(CleanerDdl {
        drop_view: format!("DROP VIEW IF EXISTS {view}"),
        view_name: view,
        create_view,
        backfill_insert,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_deterministic_backfill() {
        let s = CleanerSpec {
            name: "c1".into(),
            source_table: "raw_events".into(),
            target_table: "samples".into(),
            projection: "entity_id, ts, value_num".into(),
            backfill: "sync".into(),
            deterministic_key: false,
        };
        assert!(matches!(
            build(&s),
            Err(DdlError::NonDeterministicBackfill { .. })
        ));
    }

    #[test]
    fn emits_backfill_when_async() {
        let s = CleanerSpec {
            name: "c1".into(),
            source_table: "raw_events".into(),
            target_table: "samples".into(),
            projection: "entity_id, ts, value_num".into(),
            backfill: "async".into(),
            deterministic_key: true,
        };
        let d = build(&s).unwrap();
        assert!(d.backfill_insert.is_some());
    }
}
