//! Tenant-scoped dashboard and panel persistence.
//!
//! Like datasources, every function runs inside a tenant-bound transaction so
//! RLS isolates the rows, and reads key on the immutable id (a slug resolves to
//! an id via [`by_slug`]). Panels hang off a dashboard but carry their own
//! tenant column so the policy applies directly.

mod delete;
mod fetch;
mod insert;
pub mod panel;
mod record;

pub use delete::delete;
pub use fetch::{by_slug, list};
pub use insert::insert;
pub use record::{DashboardRecord, NewDashboard, NewPanel, PanelPatch, PanelRecord};
