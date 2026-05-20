//! Data-transfer objects that `starter-prefs` ships for the platform
//! wire surface. Each submodule owns one DTO family.
//!
//! Per **D-2.1** in `DOCS/user/scope/SCOPE.md`, the per-series
//! response shape (R8) lands as a `ToSchema` DTO in
//! [`series`] so the openapi.json captures the exact shape the SCOPE
//! example describes.

pub mod series;

pub use series::{
    FromCanonicalSeries, SeriesEnvelope, SeriesPoint, ToCanonicalSeries,
};
