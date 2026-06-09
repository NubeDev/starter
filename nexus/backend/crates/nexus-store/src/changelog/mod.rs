//! Tenant-scoped change ledger — the persistence half of WS-12 audit + undo.
//!
//! The platform `starter-changelog-postgres` backend owns a single shared
//! `starter_changes` table with no tenant column. nexus is multi-tenant with
//! Postgres RLS, so it ships its own recorder/changelog over `nexus_changes`
//! (migration `1601_changelog.sql`): identical row shape plus a `tenant_id`
//! column and an RLS policy. Both the write path ([`NexusRecorder`]) and the read
//! path ([`NexusChangeLog`]) run inside a [`crate::tenant_tx`] so the
//! `app.tenant_id` GUC is bound and a caller can never reach another tenant's
//! rows. The starter `Change`/`Actor`/`Op`/`Reversible` types and the
//! `ReversibleRegistry`/`UndoService` are reused unchanged — only the SQL is
//! nexus-local.

mod codec;
mod log;
mod prune;
mod recorder;

pub use log::NexusChangeLog;
pub use prune::prune_aged;
pub use recorder::NexusRecorder;
