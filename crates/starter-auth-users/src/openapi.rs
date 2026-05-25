//! Canonical OpenAPI document for the routes this crate ships.
//!
//! Consumers either serve this directly (via `ServerBuilder::with_openapi`)
//! or merge it into their own utoipa-derived document. The TS codegen
//! pipeline (`pnpm codegen`) reads a checked-in snapshot of this doc.

use utoipa::OpenApi;

use crate::routes::{
    LoginRequest, LoginResponse, MeResponse, MissingTenantIdResponse, PasswordNotSetResponse,
    SignupError, SignupRequest, SignupResponse, TenantMembershipEntry, TenantRequiredResponse,
    TokenRequest, TokenResponse,
};
use crate::signup::mode::SignupMode;
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
        crate::routes::token::handler,
    ),
    components(schemas(
        LoginRequest, LoginResponse, PasswordNotSetResponse, MeResponse,
        TokenRequest, TokenResponse, TenantRequiredResponse, TenantMembershipEntry,
        MissingTenantIdResponse,
        Role, Problem,
    )),
    tags((name = "auth", description = "Authentication endpoints"))
)]
/// utoipa entry point holding the path + component derives for this crate.
pub struct AuthUsersApi;

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
        crate::routes::token::handler,
        crate::routes::signup::handler,
    ),
    components(schemas(
        LoginRequest, LoginResponse, PasswordNotSetResponse, MeResponse,
        TokenRequest, TokenResponse, TenantRequiredResponse, TenantMembershipEntry,
        MissingTenantIdResponse,
        SignupRequest, SignupResponse, SignupError,
        Role, Problem,
    )),
    tags((name = "auth", description = "Authentication endpoints"))
)]
/// utoipa entry point including signup routes.
pub struct AuthUsersApiWithSignup;

/// Build the canonical OpenAPI document for this crate's routes.
/// When signup is enabled, includes the signup endpoint.
pub fn openapi(signup_mode: &SignupMode) -> utoipa::openapi::OpenApi {
    match signup_mode {
        SignupMode::Disabled => AuthUsersApi::openapi(),
        SignupMode::Open { .. } => AuthUsersApiWithSignup::openapi(),
    }
}
