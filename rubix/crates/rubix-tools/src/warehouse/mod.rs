//! warehouse goal — persistence tools (L1+).
//!
//! See `rubix/docs/sessions/data-flow/02-ingest-l1.md` for the
//! framework: persistence is a tool, delivery is a flow.

pub mod ingest;
pub mod clean_minute;
pub mod anomaly_gate;
