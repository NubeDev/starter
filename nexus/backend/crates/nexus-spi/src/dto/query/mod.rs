//! One-shot query request/response — the M0 contract.

mod run;
mod shared;

pub use run::{QueryRequest, QueryResponse};
pub use shared::{ColumnSchema, QueryStats, ResultColumnType};
