//! Tenant-scoped saved-flow persistence.
//!
//! Mirrors the datasource/dashboard stores: every function opens a tenant-bound
//! transaction so RLS isolates the rows, and reads key on the immutable id. The
//! three config blobs are stored as jsonb and returned verbatim — validation is
//! the FlowManager's job at build time, not the store's.

mod delete;
mod fetch;
mod insert;
mod record;
mod update;

pub use delete::delete;
pub use fetch::{get, list};
pub use insert::insert;
pub use record::{FlowPatch, FlowRecord, NewFlow};
pub use update::{set_enabled, update};
