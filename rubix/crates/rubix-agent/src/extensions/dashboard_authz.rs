//! Concrete rubix-side impls of the Row-5 SDK backends
//! [`DashboardBackend`] and [`AuthzBackend`].
//!
//! Both backends are minted per-call by the
//! [`super::RubixCapabilityFactory`] and bound to the inbound
//! frame's [`CallerIdentity`]. A system / host-internal frame
//! (`tenant_id.is_none()`) is refused with [`Error::Capability`]
//! — the same fail-closed gate the warehouse + event-bus backends
//! use. The host trusts the SDK to never call these handles from
//! a system context; the backend re-checks anyway because that
//! soft-trust boundary is exactly what the substrate exists to
//! enforce.
//!
//! # Sync ↔ async bridge
//!
//! The SDK trait methods are sync (so extension code can call
//! `ctx.dashboard().read(...)` without `.await`); the underlying
//! [`DashboardStore`] and [`PolicyEngine`] are `async`. We bridge
//! identically to [`super::RubixWarehouseReadBackend`]:
//! `tokio::task::block_in_place` + `Handle::current().block_on`.
//! The dispatcher runs every per-call invocation on the
//! multi-thread tokio runtime, so this is safe; the SDK doc-strings
//! call out that capability accessors may block briefly.
//!
//! # Slim v0.1 surface
//!
//! Both backends ship the minimum needed for an extension to use
//! the host's dashboard store and authz engine without loopback
//! HTTP. We deliberately don't ship:
//!
//! - **Manifest grants** (`Capability::DashboardRead` /
//!   `DashboardWrite` / `AuthzCheck`). The SPI capability enum
//!   doesn't carry these variants yet; without them, the
//!   per-extension allowlist gate has no input. We rely on the
//!   tenancy clamp (dashboard) and the policy-engine evaluation
//!   (authz) as the only enforcement points. The pattern in
//!   [`super::backends::RubixCapabilityFactory::warehouse_grant`]
//!   is the template once those variants land.
//! - **Principal reconstruction past tenancy + role.** The
//!   substrate `CallerIdentity` carries `tenant_id`, `user_id`,
//!   and a `Vec<String>` of role labels — not a full
//!   [`Principal`] with scopes / teams / extra claims. We
//!   reconstruct a minimal `Principal` from the available fields,
//!   parsing the role label via [`Role`]'s `Debug` form (the
//!   shape the caller-identity middleware writes). Engines that
//!   evaluate `principal.teams` or `principal.scopes` will see
//!   the empty defaults — accepted as a v0.1 trade-off and
//!   called out in the status memory.

use std::collections::BTreeSet;
use std::sync::Arc;

use rubix_spi::dashboard::{DashboardStore, DashboardStoreError, NewRevision};
use starter_ext_sdk::ctx::{AuthzBackend, DashboardBackend};
use starter_ext_spi::identity::CallerIdentity;
use starter_ext_spi::{Error, Result};
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{PolicyEngine, ResourceRef};

/// Per-call rubix [`DashboardBackend`].
///
/// `caller_tenant_id` is the lynchpin: every read and write is
/// clamped to it; a `None` value (system frame) refuses the call
/// with [`Error::Capability`]. `caller_user_id` is recorded as the
/// `created_by` / `owner_principal` on writes so the audit row
/// attributes the change to the right principal.
#[derive(Clone)]
pub struct RubixDashboardBackend {
    store: Arc<dyn DashboardStore>,
    caller_tenant_id: Option<String>,
    caller_user_id: Option<String>,
    /// Page ids the manifest's `dashboard_read` grant permits.
    /// `None` ⇒ host-internal frame (gate skipped); `Some(empty)`
    /// ⇒ neutralised grant (every read refused); `Some(set)` ⇒
    /// allowlist.
    granted_read_pages: Option<BTreeSet<String>>,
    /// Same shape as [`Self::granted_read_pages`] but for
    /// `dashboard_write`.
    granted_write_pages: Option<BTreeSet<String>>,
}

