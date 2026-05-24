//! `gate_tools(router, authenticator, engine)` — the auth + authz
//! sandwich every authenticated tool route runs inside.
//!
//! Wraps the tools router in three layers, innermost first:
//!
//!   1. [`Extension(Arc<dyn PolicyEngine>)`] so the
//!      [`starter_authz::with_permission`] middleware can read the
//!      engine off request extensions.
//!   2. [`starter_authz::with_permission_owned`] (kind:
//!      `"rubix.tool"`, action: `"invoke"`) — authenticated
//!      callers that the engine denies see `403`.
//!   3. [`starter_server::auth::with_principal`] — unauthenticated
//!      callers see `401` before the permission gate ever runs.
//!
//! See [docs/design/auth/](../../../docs/design/auth/README.md) and
//! [docs/design/tools/](../../../docs/design/tools/README.md). The
//! per-verb [`REQUIRED_PERMISSION`](rubix_spi::dto::system::disk::REQUIRED_PERMISSION)
//! constants on each `rubix-spi::dto::system::*` module are the
//! canonical mapping the engine's rules will resolve against; v0
//! gates at the collection level (`rubix.tool:invoke`) and lifts
//! to per-verb rules when the policy data lands.

use std::sync::Arc;

use axum::{Extension, Router};

use starter_authz::with_permission_owned;
use starter_server::auth::with_principal;
use starter_spi::auth::Authenticator;
use starter_spi::authz::PolicyEngine;

/// Resource kind the v0 collection-level gate uses.
pub const TOOL_RESOURCE_KIND: &str = "rubix.tool";

/// Action the v0 collection-level gate uses.
pub const TOOL_INVOKE_ACTION: &str = "invoke";

/// Wrap the tools router in the auth + authz sandwich described in
/// the module docs.
pub fn gate_tools(
    router: Router,
    authenticator: Arc<dyn Authenticator>,
    engine: Arc<dyn PolicyEngine>,
) -> Router {
    let permissioned = with_permission_owned(
        router,
        TOOL_RESOURCE_KIND.to_owned(),
        TOOL_INVOKE_ACTION.to_owned(),
    )
    .layer(Extension(engine));

    with_principal(permissioned, authenticator)
}
