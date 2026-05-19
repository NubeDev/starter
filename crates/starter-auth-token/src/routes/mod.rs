//! `/auth/claim` axum route.

mod claim;
mod router;

pub use claim::{ClaimRequest, ClaimState};
pub use router::claim_router;
