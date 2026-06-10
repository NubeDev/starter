//! Lifecycle audit seam — an optional sink the admin handlers notify on every
//! enable / disable / install / uninstall, carrying the acting [`Principal`].
//!
//! The kernel persists *enablement state* (one row per id via the
//! [`EnablementStore`](crate::store::EnablementStore)) but that row is a
//! current-value, not an audit trail: it records the latest state and who set it
//! (`updated_by`), not the sequence of admin actions. Consumers that keep an
//! append-only changelog (nexus's `nexus_changes`, rubix's audit ledger) need to
//! record each lifecycle mutation *with the acting principal* as it happens.
//!
//! The enable/disable handlers do not otherwise extract the principal, so this
//! seam is the single place a consumer learns "admin X enabled extension Y at
//! time T". It is **optional and additive**: when no sink is wired the handlers
//! call [`NoopAuditSink`] and behave exactly as before. A sink failure is logged
//! by the handler and never fails the mutation — audit is observational, it must
//! not block an operator action.

use async_trait::async_trait;

use starter_spi::auth::Principal;

/// A lifecycle action the admin surface performs on an extension. Stable wire
/// tokens a consumer maps onto its own audit verbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleAction {
    /// `POST /extensions/<id>/enable` — extension turned on (and, for a
    /// process flavour, its supervisor spawned).
    Enabled,
    /// `POST /extensions/<id>/disable` — extension turned off (supervisor, if
    /// any, shut down).
    Disabled,
    /// `POST /extensions/install` — a tarball was unpacked and registered
    /// (live on next boot).
    Installed,
    /// `DELETE /extensions/<id>` — the bundle was removed (optionally with a
    /// `?purge=true` cleanup pass).
    Uninstalled,
}

impl LifecycleAction {
    /// A lowercase, stable string token for the action — handy for a consumer
    /// that records the verb as text.
    pub fn as_str(self) -> &'static str {
        match self {
            LifecycleAction::Enabled => "enabled",
            LifecycleAction::Disabled => "disabled",
            LifecycleAction::Installed => "installed",
            LifecycleAction::Uninstalled => "uninstalled",
        }
    }
}

/// A consumer-supplied sink the admin handlers notify after a successful
/// lifecycle mutation. Wired via
/// [`ExtensionAdminBuilder::with_audit_sink`](crate::ExtensionAdminBuilder::with_audit_sink).
///
/// `principal` is the authenticated admin from the request, when present (the
/// routes are `Role::Admin`-gated, so in production it is always `Some`; it is
/// `None` only on the unauthenticated `TestApp` router). Implementations must
/// not panic and should treat their own failures as non-fatal — the handler
/// logs and continues.
#[async_trait]
pub trait AuditSink: Send + Sync + 'static {
    /// Record that `action` happened to `extension_id`, performed by
    /// `principal`. Called after the mutation's own persistence succeeds.
    async fn record(
        &self,
        action: LifecycleAction,
        extension_id: &str,
        principal: Option<&Principal>,
    );
}

/// The default sink — does nothing. Used when a host does not wire its own, so
/// the admin handlers can call the sink unconditionally.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAuditSink;

#[async_trait]
impl AuditSink for NoopAuditSink {
    async fn record(
        &self,
        _action: LifecycleAction,
        _extension_id: &str,
        _principal: Option<&Principal>,
    ) {
    }
}
