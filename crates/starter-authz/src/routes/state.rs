//! Shared state for the `/v1/authz/*` admin routes. Holds the
//! engine handle (so writes can `reload()` the cache) and the
//! registry handle (so `GET /v1/authz/resources` can enumerate).

use std::sync::Arc;

use starter_spi::authz::ResourceRegistry;

use crate::db_engine::DbPolicyEngine;

/// Bundle passed to every `/v1/authz/*` handler. Cheap to clone.
#[derive(Clone)]
pub struct AuthzRoutesState {
    /// DB-backed engine. Handlers call `engine.reload()` after
    /// every successful write so the cache stays in sync with the
    /// store.
    pub engine: Arc<DbPolicyEngine>,
    /// Resource registry — `GET /v1/authz/resources` enumerates
    /// this so the admin UI knows what (resource, action) pairs
    /// are valid targets for new rules.
    pub registry: Arc<dyn ResourceRegistry>,
}
