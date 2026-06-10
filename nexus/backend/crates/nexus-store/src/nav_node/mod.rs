//! Tenant-scoped navigation-tree persistence (WS-13).
//!
//! A nav node mounts a (possibly shared) dashboard page — or a static app route,
//! or is a plain group header — into a nestable tree, and is the unit access is
//! granted on. Like folders, every function runs inside a tenant-bound
//! transaction so RLS isolates the rows, and nodes nest via a self-referential
//! `parent_id` (NULL = root). Deleting a node re-roots its children rather than
//! cascading. `target`/`context` are opaque JSONB here — the API DTO owns and
//! validates their shapes; the store only persists them.
//!
//! When a dashboard is deleted, its dependent nodes are swept back to
//! `{ kind: "group" }` via [`sweep_dashboard_targets`], called from the dashboard
//! delete path so losing a page never loses the nav node.

mod delete;
mod fetch;
mod insert;
mod update;

pub mod record;

pub use delete::{delete, sweep_dashboard_targets};
pub use fetch::{by_id, list};
pub use insert::{insert, insert_with_id};
pub use record::{NavNodePatch, NavNodeRecord, NewNavNode};
pub use update::update;
