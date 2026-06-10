//! Query result caching (WS-09 P1).
//!
//! Repeated identical panel queries within a refresh tick are served from an
//! in-process TTL cache instead of re-hitting the source database, and a
//! thundering herd of simultaneous misses for one key coalesces onto a single
//! backing load. The cache key is the full C3 tuple (tenant, datasource, query,
//! resolved time, variables, units/locale/timezone) so a result is reused only
//! when every input that can change the rows is identical.
//!
//! See docs/design/ for the production-hardening overview.

mod config;
mod key;
mod run;
mod store;

pub use config::CacheConfig;
pub use key::{build as cache_key, CacheKey, UNITS_PLACEHOLDER};
pub use run::run_cached;
pub use store::{CacheStats, QueryCache};
