//! Shared state for the `/v1/authz/*` admin routes. Holds the
//! engine handle (so writes can `reload()` the cache) and the
//! registry handle (so `GET /v1/authz/resources` can enumerate).

use std::sync::Arc;

use starter_spi::authz::ResourceRegistry;

use crate::audit::DbDecisionSink;
use crate::db_engine::DbPolicyEngine;
use crate::instances::InstancesRegistry;

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
    /// Phase 7c — decision audit sink. When `Some`, the
    /// `GET /v1/authz/decisions` route is mounted; otherwise it
    /// returns `404`.
    pub decision_sink: Option<Arc<DbDecisionSink>>,
    /// G2 — per-kind instances providers. When `Some`,
    /// `GET /v1/authz/resources/:kind/instances` consults it; when
    /// `None`, the endpoint always returns 404.
    pub instances: Option<Arc<InstancesRegistry>>,
}

impl AuthzRoutesState {
    /// Construct with no audit sink — preserves the pre-Phase-7c
    /// router shape.
    pub fn new(engine: Arc<DbPolicyEngine>, registry: Arc<dyn ResourceRegistry>) -> Self {
        Self {
            engine,
            registry,
            decision_sink: None,
            instances: None,
        }
    }

    /// Attach an audit sink so `GET /v1/authz/decisions` can read
    /// the table.
    pub fn with_decision_sink(mut self, sink: Arc<DbDecisionSink>) -> Self {
        self.decision_sink = Some(sink);
        self
    }

    /// Attach an instances registry so per-kind instance lookups
    /// resolve. Without this the `/instances` endpoint 404s.
    pub fn with_instances(mut self, registry: Arc<InstancesRegistry>) -> Self {
        self.instances = Some(registry);
        self
    }
}
