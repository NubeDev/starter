//! Types shared by the query request and response.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The column's logical type, derived from the Arrow schema of the result.
/// Kept deliberately coarse — the frontend renders cells, it does not need the
/// full Arrow type lattice. Anything not in this set arrives as [`Self::Other`]
/// with the rows already JSON-encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResultColumnType {
    Bool,
    Int,
    Float,
    String,
    /// Timestamp / date / time — serialized as an RFC-3339 string in the rows.
    Timestamp,
    /// A type with no coarse mapping; the row value is the raw JSON encoding.
    Other,
}

/// One column of a query result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ColumnSchema {
    /// Column name as it appears in the result set.
    pub name: String,
    /// Coarse logical type for rendering.
    #[serde(rename = "type")]
    pub column_type: ResultColumnType,
}

/// Execution metadata returned alongside the rows. `truncated` is the signal a
/// caller needs: when `true` the result hit a server-enforced row/byte cap and
/// is incomplete — the caller must narrow the query rather than assume it saw
/// everything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct QueryStats {
    /// Number of rows actually returned (after any cap).
    pub row_count: u64,
    /// Approximate serialized byte size of the returned rows.
    pub byte_count: u64,
    /// Wall-clock execution time in milliseconds.
    pub elapsed_ms: u64,
    /// `true` if a row/byte cap stopped the stream before it completed.
    pub truncated: bool,
}
