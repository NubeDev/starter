//! Tenant-scoped query-kind persistence.
//!
//! Mirrors the agent store: every function opens a tenant-bound transaction so
//! RLS isolates the rows, and reads key on the immutable id. A *query-kind* is a
//! tenant-authored named SQL query promoted from an Explore session. `sql`,
//! `params_schema`, and `tables` are stored and returned verbatim — the API lint
//! validated them before insert, the store only persists them.

mod delete;
mod fetch;
mod insert;
mod record;
mod update;

pub use delete::delete;
pub use fetch::{get, get_by_name, list};
pub use insert::insert;
pub use record::{NewQueryKind, QueryKindPatch, QueryKindRecord};
pub use update::update;
