//! `/auth/claim` axum route.

mod claim;
mod router;

pub use claim::ClaimRequest;
pub use router::claim_router;
