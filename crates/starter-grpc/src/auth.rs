//! Authentication seam for the gRPC service.
//!
//! tonic interceptors are sync, but [`starter_spi::auth::Authenticator::verify`]
//! is async. The cleanest fit is therefore to perform the bearer
//! check inside each handler rather than in a `tower::Layer` (which
//! would need to spawn a separate task per RPC just to bridge the
//! sync→async gap). Pattern mirrors what `examples/notes/src/grpc.rs`
//! does today.
//!
//! [`GrpcAuth`] is the small enum the server holds: `Open` for
//! single-user / network-isolated deployments, `Bearer(...)` to
//! require `Authorization: Bearer <token>` on every RPC and route
//! the token through the consumer-supplied `Authenticator` impl.

use std::sync::Arc;

use starter_spi::auth::{Authenticator, Principal};
use tonic::{Request, Status};

/// Authentication policy applied to every RPC the gRPC service
/// receives.
#[derive(Clone)]
pub enum GrpcAuth {
    /// No credential required. Suitable for sidecars bound to
    /// `127.0.0.1`, in-process tests, or deployments fronted by
    /// another auth layer. Matches `starter-mcp`'s default for
    /// stdio + open-HTTP.
    Open,

    /// Require `Authorization: Bearer <token>`; the supplied
    /// `Authenticator` resolves the token to a [`Principal`].
    /// Failures surface as `Status::unauthenticated`.
    Bearer(Arc<dyn Authenticator>),
}

impl GrpcAuth {
    /// Run the configured check against an inbound request. On
    /// success the resolved `Principal` (if any) is attached to the
    /// request's extensions so handlers can read it via
    /// `request.extensions().get::<Principal>()`. On failure a
    /// `tonic::Status` ready to return from the handler.
    pub async fn check<T>(&self, req: &mut Request<T>) -> Result<(), Status> {
        let authenticator = match self {
            GrpcAuth::Open => return Ok(()),
            GrpcAuth::Bearer(a) => a,
        };

        let raw = req
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("missing authorization metadata"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("non-ASCII authorization metadata"))?;
        let token = raw
            .strip_prefix("Bearer ")
            .or_else(|| raw.strip_prefix("bearer "))
            .ok_or_else(|| Status::unauthenticated("authorization must be `Bearer <token>`"))?
            .trim();

        match authenticator.verify(token).await {
            Ok(principal) => {
                req.extensions_mut().insert::<Principal>(principal);
                Ok(())
            }
            Err(_) => Err(Status::unauthenticated("invalid bearer token")),
        }
    }
}

impl std::fmt::Debug for GrpcAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrpcAuth::Open => f.write_str("GrpcAuth::Open"),
            GrpcAuth::Bearer(_) => f.write_str("GrpcAuth::Bearer(<authenticator>)"),
        }
    }
}
