//! Mount the extension admin router under nexus's `/api/v1` prefix.
//!
//! The kernel's `router_with_auth` serves its routes at `/extensions/*` and
//! applies its **own** `with_principal` → `with_role(Role::Admin)` layer (the
//! same `starter-server` middleware nexus uses). So it must be merged as a
//! *sibling* of nexus's own principal-wrapped routers (like `authz`/`tenants`),
//! **not** nested inside nexus's `with_principal(product)` — otherwise the
//! principal layer would run twice. nexus's product routes live at absolute
//! `/api/v1/...` paths; the kernel's are relative to `/extensions`, so we nest
//! the whole admin router under `/api/v1` to land them at
//! `/api/v1/extensions/*` — the path the frontend `bootstrapExtensions` already
//! calls.

use std::sync::Arc;

use axum::Router;
use starter_ext_server::ExtensionAdmin;
use starter_spi::auth::Authenticator;

use crate::state::AppState;

/// Build the `/api/v1/extensions/*` admin router, authenticated by nexus's
/// `Authenticator` with mutations gated `Role::Admin` (applied inside the
/// kernel router). The public UI-bundle / i18n routes stay unauthenticated, as
/// the kernel intends (the bytes ship inside an admin-approved extension).
pub fn router<A>(admin: ExtensionAdmin, authenticator: Arc<A>) -> Router<AppState>
where
    A: Authenticator + ?Sized,
{
    let ext: Router<AppState> = starter_ext_server::router_with_auth(admin, authenticator);
    Router::new().nest("/api/v1", ext)
}
