//! `starter-blob-axum` — authenticated GET proxy for any
//! [`BlobStore`].
//!
//! # Why this crate exists
//!
//! `starter-blob-fs` and `starter-blob-memory` already ship
//! feature-gated routers that serve *presigned* URLs. Presign is
//! the right primitive for direct-upload-from-browser, but the
//! wrong primitive for **inline content served by per-request auth**:
//! a markdown page rendered at arbitrary times cannot carry
//! presigned image URLs with a TTL without either rewriting the
//! body on every edit (lossy round-trip) or refreshing every link
//! on every render (extra latency, signing cost per render).
//!
//! Instead, every starter consumer building a content-rendering
//! product writes the same ~30-line `axum` handler:
//!
//! - Parse a [`BlobRef`] out of the URL.
//! - Decide whether the *current viewer* can see *this* `BlobRef`
//!   (a domain-level question only the consumer can answer).
//! - Map [`BlobError`] variants to HTTP status codes.
//! - Forward `Range`, `If-None-Match`, and `Accept-Encoding`.
//! - Read the reserved `filename` user-metadata key for
//!   `Content-Disposition`.
//!
//! That handler is library-shaped work. This crate ships it once,
//! parametrised on a consumer-supplied authz closure.
//!
//! # Why authz takes a [`BlobContext`] alongside the [`BlobRef`]
//!
//! The naive closure shape `Fn(&BlobRef, &Request) -> Result<…>`
//! forces the consumer to re-parse the namespace prefix
//! (e.g. `"project-7"`) out of an opaque `BlobRef` so they can ask
//! "does this viewer have access to project 7?". That leaks the
//! namespace scheme into authorization code and undermines B1/B2.
//!
//! The closure here receives a [`BlobContext`] populated by the
//! combinator stack — a structured value carrying the parsed
//! namespace prefixes. The consumer authorizes against a string
//! they already understand, not by fishing inside the opaque ref.
//!
//! # Non-goals
//!
//! - **No upload route.** Presign + direct PUT is the contract for
//!   uploads; this crate is for GETs.
//! - **No range coalescing or caching.** Each request hits the
//!   underlying store. Combine with `ReadThroughCache` if a cache
//!   is wanted.
//! - **No body transcoding.** Bytes flow through unmodified.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod handler;
mod mapping;

pub use handler::{blob_proxy_handler, AuthzFn};
pub use mapping::blob_error_to_status;
