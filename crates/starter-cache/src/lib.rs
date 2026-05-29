//! # starter-cache
//!
//! Reusable async cache for the starter workspace. Primary use case
//! is helping `starter-server` handlers cut page-load latency by
//! memoising expensive renders / DB reads / upstream API calls, but
//! the [`Cache`] trait is generic enough for any consumer.
//!
//! ## Layout
//!
//! - [`Cache`] — the trait every backend implements.
//! - [`CacheError`] — error surface for fallible loads.
//! - [`CacheStats`] — lightweight hit/miss counters.
//! - [`backends`] — concrete implementations, each behind its own
//!   cargo feature. `moka` is on by default.
//!
//! ## Adding a backend later
//!
//! When we outgrow in-process caching (multiple server instances
//! sharing state), add a `valkey` feature + a `backends::valkey`
//! module. Consumers swap by changing the type at one wiring site —
//! call sites stay on the [`Cache`] trait.
//!
//! ## Example
//!
//! ```no_run
//! use starter_cache::{Cache, backends::moka::MokaCache};
//! use std::time::Duration;
//!
//! # async fn demo() {
//! let cache: MokaCache<String, String> = MokaCache::builder()
//!     .max_capacity(10_000)
//!     .time_to_live(Duration::from_secs(60))
//!     .build();
//!
//! // Stampede-safe page render: only ONE task runs the loader per
//! // missing key; concurrent callers wait on its result.
//! let page = cache
//!     .get_or_insert_with("home".into(), || async {
//!         Ok::<_, std::convert::Infallible>(render_home().await)
//!     })
//!     .await
//!     .unwrap();
//! # let _ = page;
//! # }
//! # async fn render_home() -> String { String::new() }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod backends;
mod cache;
mod error;
mod stats;

pub mod clock;
pub mod invalidator;
pub mod layer;
pub mod per_spec_stats;
pub mod spec;
pub mod tracing_cache;

pub use cache::Cache;
pub use error::CacheError;
pub use stats::CacheStats;

pub use clock::{Clock, MockClock, SystemClock};
pub use invalidator::{InMemoryInvalidator, Invalidator, TokenSnapshot};
pub use layer::{Bytes, CacheLayer, CallerScope, LayerConfig, LoadOutcome};
pub use per_spec_stats::{
    LoadLatency, LoadLatencySnapshot, PerSpecSnapshot, PerSpecStats, SpecCounters,
};
pub use spec::{BucketTagSpec, CacheScope, CacheSidecar, CacheSpec, InvalidateOn, SpecParseError};
pub use tracing_cache::{CacheEvent, TracingCache};
