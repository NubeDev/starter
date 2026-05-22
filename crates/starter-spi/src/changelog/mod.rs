//! Append-only change envelope shared by audit, agent-log, undo/redo,
//! duplicate, and copy/paste.
//!
//! This module defines the seam between starter and consumer code:
//! the [`Change`] envelope, the [`ChangeRecorder`] / [`ChangeTx`]
//! traits the backends implement, and the [`Reversible`] trait the
//! consumer implements once per resource kind.
//!
//! No SQL, no transports, no defaults here — this is a contract crate.
//! See `DOCS/backend/undo-redo/SCOPE.md`.

mod actor;
mod change;
mod ids;
mod op;
mod recorder;
mod reversible;
mod visibility;

pub use actor::Actor;
pub use change::Change;
pub use ids::{ChangeId, GroupId, TraceId};
pub use op::Op;
pub use recorder::{ChangeRecorder, ChangeTx};
pub use reversible::Reversible;
pub use visibility::ChangelogVisibility;
