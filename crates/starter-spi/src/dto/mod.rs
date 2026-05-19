//! DTOs that are part of the wire surface and therefore shared
//! across crates. Each DTO carries `utoipa::ToSchema` so the
//! OpenAPI document — and the codegen'd TS client — pick them up
//! automatically (R7).
//!
//! Domain-specific DTOs (consumer's `User`, `Project`, …) do **not**
//! live here. Only types that starter itself ships across crate
//! boundaries.

mod health;
mod problem;

pub use health::Health;
pub use problem::Problem;
