//! JSON envelope helpers shared by the three Postgres flow store
//! impls. Twin of `starter-store-sqlite::flow::schema`; the only
//! shape difference is that Postgres `JSONB` columns take a
//! `serde_json::Value` binding rather than a `String`, so the
//! conversion helpers yield `Value` instead of compact JSON text.
//!
//! All conversion failures map to [`FlowError::Backend`] — they
//! mean either schema drift (a future migration didn't land) or
//! disk corruption, both of which are operator-actionable.

use serde::{de::DeserializeOwned, Serialize};
use starter_flow_spi::flow::{FlowError, FlowResult};

/// Serialize a value to a `serde_json::Value` for binding to a
/// `JSONB` column, mapping failures to [`FlowError::Backend`].
pub(super) fn to_value<T: Serialize>(value: &T) -> FlowResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| FlowError::Backend(format!("serialize: {e}")))
}

/// Deserialize a `serde_json::Value` read from a `JSONB` column,
/// mapping failures to [`FlowError::Backend`] with a column hint
/// so disk corruption is operator-actionable.
pub(super) fn from_value<T: DeserializeOwned>(
    column: &str,
    raw: serde_json::Value,
) -> FlowResult<T> {
    serde_json::from_value(raw)
        .map_err(|e| FlowError::Backend(format!("deserialize {column}: {e}")))
}

/// Map a `sqlx::Error` to [`FlowError`], teasing apart the
/// "row absent" case the store contract expresses as
/// [`FlowError::NotFound`] from everything else.
pub(super) fn sqlx_to_flow(err: sqlx::Error, kind: &'static str, id: String) -> FlowError {
    match err {
        sqlx::Error::RowNotFound => FlowError::NotFound { kind, id },
        other => FlowError::Backend(format!("{kind} {id}: {other}")),
    }
}

/// Map a `sqlx::Error` to [`FlowError::Backend`] for calls where
/// row absence is not an error (`list`, `head`, `find_by_*`).
pub(super) fn sqlx_backend(err: sqlx::Error) -> FlowError {
    FlowError::Backend(err.to_string())
}
