//! `GET /api/v1/ui/table`.
//!
//! Query string:
//!
//! ```text
//! ?source_id=<id>
//!  &page=1
//!  &size=50
//!  &sort=<field>,-<field>
//!  &filter=<rsql>
//! ```
//!
//! Per R6: tables on the wire are *queries*, not row lists. The
//! resolver emits an empty `Component::Table`; the client issues
//! one `/table` request per page. Sort / filter each produce one
//! round-trip. The host's [`crate::QueryEngine`] owns RSQL
//! parsing, push-down to the backing store, and column
//! whitelisting.
//!
//! R8 caps the `size` parameter to
//! [`crate::limits::MAX_TABLE_ROWS_PER_PAGE`] before dispatch; the
//! response from the engine is otherwise returned unchanged.

use axum::extract::{Query, State};
use axum::Json;

use crate::error::SduiError;
use crate::limits;
use crate::query::{QueryRequest, QueryResponse};
use crate::state::SduiState;

/// Axum handler.
pub async fn handler(
    State(state): State<SduiState>,
    Query(params): Query<QueryRequest>,
) -> Result<Json<QueryResponse>, SduiError> {
    limits::enforce_table_page_size(params.size)?;
    let resp = state.query.query(&params).await?;
    Ok(Json(resp))
}
