//! Construct the shared preference handles held on [`AppState`].
//!
//! One `PgPrefsStore` (over the metadata pool) and the platform
//! [`SystemDefaults`] back both the `/me/preferences` routes and the
//! `Accept-Units` units-conversion middleware, so the resolver sees the same
//! rows the routes write. Bundled into [`NexusPrefs`] (cheap to clone — an `Arc`
//! plus a small value struct) for the state field.

use std::sync::Arc;

use sqlx::PgPool;
use starter_prefs::resolver::SystemDefaults;
use starter_prefs::store::PgPrefsStore;

/// The preference handles shared across requests. Cloneable: the store is an
/// `Arc` and the defaults a small `Arc`-wrapped value, so this is a couple of
/// pointer copies.
#[derive(Clone)]
pub struct NexusPrefs {
    /// Postgres-backed preference persistence over the metadata pool.
    pub store: Arc<PgPrefsStore>,
    /// The last-resort layer for the three-layer resolver — the platform
    /// `en-US` / UTC / metric defaults.
    pub defaults: Arc<SystemDefaults>,
}

/// Build the preference handles over the metadata pool. The prefs tables live in
/// the metadata DB (migration `1501_prefs.sql`), so the store shares the control
/// plane's pool rather than opening its own.
pub fn prefs_store(metadata: PgPool) -> NexusPrefs {
    NexusPrefs {
        store: Arc::new(PgPrefsStore::new(metadata)),
        defaults: Arc::new(SystemDefaults::starter()),
    }
}
