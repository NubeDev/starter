//! `/auth/*` routes. One file per endpoint so the handlers stay
//! easy to find by name.

pub(crate) mod login;
pub(crate) mod logout;
pub(crate) mod me;
mod router;
pub(crate) mod signup;
mod state;
pub mod tenants;
pub(crate) mod token;

pub use login::{LoginRequest, LoginResponse, PasswordNotSetResponse, CSRF_COOKIE};
pub use me::MeResponse;
pub use router::auth_router;
pub use signup::{SignupError, SignupRequest, SignupResponse};
pub use state::AuthState;
pub use tenants::{
    tenants_router, AddMemberBody, CreateTenantBody, MembershipView, PatchMemberBody,
    PatchTenantBody, TenantView,
};
pub use token::{
    MissingTenantIdResponse, TenantMembershipEntry, TenantRequiredResponse, TokenRequest,
    TokenResponse, TOKEN_DEFAULT_TTL_DAYS,
};
