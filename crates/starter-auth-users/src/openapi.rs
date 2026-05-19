//! Canonical OpenAPI document for the routes this crate ships.
//!
//! Consumers either serve this directly (via `ServerBuilder::with_openapi`)
//! or merge it into their own utoipa-derived document. The TS codegen
//! pipeline (`pnpm codegen`) reads a checked-in snapshot of this doc.

use utoipa::OpenApi;

use crate::routes::{LoginRequest, LoginResponse, MeResponse, PasswordNotSetResponse};
use starter_spi::auth::Role;
use starter_spi::dto::Problem;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "starter-auth-users",
        description = "Cookie-session + API-token authentication routes shipped by starter-auth-users.",
        version = env!("CARGO_PKG_VERSION"),
    ),
    paths(
        crate::routes::login::handler,
        crate::routes::logout::handler,
        crate::routes::me::handler,
    ),
    components(schemas(LoginRequest, LoginResponse, PasswordNotSetResponse, MeResponse, Role, Problem)),
    tags((name = "auth", description = "Authentication endpoints"))
)]
/// utoipa entry point holding the path + component derives for this crate.
pub struct AuthUsersApi;

/// Build the canonical OpenAPI document for this crate's routes.
pub fn openapi() -> utoipa::openapi::OpenApi {
    AuthUsersApi::openapi()
}
