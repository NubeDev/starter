//! Query-history wire types — recall, re-run, star past queries.

mod entry;

pub use entry::{QueryHistoryEntry, QueryHistoryList, StarQueryRequest};
