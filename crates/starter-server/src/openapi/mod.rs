//! OpenAPI document assembly. The consumer builds their `OpenApi`
//! via `utoipa::OpenApi` derive; this module exposes helpers to
//! merge starter-owned paths (`/health`, `/metrics`) into it so the
//! single served document covers the whole surface.

mod merge;

pub use merge::merge_starter_paths;
