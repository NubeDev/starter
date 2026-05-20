//! JSON envelope helpers shared by the three flow store impls.
//!
//! Every payload the engine hands the store is a typed SPI struct
//! that derives `Serialize`/`Deserialize`. We persist them as
//! opaque JSON TEXT columns (R6: "the store treats them as opaque
//! blobs"); this module is the chokepoint that converts between
//! the typed SPI value and the on-disk JSON.
//!
//! All conversion failures map to [`FlowError::Backend`] — they
//! mean either schema drift (a future migration didn't land) or
//! disk corruption, both of which are operator-actionable.

use serde::{de::DeserializeOwned, Serialize};
use starter_flow_spi::flow::{FlowError, FlowResult};

/// Serialize a value to compact JSON, mapping failures to
/// [`FlowError::Backend`].
pub(super) fn to_json<T: Serialize>(value: &T) -> FlowResult<String> {
    serde_json::to_string(value).map_err(|e| FlowError::Backend(format!("serialize: {e}")))
}

/// Parse JSON from a TEXT column, mapping failures to
/// [`FlowError::Backend`] with a column hint so disk corruption
/// is operator-actionable.
pub(super) fn from_json<T: DeserializeOwned>(column: &str, raw: &str) -> FlowResult<T> {
    serde_json::from_str(raw).map_err(|e| FlowError::Backend(format!("deserialize {column}: {e}")))
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
