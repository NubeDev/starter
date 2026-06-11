//! Tenant-scoped detection persistence: the saved analytic rules that run on a
//! schedule to emit findings.
//!
//! Every per-tenant function opens a tenant-bound transaction so RLS isolates
//! the rows. The one exception is [`due::claim_due`], the runner's cross-tenant
//! claim, which goes through a SECURITY DEFINER function — the single controlled
//! hole the system task needs, mirroring [`crate::alert::due`].

pub mod crud;
pub mod due;
pub mod record;
pub mod stats;

pub use crud::{delete, get, insert, list, update};
pub use due::{claim_due, DueDetection};
pub use record::{DetectionPatch, DetectionRecord, DetectionStats, NewDetection};
pub use stats::stats;
