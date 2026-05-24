//! `router(admin)` and `router_with_auth(admin, authenticator)` — build
//! the admin Axum router.
//!
//! The router mounts:
//!
//! | Method | Path                              | Guard          |
//! |--------|-----------------------------------|----------------|
//! | GET    | `/extensions`                     | `Role::Admin`  |
//! | GET    | `/extensions/:id`                 | `Role::Admin`  |
//! | GET    | `/extensions/:id/events`          | `Role::Admin`  |
//! | POST   | `/extensions/:id/enable`          | `Role::Admin`  |
//! | POST   | `/extensions/:id/disable`         | `Role::Admin`  |
//! | GET    | `/extensions/:id/ui/*path`        | (unauthed)     |
//! | GET    | `/extensions/:id/i18n/:lang`      | (unauthed)     |
//!
//! The UI bundle path is deliberately unauthed: Module-Federation hosts
//! load `remoteEntry.js` with no credentials, and the bundle bytes are
//! the same ones the operator already approved by enabling the
//! extension. The list / detail / events / toggle endpoints carry
//! operator-level information (manifest, capability grants, stderr
//! lines) and stay gated.
//!
//! [`router_with_auth`] wraps the gated sub-router with the parent
//! workspace's `with_principal` → `with_role(Role::Admin)`. [`router`]
//! omits both, suitable for `TestApp` only — production binaries must
//! always go through [`router_with_auth`].

use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use starter_server::auth::{with_principal, with_role};
use starter_spi::auth::{Authenticator, Role};

use crate::admin::ExtensionAdmin;
use crate::events::events;
use crate::i18n::i18n;
use crate::lifecycle::{install, uninstall};
use crate::routes::{detail, disable, enable, list};
use crate::ui::ui;

/// Build options. Kept as a struct so future knobs (custom path prefix,
/// alternate role, CORS) can land without breaking existing callers.
#[derive(Default)]
pub struct AdminRouterOptions;

/// Build the admin router *without* authentication. Suitable for
/// `TestApp` and smoke tests — production callers must use
/// [`router_with_auth`].
pub fn router<S>(admin: ExtensionAdmin) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let gated: Router<S> = gated_routes(admin.clone());
    let public: Router<S> = public_routes(admin);
    Router::new().merge(gated).merge(public)
}

/// Build the admin router with `with_principal` + `with_role(Admin)`.
/// The UI bundle path remains unauthenticated (see module docs).
pub fn router_with_auth<S, A>(admin: ExtensionAdmin, authenticator: Arc<A>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    A: Authenticator + ?Sized,
{
    let gated: Router<S> = gated_routes(admin.clone());
    let gated = with_principal(with_role(gated, Role::Admin), authenticator);
    let public: Router<S> = public_routes(admin);
    Router::new().merge(gated).merge(public)
}

fn gated_routes<S>(admin: ExtensionAdmin) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/extensions", get(list))
        .route("/extensions/{id}", get(detail))
        .route("/extensions/{id}/events", get(events))
        .route("/extensions/{id}/enable", post(enable))
        .route("/extensions/{id}/disable", post(disable))
        .route("/extensions/install", post(install))
        .route("/extensions/{id}", delete(uninstall))
        .with_state(admin)
}

fn public_routes<S>(admin: ExtensionAdmin) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/extensions/{id}/ui/{*path}", get(ui))
        // Catalog endpoint — same unauthed posture as the UI bundle:
        // the bytes ship inside the admin-approved extension and carry
        // no operator data. See `examples/notes/user-pref.md` § Stage 5.
        .route("/extensions/{id}/i18n/{lang}", get(i18n))
        .with_state(admin)
}
