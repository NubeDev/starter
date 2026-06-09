//! Audit + undo substrate (WS-12) — the nexus integration of the platform
//! changelog.
//!
//! Audit and undo are one append-only ledger, not two systems. nexus reuses the
//! platform `Change`/`Actor`/`Reversible` model, the `ReversibleRegistry`, the
//! `UndoService`, and the Postgres redo cursor, and supplies the tenant-aware
//! `NexusRecorder`/`NexusChangeLog` (in `nexus-store`) plus the per-request wiring
//! here:
//!
//! - [`actor_from`] maps an authenticated [`Principal`] to a changelog [`Actor`].
//! - [`record`] is the tenant-pinned `record_if_reversible` each mutating handler
//!   calls right after a successful domain mutation (C6 convention).
//! - [`undo_service_for`] builds the per-request [`UndoService`] (tenant-pinned
//!   log + shared registry + shared redo cursor).
//! - [`build_registry`] assembles the [`ReversibleRegistry`] at boot.
//! - [`known_mutable_kinds`] is the coverage-guard manifest (see `reversible`).
//! - [`policy::RetentionPolicy`] + [`prune`] keep the append-only ledger bounded:
//!   a background sweep deletes rows past the retention horizon.

mod factory;
pub mod policy;
pub mod prune;
mod record;

pub use factory::{build_registry, undo_service_for, ChangelogHandles};
pub use policy::RetentionPolicy;
pub use record::{actor_from, record};
