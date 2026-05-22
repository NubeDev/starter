//! # starter-changelog
//!
//! Append-only change envelope shared by audit, agent-log, undo/redo,
//! duplicate, and copy/paste. This crate owns the read-side query API
//! ([`ChangeLog`]), the default [`ChangelogVisibilityRegistry`], the
//! [`Prune`] trait, and the [`ChangeTail`] subscription trait.
//!
//! No SQL — backends live in `starter-changelog-sqlite` and
//! `starter-changelog-postgres`. The write-side traits ([`Recorder`],
//! [`Tx`], [`Reversible`]) live in `starter-spi` behind the
//! `changelog` feature and are re-exported here for ergonomics.
//!
//! See `DOCS/backend/undo-redo/SCOPE.md`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod prune;
mod query;
mod tail;
mod visibility;

pub use prune::{Prune, PruneReport, PruneRequest};
pub use query::{filter_for_actor, filter_for_resource, ChangeFilter, ChangeLog, ChangePage};
pub use tail::ChangeTail;
pub use visibility::ChangelogVisibilityRegistry;

/// Re-export of the spi write-side recorder trait.
pub use starter_spi::changelog::ChangeRecorder as Recorder;
/// Re-export of the spi write-side transaction handle.
pub use starter_spi::changelog::ChangeTx as Tx;
/// Re-export of the consumer-side extension trait.
pub use starter_spi::changelog::Reversible;
/// Re-export of the per-kind ACL trait.
pub use starter_spi::changelog::ChangelogVisibility;

/// Re-export of the envelope types.
pub use starter_spi::changelog::{Actor, Change, ChangeId, GroupId, Op, TraceId};