impl std::fmt::Debug for RubixDashboardBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RubixDashboardBackend")
            .field("caller_tenant_id", &self.caller_tenant_id)
            .field("caller_user_id", &self.caller_user_id)
            .field("granted_read_pages", &self.granted_read_pages)
            .field("granted_write_pages", &self.granted_write_pages)
            .finish_non_exhaustive()
    }
}

impl RubixDashboardBackend {
    /// New backend bound to the caller-side tenancy + user id,
    /// with manifest-resolved page allowlists. Pass `None` for
    /// either allowlist to opt out of the per-page gate (the
    /// host-internal posture).
    pub fn new(
        store: Arc<dyn DashboardStore>,
        caller_tenant_id: Option<String>,
        caller_user_id: Option<String>,
        granted_read_pages: Option<BTreeSet<String>>,
        granted_write_pages: Option<BTreeSet<String>>,
    ) -> Self {
        Self {
            store,
            caller_tenant_id,
            caller_user_id,
            granted_read_pages,
            granted_write_pages,
        }
    }

    fn check_read_grant(&self, page_id: &str) -> Result<()> {
        if let Some(grant) = &self.granted_read_pages {
            if !grant.contains(page_id) {
                return Err(Error::capability(format!(
                    "dashboard.read {page_id:?} refused: not in calling extension's \
                     `dashboard_read.pages` grant"
                )));
            }
        }
        Ok(())
    }

    fn check_write_grant(&self, page_id: &str) -> Result<()> {
        if let Some(grant) = &self.granted_write_pages {
            if !grant.contains(page_id) {
                return Err(Error::capability(format!(
                    "dashboard.write {page_id:?} refused: not in calling extension's \
                     `dashboard_write.pages` grant"
                )));
            }
        }
        Ok(())
    }

    fn tenant(&self) -> Result<&str> {
        self.caller_tenant_id.as_deref().ok_or_else(|| {
            Error::capability(
                "dashboard: system / host-internal frame has no caller; \
                 refuse rather than expose another tenant's pages",
            )
        })
    }

    fn user(&self) -> Result<&str> {
        self.caller_user_id.as_deref().ok_or_else(|| {
            Error::capability("dashboard: caller has no user_id; writes need an audit principal")
        })
    }
}

impl DashboardBackend for RubixDashboardBackend {
    fn read(&self, page_id: &str) -> Result<serde_json::Value> {
        self.check_read_grant(page_id)?;
        let tenant = self.tenant()?;
        let row = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.store.get_active(tenant, page_id))
        });
        match row {
            Ok(Some(rev)) => Ok(rev.body_json),
            Ok(None) => Err(Error::extension_internal(format!(
                "dashboard page `{tenant}:{page_id}` not found"
            ))),
            Err(DashboardStoreError::NotFound { tenant_id, page_id }) => {
                Err(Error::extension_internal(format!(
                    "dashboard page `{tenant_id}:{page_id}` not found"
                )))
            }
            Err(DashboardStoreError::Backend(msg)) => Err(Error::extension_internal(format!(
                "dashboard backend: {msg}"
            ))),
        }
    }

    fn write(&self, page_id: &str, body: serde_json::Value) -> Result<()> {
        self.check_write_grant(page_id)?;
        let tenant = self.tenant()?.to_owned();
        let user = self.user()?.to_owned();
        let new_revision = NewRevision {
            page_id: page_id.to_owned(),
            tenant_id: tenant,
            owner_principal: user.clone(),
            // v0.1: extension-driven writes carry no title / tags.
            // The admin UI fills these in when an operator edits;
            // an extension that needs them today must round-trip
            // through `rubix.dashboard.update` (which the next
            // slice can replace with a richer SDK accessor).
            title: String::new(),
            tags: Vec::new(),
            body_json: body,
            created_by: user,
        };
        let outcome = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.store.insert_revision(new_revision))
        });
        match outcome {
            Ok(_rev) => Ok(()),
            Err(DashboardStoreError::NotFound { tenant_id, page_id }) => {
                Err(Error::extension_internal(format!(
                    "dashboard page `{tenant_id}:{page_id}` not found"
                )))
            }
            Err(DashboardStoreError::Backend(msg)) => Err(Error::extension_internal(format!(
                "dashboard backend: {msg}"
            ))),
        }
    }
}

