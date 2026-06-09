//! Tenant-scoped dashboard-variable persistence (WS-02).
//!
//! Like dashboards and panels, every function runs inside a tenant-bound
//! transaction so RLS isolates the rows. Variables hang off a dashboard (FK with
//! cascade delete) but carry their own tenant column so the policy applies
//! directly. The store keeps the variable's kind as its wire string; the DTO
//! layer owns the enum.

mod delete;
mod fetch;
mod insert;
mod record;
mod update;

pub use delete::delete;
pub use fetch::{by_id, list_for_dashboard};
pub use insert::insert;
pub use record::{NewVariable, VariablePatch, VariableRecord};
pub use update::update;
