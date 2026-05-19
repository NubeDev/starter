//! Merge starter-owned paths into the consumer's OpenAPI document.
//! Run once at startup; the merged doc is what `/openapi.json`
//! serves and what `pnpm codegen` reads.

use utoipa::openapi::OpenApi;

/// Extend `doc` in place with starter's own paths and component
/// schemas.
///
/// Idempotent: calling twice is a no-op. Paths starter owns are
/// `/health`, `/metrics`, `/openapi.json`; schemas it owns are the
/// DTOs in `starter_spi::dto`.
pub fn merge_starter_paths(_doc: &mut OpenApi) {
    // TODO(ap): build a `utoipa::OpenApi` derive over starter's own
    // handlers and merge here. Stubbed so the seam is locked.
}