/// Per-call rubix [`AuthzBackend`].
///
/// Bound to a [`Principal`] reconstructed from the inbound frame's
/// [`CallerIdentity`]. A system frame (no `tenant_id` AND no
/// `user_id`) is refused with [`Error::Capability`].
#[derive(Clone)]
pub struct RubixAuthzBackend {
    engine: Arc<dyn PolicyEngine>,
    /// `None` for a system / host-internal frame.
    principal: Option<Principal>,
    /// Resource kinds the manifest's `authz_check` grant permits.
    /// `None` ⇒ host-internal frame (gate skipped); `Some(set)`
    /// ⇒ allowlist (empty set ⇒ neutralised grant, every check
    /// refused).
    granted_kinds: Option<BTreeSet<String>>,
}

impl std::fmt::Debug for RubixAuthzBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RubixAuthzBackend")
            .field(
                "principal",
                &self.principal.as_ref().map(|p| p.subject.as_str()),
            )
            .field("granted_kinds", &self.granted_kinds)
            .finish_non_exhaustive()
    }
}

impl RubixAuthzBackend {
    /// New backend bound to a principal reconstructed from `caller`,
    /// with a manifest-resolved resource-kind allowlist. Pass `None`
    /// for `granted_kinds` to opt out of the per-kind gate.
    ///
    /// Prefer [`Self::with_principal`] when the inbound transport
    /// has the full [`Principal`] — the reconstruction here drops
    /// `scopes` / `teams` / `extra` (see module docs).
    pub fn new(
        engine: Arc<dyn PolicyEngine>,
        caller: Option<&CallerIdentity>,
        granted_kinds: Option<BTreeSet<String>>,
    ) -> Self {
        let principal = caller.and_then(principal_from_caller);
        Self {
            engine,
            principal,
            granted_kinds,
        }
    }

    /// New backend bound to a pre-resolved `Principal`. Skip the
    /// lossy `CallerIdentity` round-trip — the policy engine sees
    /// the full identity (including scopes, teams, extra) that the
    /// authenticator originally minted.
    pub fn with_principal(
        engine: Arc<dyn PolicyEngine>,
        principal: Principal,
        granted_kinds: Option<BTreeSet<String>>,
    ) -> Self {
        Self {
            engine,
            principal: Some(principal),
            granted_kinds,
        }
    }

    fn check_kind_grant(&self, kind: &str) -> Result<()> {
        if let Some(grant) = &self.granted_kinds {
            if !grant.contains(kind) {
                return Err(Error::capability(format!(
                    "authz.check {kind:?} refused: not in calling extension's \
                     `authz_check.kinds` grant"
                )));
            }
        }
        Ok(())
    }

    fn principal(&self) -> Result<&Principal> {
        self.principal.as_ref().ok_or_else(|| {
            Error::capability(
                "authz: system / host-internal frame has no principal; \
                 refuse rather than evaluate against an anonymous identity",
            )
        })
    }
}

impl AuthzBackend for RubixAuthzBackend {
    fn check(&self, action: &str, resource: &str) -> Result<bool> {
        let principal = self.principal()?;
        // `resource` arrives as `kind` or `kind:id`. The engine
        // wants a `ResourceRef`; build a collection-level ref when
        // no id is present, a row-level ref otherwise. Tenancy is
        // populated from the principal so tenant-scoped kinds get
        // a coherent cross-tenant predicate input.
        let (kind, id) = match resource.split_once(':') {
            None => (resource, None),
            Some((k, i)) => (k, Some(i.to_owned())),
        };
        // Manifest gate: refuse before consulting the engine if
        // the kind is outside the extension's `authz_check` grant.
        // The engine's own deny is fine but does extra work; the
        // gate also keeps the engine's audit log from filling up
        // with `unknown_resource` lines on probes the extension
        // shouldn't be making in the first place.
        self.check_kind_grant(kind)?;
        let object = ResourceRef {
            kind: kind.to_owned(),
            id,
            owner: None,
            tenant: principal.tenant_id.clone(),
        };
        let decision = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.engine.check(principal, action, &object))
        });
        Ok(decision.is_allow())
    }
}

