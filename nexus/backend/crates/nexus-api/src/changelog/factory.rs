//! Boot-time and per-request construction of the audit/undo handles.
//!
//! The registry and redo cursor are built once at boot and shared (cheap `Arc`
//! handles on [`AppState`]); the tenant-pinned log and recorder are built per
//! request because they carry the caller's tenant for RLS binding.

use std::sync::Arc;

use sqlx::PgPool;
use starter_changelog::ChangeLog;
use starter_undo::cursor_postgres::PgUndoCursor;
use starter_undo::{ReversibleRegistry, UndoService};

use nexus_store::datasource::Envelope;

use crate::reversible::register_all;

/// Shared, boot-built handles held on [`crate::state::AppState`]. Both are cheap
/// to clone (an `Arc` and a pool-wrapping cursor).
#[derive(Clone)]
pub struct ChangelogHandles {
    /// Per-kind [`starter_spi::changelog::Reversible`] lookup, built at boot.
    pub registry: Arc<ReversibleRegistry>,
    /// Per-actor redo stack, persisted in `starter_undo_cursors`.
    pub cursor: Arc<PgUndoCursor>,
}

impl ChangelogHandles {
    /// Assemble the boot handles over the metadata pool. The secret envelope is
    /// passed through to the reversibles for secret-bearing kinds (datasources).
    pub fn new(metadata: PgPool, envelope: Envelope) -> Self {
        Self {
            registry: Arc::new(build_registry(metadata.clone(), envelope)),
            cursor: Arc::new(PgUndoCursor::new(
                starter_store_postgres::Pool::from_sqlx(metadata),
            )),
        }
    }
}

/// Build the [`ReversibleRegistry`] with every nexus reversible kind registered.
/// One line per kind; the impls close over the metadata pool (and the secret
/// envelope for secret-bearing kinds) so undo can apply inverses against the
/// store.
pub fn build_registry(metadata: PgPool, envelope: Envelope) -> ReversibleRegistry {
    register_all(ReversibleRegistry::new(), metadata, envelope)
}

/// Build the per-request [`UndoService`]: a tenant-pinned change log over
/// `nexus_changes`, the shared registry, and the shared redo cursor. The service
/// only ever reads/writes the caller's tenant because the log binds
/// `app.tenant_id` on every query.
pub fn undo_service_for(handles: &ChangelogHandles, metadata: PgPool, tenant_id: &str) -> UndoService {
    let log: Arc<dyn ChangeLog> =
        Arc::new(nexus_store::changelog::NexusChangeLog::new(metadata, tenant_id));
    UndoService::with_cursor(log, handles.registry.clone(), handles.cursor.clone())
}
