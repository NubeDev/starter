//! Tenant-scoped findings persistence: the persistent "sparks" a detection run
//! emits, one per flagged target, with an open → acknowledged → resolved
//! lifecycle.
//!
//! Two write paths: [`upsert::reconcile`] is the runner's per-run dedup +
//! auto-resolve, and [`browse`] is the API's read + manual ack/resolve. This
//! generalises [`crate::alert::event`] — an append-only event becomes a
//! per-target record with a workflow.

pub mod browse;
pub mod record;
pub mod upsert;

pub use browse::{acknowledge, get, list, resolve};
pub use record::{FindingFilter, FindingRecord, FindingTransition, NewFinding, Reconciled};
pub use upsert::{latest_open_at, reconcile};
