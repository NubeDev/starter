//! `POST /api/v1/insights/preview` — run an inline script over sample rows.
//!
//! The insights authoring workbench's instant edit→result loop: a tenant POSTs a
//! draft script plus a small sample of input rows and gets the transformed result
//! back, shaped exactly like a [`QueryResponse`] so the existing ResultGrid can
//! render it. Nothing is persisted. A *script* error (compile or runtime) is not a
//! transport failure — it is returned as `ok: false` with HTTP 200 so the
//! workbench shows it inline in the result pane; only auth / malformed-request
//! faults are true HTTP errors.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::dto::query::QueryResponse;

/// Run an inline Rhai script against `rows` without saving anything. The frontend
/// already holds query results, so preview is rows-in / rows-out and decoupled
/// from re-querying — there is deliberately no datasource/sql path in v1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PreviewInsightRequest {
    /// The inline Rhai script to run over the sample rows.
    pub script: String,
    /// The sample input rows (JSON objects) to transform.
    #[serde(default)]
    pub rows: Vec<Value>,
    /// Optional parameters bound as the script's `params` object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// The preview outcome. A successful run carries the transformed result and the
/// input row count (so the UI can show "120 → 87 rows"); a script error carries a
/// structured, tenant-safe message. Untagged so the wire shape is simply one of
/// the two object forms keyed by `ok`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum PreviewInsightResponse {
    /// The script ran. `result` is the QueryResponse-shaped transformed output;
    /// `row_count_in` is the number of input rows the script saw.
    Ok {
        /// Always `true` for this variant — lets the client discriminate.
        ok: bool,
        /// The transformed rows, columns, and stats — ready for the ResultGrid.
        result: QueryResponse,
        /// The number of input rows submitted (for the "in → out" row delta).
        row_count_in: u64,
    },
    /// The script failed to compile or run. Returned with HTTP 200 so the UI
    /// renders it inline rather than as a failed request.
    Err {
        /// Always `false` for this variant.
        ok: bool,
        /// The structured error detail.
        error: PreviewInsightError,
    },
}

impl PreviewInsightResponse {
    /// A successful preview carrying the transformed result.
    pub fn ok(result: QueryResponse, row_count_in: u64) -> Self {
        Self::Ok {
            ok: true,
            result,
            row_count_in,
        }
    }

    /// A script-error preview, returned with HTTP 200.
    pub fn err(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Err {
            ok: false,
            error: PreviewInsightError {
                message: message.into(),
                kind: kind.into(),
            },
        }
    }
}

/// A tenant-safe script error. `kind` classifies the phase that failed so the UI
/// can colour/label it: `"compile"` (syntax), `"runtime"` (logic against the
/// data), or `"limit"` (a sandbox bound tripped). `message` is the full
/// position-annotated detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PreviewInsightError {
    /// The full, tenant-safe error message.
    pub message: String,
    /// One of `"compile" | "runtime" | "limit"`.
    pub kind: String,
}
