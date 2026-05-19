//! Fine-grained scopes attached to API tokens. Roles are coarse;
//! scopes let a consumer issue a token that does exactly one thing
//! (e.g. `read:metrics`).

mod kind;

pub use kind::Scope;
