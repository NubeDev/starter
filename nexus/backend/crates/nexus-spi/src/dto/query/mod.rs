//! One-shot query request/response — the M0 contract.

mod kinds;
mod run;
mod shared;

pub use kinds::{QueryKindList, QueryKindSummary};
pub use run::{FederatedSourceRef, QueryRequest, QueryResponse, QueryTimeRange, QueryVariable};
pub use shared::{ColumnSchema, QueryStats, ResultColumnType};
