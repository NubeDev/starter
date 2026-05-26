//! Typed write paths for the TimescaleDB hypertables. Mirrors
//! the structure of [`crate::store`] but targets PgPool and uses
//! `COPY` for batch ingest.
//!
//! All four hypertables (`raw_events`, `samples`, `events`,
//! `documents`) carry an explicit `tenant_id TEXT NOT NULL`
//! column (ADR-003 / proposal §"Tenancy on caggs"). Writers must
//! supply it on every row.

pub mod documents;
pub mod events;
pub mod raw_events;
pub mod samples;

use chrono::{DateTime, Utc};

/// RFC3339 (with fractional seconds) is the canonical
/// representation TimescaleDB's `COPY ... FORMAT TEXT` parser
/// accepts for `TIMESTAMPTZ`.
pub(crate) fn fmt_ts(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

/// Escape a string for `COPY ... FORMAT TEXT`. Tab, newline,
/// carriage return, and backslash are the four bytes the text
/// format reserves; backslash also escapes itself.
pub(crate) fn copy_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

/// Render a JSON `Value` (or default empty object) into the
/// COPY-safe textual form for a `JSONB` column.
pub(crate) fn copy_json(value: &serde_json::Value) -> String {
    copy_escape(&value.to_string())
}

/// Sentinel used by `COPY ... FORMAT TEXT` for SQL NULL.
pub(crate) const NULL: &str = "\\N";
