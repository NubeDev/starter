//! Auth-guard helpers — `read` requires *any* authenticated
//! principal, `admin` requires `Role::Admin`. The handlers call
//! these inline rather than relying on tower layers so the same
//! crate compiles cleanly without depending on `starter-server`.
//!
//! Returns `Some(error_response)` when the guard fails and `None`
//! when the caller may proceed; this shape sidesteps the
//! `result_large_err` clippy lint (the `axum::Response` body is
//! large enough that returning it in a `Result::Err` triggers it).

use axum::http::Request;
use axum::response::Response;
use starter_spi::auth::{Principal, Role};

use super::errors::{forbidden, unauthorized};

pub(super) fn require_admin<B>(req: &Request<B>) -> Option<Response> {
    let Some(p) = req.extensions().get::<Principal>() else {
        return Some(unauthorized());
    };
    if matches!(p.role, Role::Admin) {
        None
    } else {
        Some(forbidden())
    }
}

pub(super) fn require_authenticated<B>(req: &Request<B>) -> Option<Response> {
    if req.extensions().get::<Principal>().is_some() {
        None
    } else {
        Some(unauthorized())
    }
}