/// Reconstruct a minimal [`Principal`] from a [`CallerIdentity`].
///
/// Returns `None` for a true system frame (no tenant AND no user).
/// The synthesised principal carries the parsed role, the
/// tenant id, and an empty `scopes` / `teams` / `extra` —
/// engines that only consult `subject` / `role` / `tenant_id`
/// (the rubix v0 posture) see the same answer as if the
/// authenticator had built the principal directly. See module
/// docs for the trade-off.
fn principal_from_caller(caller: &CallerIdentity) -> Option<Principal> {
    if caller.is_system() {
        return None;
    }
    let role = caller
        .roles
        .iter()
        .find_map(|r| match r.as_str() {
            "Admin" => Some(Role::Admin),
            "Writer" => Some(Role::Writer),
            "Reader" => Some(Role::Reader),
            _ => None,
        })
        .unwrap_or(Role::Reader);
    Some(Principal {
        subject: caller.user_id.clone().unwrap_or_default(),
        role,
        scopes: Vec::new(),
        tenant_id: caller.tenant_id.clone(),
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rubix_spi::dashboard::{DashboardRevision, InsertOutcome, ListFilter};
    use starter_spi::authz::Decision;

    // Tiny in-memory dashboard store.
    #[derive(Default)]
    struct MemStore {
        inner: std::sync::Mutex<std::collections::HashMap<(String, String), serde_json::Value>>,
    }

    #[async_trait]
    impl DashboardStore for MemStore {
        async fn insert_revision(
            &self,
            new: NewRevision,
        ) -> std::result::Result<DashboardRevision, DashboardStoreError> {
            self.inner.lock().unwrap().insert(
                (new.tenant_id.clone(), new.page_id.clone()),
                new.body_json.clone(),
            );
            Ok(DashboardRevision {
                page_id: new.page_id,
                revision_id: "r-1".into(),
                tenant_id: new.tenant_id,
                owner_principal: new.owner_principal,
                title: new.title,
                tags: new.tags,
                body_json: new.body_json,
                created_by: new.created_by,
                created_at: "1970-01-01T00:00:00Z".into(),
                superseded_at: None,
            })
        }

        async fn insert_revision_with_prior(
            &self,
            new: NewRevision,
        ) -> std::result::Result<InsertOutcome, DashboardStoreError> {
            let inserted = self.insert_revision(new).await?;
            Ok(InsertOutcome {
                inserted,
                prior: None,
            })
        }

        async fn get_active(
            &self,
            tenant_id: &str,
            page_id: &str,
        ) -> std::result::Result<Option<DashboardRevision>, DashboardStoreError> {
            let map = self.inner.lock().unwrap();
            Ok(map
                .get(&(tenant_id.to_owned(), page_id.to_owned()))
                .map(|body| DashboardRevision {
                    page_id: page_id.to_owned(),
                    revision_id: "r-1".into(),
                    tenant_id: tenant_id.to_owned(),
                    owner_principal: "u-1".into(),
                    title: String::new(),
                    tags: Vec::new(),
                    body_json: body.clone(),
                    created_by: "u-1".into(),
                    created_at: "1970-01-01T00:00:00Z".into(),
                    superseded_at: None,
                }))
        }

        async fn list_active(
            &self,
            _: &str,
            _: &ListFilter,
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(Vec::new())
        }

        async fn mark_superseded(
            &self,
            _: &str,
            _: &str,
        ) -> std::result::Result<u64, DashboardStoreError> {
            Ok(0)
        }

        async fn history(
            &self,
            _: &str,
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dashboard_read_refuses_system_frame() {
        let store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        let backend = RubixDashboardBackend::new(store, None, None, None, None);
        let err = backend.read("p1").expect_err("system frame must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dashboard_write_then_read_round_trips() {
        let store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        let backend = RubixDashboardBackend::new(
            store.clone(),
            Some("t-1".into()),
            Some("u-1".into()),
            None,
            None,
        );
        backend
            .write("p1", serde_json::json!({"hi": 1}))
            .expect("write");
        let body = backend.read("p1").expect("read after write");
        assert_eq!(body, serde_json::json!({"hi": 1}));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dashboard_read_unknown_page_is_not_found() {
        let store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        let backend =
            RubixDashboardBackend::new(store, Some("t-1".into()), Some("u-1".into()), None, None);
        let err = backend.read("missing").expect_err("not found");
        assert!(matches!(err, Error::ExtensionInternal(_)), "got {err:?}");
    }

    // Authz engine that always allows or always denies, so the
    // backend's plumbing (caller-binding, ResourceRef shape) is
    // what we exercise here — not the policy logic.
    struct FixedEngine(bool);

    #[async_trait]
    impl PolicyEngine for FixedEngine {
        async fn check(
            &self,
            _principal: &Principal,
            _action: &str,
            _object: &ResourceRef,
        ) -> Decision {
            if self.0 {
                Decision::allow()
            } else {
                Decision::deny("test_denied")
            }
        }
    }

    #[test]
    fn authz_refuses_system_frame() {
        let engine: Arc<dyn PolicyEngine> = Arc::new(FixedEngine(true));
        let backend = RubixAuthzBackend::new(engine, None, None);
        let err = backend
            .check("view", "rubix.dashboard.page:p1")
            .expect_err("system frame must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn authz_returns_engine_decision() {
        let allow: Arc<dyn PolicyEngine> = Arc::new(FixedEngine(true));
        let deny: Arc<dyn PolicyEngine> = Arc::new(FixedEngine(false));
        let caller = CallerIdentity {
            tenant_id: Some("t-1".into()),
            user_id: Some("u-1".into()),
            roles: vec!["Reader".into()],
            request_id: String::new(),
        };
        let allow_b = RubixAuthzBackend::new(allow, Some(&caller), None);
        let deny_b = RubixAuthzBackend::new(deny, Some(&caller), None);
        assert!(allow_b.check("view", "rubix.dashboard.page:p1").unwrap());
        assert!(!deny_b.check("view", "rubix.dashboard.page:p1").unwrap());
    }

    /// Asserting engine that captures the `Principal` it was
    /// handed so the test can verify the backend forwarded the
    /// full identity (scopes / teams / extra) instead of the
    /// lossy `CallerIdentity` reconstruction.
    struct CapturingEngine {
        captured: std::sync::Mutex<Option<Principal>>,
    }

    #[async_trait]
    impl PolicyEngine for CapturingEngine {
        async fn check(
            &self,
            principal: &Principal,
            _action: &str,
            _object: &ResourceRef,
        ) -> Decision {
            *self.captured.lock().unwrap() = Some(principal.clone());
            Decision::allow()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn with_principal_preserves_scopes_and_teams() {
        let capturing = Arc::new(CapturingEngine {
            captured: std::sync::Mutex::new(None),
        });
        let engine: Arc<dyn PolicyEngine> = capturing.clone();
        // Build a fully-populated principal: scopes + teams + extra
        // must survive the round-trip through `with_principal`.
        let p = Principal {
            subject: "u-1".into(),
            role: Role::Writer,
            scopes: vec![starter_spi::auth::Scope::new("dashboard:write")],
            tenant_id: Some("t-1".into()),
            teams: vec!["hvac-ops".into()],
            extra: serde_json::json!({"k": "v"}),
        };
        let backend = RubixAuthzBackend::with_principal(engine, p.clone(), None);
        backend
            .check("view", "rubix.dashboard.page:p1")
            .expect("allow");
        let seen = capturing
            .captured
            .lock()
            .unwrap()
            .clone()
            .expect("engine saw a principal");
        assert_eq!(seen.subject, "u-1");
        assert_eq!(seen.role, Role::Writer);
        assert_eq!(seen.scopes.len(), 1);
        assert_eq!(seen.teams, vec!["hvac-ops".to_string()]);
        assert_eq!(seen.extra, serde_json::json!({"k": "v"}));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dashboard_read_gate_refuses_page_outside_grant() {
        let store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        // Allowlist that names a different page.
        let grant: BTreeSet<String> = ["other-page".to_string()].into_iter().collect();
        let backend = RubixDashboardBackend::new(
            store,
            Some("t-1".into()),
            Some("u-1".into()),
            Some(grant),
            None,
        );
        let err = backend
            .read("p1")
            .expect_err("out-of-grant page must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dashboard_read_gate_allows_page_in_grant() {
        let store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        let grant: BTreeSet<String> = ["p1".to_string()].into_iter().collect();
        let backend = RubixDashboardBackend::new(
            store.clone(),
            Some("t-1".into()),
            Some("u-1".into()),
            Some(grant.clone()),
            Some(grant),
        );
        backend
            .write("p1", serde_json::json!({"hi": 1}))
            .expect("write in-grant");
        let body = backend.read("p1").expect("read in-grant");
        assert_eq!(body, serde_json::json!({"hi": 1}));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dashboard_write_gate_independent_from_read() {
        // Operator grants read-only — write must refuse even
        // though the page id is in the read grant.
        let store: Arc<dyn DashboardStore> = Arc::new(MemStore::default());
        let read_grant: BTreeSet<String> = ["p1".to_string()].into_iter().collect();
        let backend = RubixDashboardBackend::new(
            store,
            Some("t-1".into()),
            Some("u-1".into()),
            Some(read_grant),
            // No write grant — neutralised.
            Some(BTreeSet::new()),
        );
        let err = backend
            .write("p1", serde_json::json!({}))
            .expect_err("read-only grant must refuse write");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn authz_kind_gate_refuses_kind_outside_grant() {
        let engine: Arc<dyn PolicyEngine> = Arc::new(FixedEngine(true));
        let caller = CallerIdentity {
            tenant_id: Some("t-1".into()),
            user_id: Some("u-1".into()),
            roles: vec!["Reader".into()],
            request_id: String::new(),
        };
        let grant: BTreeSet<String> = ["rubix.tool".to_string()].into_iter().collect();
        let backend = RubixAuthzBackend::new(engine, Some(&caller), Some(grant));
        // Engine would allow, but the manifest gate refuses before
        // we get there.
        let err = backend
            .check("view", "rubix.dashboard.page:p1")
            .expect_err("out-of-grant kind must refuse");
        assert!(matches!(err, Error::Capability(_)), "got {err:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn authz_kind_gate_allows_kind_in_grant() {
        let engine: Arc<dyn PolicyEngine> = Arc::new(FixedEngine(true));
        let caller = CallerIdentity {
            tenant_id: Some("t-1".into()),
            user_id: Some("u-1".into()),
            roles: vec!["Reader".into()],
            request_id: String::new(),
        };
        let grant: BTreeSet<String> = ["rubix.dashboard.page".to_string()].into_iter().collect();
        let backend = RubixAuthzBackend::new(engine, Some(&caller), Some(grant));
        let allow = backend
            .check("view", "rubix.dashboard.page:p1")
            .expect("in-grant kind must reach the engine");
        assert!(allow);
    }

    #[test]
    fn principal_from_caller_parses_role_label() {
        let admin = CallerIdentity {
            tenant_id: Some("t-1".into()),
            user_id: Some("u-1".into()),
            roles: vec!["Admin".into()],
            request_id: String::new(),
        };
        let p = principal_from_caller(&admin).expect("not a system frame");
        assert_eq!(p.role, Role::Admin);
        assert_eq!(p.tenant_id.as_deref(), Some("t-1"));
        assert_eq!(p.subject, "u-1");

        // Unknown role label falls back to Reader (most restrictive).
        let weird = CallerIdentity {
            tenant_id: Some("t-1".into()),
            user_id: Some("u-1".into()),
            roles: vec!["Weirdo".into()],
            request_id: String::new(),
        };
        let p = principal_from_caller(&weird).expect("not a system frame");
        assert_eq!(p.role, Role::Reader);
    }
}
