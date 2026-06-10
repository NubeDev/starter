//! nexus's [`AuditSink`] — records extension lifecycle mutations into the
//! `nexus_changes` audit ledger with the acting principal (WS-14 §5 Q4 + the
//! audit acceptance criterion).
//!
//! Extensions are **global config**, not per-tenant reversible resources, so
//! these are **audit-only** entries (no undo cursor, no `Reversible`
//! registration) — consistent with how WS-12 treats file-pack kinds. The entry
//! is recorded under the **acting admin's tenant**: the admin who performed the
//! action has a tenant context, and their own tenant view is the natural place
//! the audit trail surfaces. An action by a super-admin with no tenant (`"*"` or
//! `None`) is recorded under a sentinel tenant so it is never silently dropped.
//!
//! Wired into the kernel via `ExtensionAdminBuilder::with_audit_sink`. The
//! kernel calls [`AuditSink::record`] after each successful enable / disable /
//! install / uninstall, passing the `Principal` it extracted from the request.
//! A failure here is logged and swallowed — audit must never fail an operator
//! action that already took effect.

use async_trait::async_trait;
use sqlx::PgPool;
use starter_ext_server::{AuditSink, LifecycleAction};
use starter_spi::auth::Principal;
use starter_spi::changelog::{Actor, Op};

/// Records extension lifecycle actions to `nexus_changes`. Holds the metadata
/// pool; cheap to clone.
#[derive(Clone)]
pub struct NexusExtensionAudit {
    metadata: PgPool,
}

impl NexusExtensionAudit {
    /// Build from the metadata pool.
    pub fn new(metadata: PgPool) -> Self {
        Self { metadata }
    }
}

/// The resource kind recorded for an extension lifecycle audit row. Matches the
/// authz/resource naming convention (lowercase, singular).
const RESOURCE_KIND: &str = "extension";

/// Sentinel tenant for an action by a principal with no concrete tenant (a
/// super-admin scoped to `"*"`). Keeps a global action attributable rather than
/// dropping the audit row when there is no tenant to key it under.
const GLOBAL_TENANT: &str = "_global";

#[async_trait]
impl AuditSink for NexusExtensionAudit {
    async fn record(
        &self,
        action: LifecycleAction,
        extension_id: &str,
        principal: Option<&Principal>,
    ) {
        let actor = match principal {
            Some(p) => Actor::User {
                subject: p.subject.clone(),
            },
            // The routes are Role::Admin-gated, so in production a principal is
            // always present; record `system` only on the unauthenticated test
            // router so the entry is still attributable.
            None => Actor::User {
                subject: "system".to_string(),
            },
        };

        // The acting admin's tenant, falling back to the global sentinel for a
        // super-admin with no concrete tenant (`"*"` is the super-admin marker).
        let tenant = principal
            .and_then(|p| p.tenant_id.as_deref())
            .filter(|t| !t.is_empty() && *t != "*")
            .unwrap_or(GLOBAL_TENANT)
            .to_string();

        let op = Op::Custom(action.as_str().to_string());

        if let Err(e) = nexus_store::changelog::record_audit(
            &self.metadata,
            &tenant,
            &actor,
            &op,
            RESOURCE_KIND,
            extension_id,
        )
        .await
        {
            // Non-fatal: the lifecycle mutation already succeeded. Log loudly so
            // a persistently-failing audit path is visible without breaking the
            // operator action.
            tracing::warn!(
                target: "nexus_api::extensions::audit",
                extension = %extension_id,
                action = %action.as_str(),
                error = %e,
                "recording extension lifecycle audit failed"
            );
        }
    }
}
