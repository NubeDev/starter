//! Adapter letting the `starter-server` `Accept-Units` middleware resolve a
//! caller's preferences once per request.
//!
//! LAYER: transport (middleware adapter). It reads the `Principal` the auth
//! layer injected, maps it onto the nexus tenancy keys (`user_id =
//! principal.subject`, `workspace_id = principal.tenant_id`), and delegates the
//! actual three-layer merge to `starter-prefs`' pure resolver. No conversion
//! math or business predicates live here — that is the middleware's `UnitsCtx`.

use std::sync::Arc;

use async_trait::async_trait;
use axum::http::request::Parts;
use axum::response::Response;
use starter_prefs::resolver::{resolve, SystemDefaults};
use starter_prefs::store::PgPrefsStore;
use starter_server::middleware::PrefsResolverFor;
use starter_spi::auth::Principal;
use starter_spi::preferences::ResolvedPreferences;

use super::factory::NexusPrefs;

/// Resolves preferences for the in-flight request by reading its `Principal`.
///
/// An authenticated, tenant-bound caller resolves user → org(tenant) → default;
/// any caller without a principal or tenant binding (the dev single-datasource
/// shortcut, health probes) falls back to the system defaults rather than
/// failing the request, so the units layer never blocks an otherwise-valid
/// unauthenticated path.
pub struct NexusPrefsResolver {
    store: Arc<PgPrefsStore>,
    defaults: Arc<SystemDefaults>,
}

impl NexusPrefsResolver {
    /// Build the resolver from the shared preference handles.
    pub fn new(prefs: &NexusPrefs) -> Self {
        Self {
            store: prefs.store.clone(),
            defaults: prefs.defaults.clone(),
        }
    }
}

#[async_trait]
impl PrefsResolverFor for NexusPrefsResolver {
    async fn resolve_for(&self, parts: &Parts) -> Result<ResolvedPreferences, Response> {
        // No principal or no tenant binding → system defaults. A units context
        // is always available; conversion simply uses the platform defaults.
        let Some((user_id, workspace_id)) = principal_keys(parts) else {
            return Ok(resolve(None, None, &self.defaults));
        };

        // A store error must not 500 the whole request — units are a
        // presentation concern. Fall back to defaults and let the request
        // proceed; the error surfaces in logs via the store.
        let user = self
            .store
            .get_user_prefs(&user_id, &workspace_id)
            .await
            .unwrap_or(None);
        let org = self
            .store
            .get_org_prefs(&workspace_id)
            .await
            .unwrap_or(None);
        Ok(resolve(user, org, &self.defaults))
    }
}

/// Pull `(user_id, workspace_id)` from the request's `Principal`, if present and
/// tenant-bound. `user_id` is the principal subject; `workspace_id` is its
/// tenant — the same pinning the `/me/preferences` routes use, so a caller's
/// resolved units always match the rows it can read and write.
fn principal_keys(parts: &Parts) -> Option<(String, String)> {
    let p = parts.extensions.get::<Principal>()?;
    let tenant = p.tenant_id.as_deref().filter(|t| !t.is_empty())?;
    Some((p.subject.clone(), tenant.to_string()))
}

// The store trait is in scope for the calls above.
use starter_prefs::store::PrefsStore as _;
