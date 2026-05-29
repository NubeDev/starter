//! `CacheSpec` — the declarative shape an opt-in caller hands to the
//! cache layer. Stage 1 (v1) of the [opt-in caching proposal][1] adds:
//!
//! - `stale_while_revalidate` + `max_stale` — serve cached values
//!   inside an SWR window while a follow-up refresh happens.
//! - `empty_ttl` + `cache_empty` — empty results are cached separately
//!   (shorter TTL) so a cold-spot query doesn't drum the warehouse.
//! - `InvalidateOn::events` — write-path event tags like
//!   `event:ingest.batch.committed`.
//! - `InvalidateOn::buckets` — declare a bucket granularity per table
//!   so writers can fire `bucket:<table>:<floor(t, granularity)>`.
//!
//! [1]: ../../../rubix/docs/proposal/fe-cache-opt-in.md

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Scope at which a cache entry is shared between callers.
///
/// Picked by the author. The cache layer mixes the matching identity
/// bits into the key automatically — `user` adds `(tenant, user)`,
/// `tenant` adds `(tenant,)`, `global` adds nothing.
///
/// The default is `tenant`. `global` requires an explicit author
/// choice; the proposal makes this point hard so a noisy or
/// tenant-coupled query cannot accidentally leak between tenants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheScope {
    /// Shared across the whole platform. Use only for reference
    /// data with no tenant coupling.
    Global,
    /// Shared between users in the same tenant. The safe default
    /// for tenant-coupled reads.
    Tenant,
    /// Per-user. Use whenever the rendered answer depends on user
    /// identity, AuthZ, locale, or unit prefs.
    User,
}

impl Default for CacheScope {
    fn default() -> Self {
        CacheScope::Tenant
    }
}

/// Bucket-granularity invalidation declaration on a spec.
///
/// A writer that ingests row at time `t` fires the tag
/// `bucket:<table>:<floor(t, granularity)>`. Specs subscribed to the
/// matching `(table, granularity)` receive a coarse subscription tag
/// `bucket:<table>:<granularity>` that the invalidator treats as a
/// distinct hashable string. Fine-grained bucket-fan-out is wired in
/// v2 (the `TimescaleWindowedFetcher`); v1 lands the structural
/// declaration and the broad subscription tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BucketTagSpec {
    /// Table whose bucket events the spec subscribes to.
    pub table: String,
    /// Granularity of the bucket — accepted strings are
    /// `"1m" | "5m" | "15m" | "1h" | "1d"` (parsed but not enforced
    /// at the layer; writers must agree with spec authors).
    pub granularity: String,
    /// v3 — dimension columns the writer emits alongside the bucket
    /// tag. A row `(meter=42, t=…)` written with `dimensions:
    /// ["meter"]` causes the chokepoint to fire
    /// `table:<name>:meter=42` in addition to the per-row bucket
    /// tag. Specs subscribe by listing the literal dimensional tag
    /// in `invalidate_on.tables`. Empty (default) keeps prior
    /// behaviour.
    #[serde(default)]
    pub dimensions: Vec<String>,
}

/// What invalidation tags the cache should subscribe an entry to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidateOn {
    /// Tables whose `table:<name>` tag invalidates this spec.
    #[serde(default)]
    pub tables: Vec<String>,
    /// Write-path event tags (e.g. `"event:ingest.batch.committed"`).
    /// Each entry is fired verbatim by the write path; the spec gets
    /// the same string as a subscription tag. Authors write either
    /// bare names (`"ingest.batch.committed"`) or fully-qualified
    /// (`"event:ingest.batch.committed"`) — the layer normalises with
    /// the `event:` prefix.
    #[serde(default)]
    pub events: Vec<String>,
    /// Optional bucket-grain subscription. One per spec.
    #[serde(default)]
    pub buckets: Option<BucketTagSpec>,
}

/// The cache spec. Compose with builder methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheSpec {
    /// How long an entry lives before forced refresh.
    pub ttl: Duration,
    /// Who shares this entry.
    pub scope: CacheScope,
    /// Which tags drop this entry's bucket when fired.
    pub invalidate_on: InvalidateOn,
    /// Inside the last `stale_while_revalidate` of the TTL (or after
    /// expiry but within `max_stale`), the layer serves the cached
    /// value as a hit and marks the entry for refresh. Default is
    /// zero (SWR disabled — pre-v1 behaviour).
    pub stale_while_revalidate: Duration,
    /// Maximum age past TTL at which the layer will still serve
    /// stale. Default is `2 * ttl`. Past this, the next read is a
    /// hard miss.
    pub max_stale: Duration,
    /// TTL for empty results — when the loader returns
    /// [`LoadOutcome::Empty`][crate::layer::LoadOutcome::Empty], the
    /// layer stores the marker for at most this long. Default 5s,
    /// clamped to `min(empty_ttl, ttl)`.
    pub empty_ttl: Duration,
    /// When `false`, empty results are not cached at all (every miss
    /// re-runs the loader). Default `true`.
    pub cache_empty: bool,
    /// v2: opt-in time-series block — enables bucket decomposition,
    /// tail-vs-body TTLs, and bucket-level invalidation tag fan-out.
    /// When `Some`, the layer's
    /// [`get_or_load_windowed`][crate::layer::CacheLayer::get_or_load_windowed]
    /// entry point is the right call site; other entry points ignore
    /// the field.
    pub time_series: Option<TimeSeriesBlock>,
    /// v2: opt-in two-layer cache scope (§Layer 6c). When `Some` and
    /// `scope: user`, the layer first performs a lookup at
    /// `inner_scope` (typically `tenant`), runs the caller's
    /// conversion closure against the user's prefs, and stores the
    /// rendered output at `scope`.
    pub inner_scope: Option<CacheScope>,
}

/// v2: structured time-series block. Mirrors the `time_series:`
/// YAML block from the proposal and feeds straight into
/// [`starter_windowed::WindowedSpec`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSeriesBlock {
    /// Request param carrying the upper-bound timestamp ("now").
    pub time_param: String,
    /// Request param carrying the window start.
    pub range_param: String,
    /// Bucket granularity string (e.g. `"1h"`).
    pub bucket: String,
    /// Tail (open bucket) TTL string (e.g. `"30s"`).
    pub tail_ttl: String,
    /// Body (closed buckets) TTL string (e.g. `"24h"`).
    pub body_ttl: String,
    /// Bucket alignment. Today only `"utc"` is honoured.
    #[serde(default = "default_align_to")]
    pub align_to: String,
}

fn default_align_to() -> String {
    "utc".to_string()
}

impl CacheSpec {
    /// Start a spec with a TTL. Scope defaults to `tenant`, no tags,
    /// SWR/max_stale disabled, empty_ttl 5s clamped to ttl,
    /// cache_empty true.
    pub fn ttl(ttl: Duration) -> Self {
        let empty_ttl = std::cmp::min(Duration::from_secs(5), ttl);
        Self {
            ttl,
            scope: CacheScope::default(),
            invalidate_on: InvalidateOn::default(),
            stale_while_revalidate: Duration::ZERO,
            max_stale: ttl.saturating_mul(2),
            empty_ttl,
            cache_empty: true,
            time_series: None,
            inner_scope: None,
        }
    }

    /// Attach a time-series block (v2).
    pub fn time_series(mut self, ts: TimeSeriesBlock) -> Self {
        self.time_series = Some(ts);
        self
    }

    /// Set the inner-scope (v2 two-layer cache).
    pub fn inner_scope(mut self, scope: CacheScope) -> Self {
        self.inner_scope = Some(scope);
        self
    }

    /// Materialise the time-series block as a
    /// [`starter_windowed::WindowedSpec`]. `None` when the spec
    /// has no `time_series:` declared, or the block carries an
    /// unparseable duration.
    pub fn windowed_spec(&self) -> Option<starter_windowed::WindowedSpec> {
        let ts = self.time_series.as_ref()?;
        let bucket = parse_chrono_duration(&ts.bucket).ok()?;
        let tail_ttl = parse_duration(&ts.tail_ttl).ok()?;
        let body_ttl = parse_duration(&ts.body_ttl).ok()?;
        let align_to = match ts.align_to.as_str() {
            "utc" | "UTC" => starter_windowed::AlignTo::Utc,
            _ => starter_windowed::AlignTo::Utc,
        };
        Some(starter_windowed::WindowedSpec {
            time_param: ts.time_param.clone(),
            range_param: ts.range_param.clone(),
            bucket,
            tail_ttl,
            body_ttl,
            align_to,
        })
    }

    /// Set the scope.
    pub fn scope(mut self, scope: CacheScope) -> Self {
        self.scope = scope;
        self
    }

    /// Subscribe to one warehouse table tag (`table:<name>`).
    pub fn invalidate_on_table(mut self, table: impl Into<String>) -> Self {
        self.invalidate_on.tables.push(table.into());
        self
    }

    /// Subscribe to one event tag. Accepts a bare name or a
    /// `event:`-prefixed string.
    pub fn invalidate_on_event(mut self, event: impl Into<String>) -> Self {
        self.invalidate_on.events.push(event.into());
        self
    }

    /// Declare a stale-while-revalidate window.
    pub fn stale_while_revalidate(mut self, swr: Duration) -> Self {
        self.stale_while_revalidate = swr;
        self
    }

    /// Override the default `max_stale` (`2 * ttl`).
    pub fn max_stale(mut self, max: Duration) -> Self {
        self.max_stale = max;
        self
    }

    /// Override the default empty-result TTL. Clamped to ttl on
    /// derivation.
    pub fn empty_ttl(mut self, et: Duration) -> Self {
        self.empty_ttl = std::cmp::min(et, self.ttl);
        self
    }

    /// Toggle whether empty results are cached at all.
    pub fn cache_empty(mut self, on: bool) -> Self {
        self.cache_empty = on;
        self
    }

    /// Derive the tag set this spec is subscribed to. v1 includes
    /// `table:<name>` for each declared table, the normalised
    /// `event:<name>` for each declared event, and the coarse
    /// `bucket:<table>:<granularity>` subscription tag (if any).
    pub fn derived_tags(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .invalidate_on
            .tables
            .iter()
            .map(|t| format!("table:{t}"))
            .collect();
        for e in &self.invalidate_on.events {
            if let Some(rest) = e.strip_prefix("event:") {
                out.push(format!("event:{rest}"));
            } else {
                out.push(format!("event:{e}"));
            }
        }
        if let Some(b) = &self.invalidate_on.buckets {
            out.push(format!("bucket:{}:{}", b.table, b.granularity));
        }
        out
    }
}

// ---- YAML wire shape for `kind.cache.yaml` ---------------------------------

/// Wire shape of a `kind.cache.yaml` sidecar.
#[derive(Debug, Clone, Deserialize)]
pub struct CacheSidecar {
    /// The single `cache:` block at the top of the sidecar.
    pub cache: CacheSidecarBody,
}

/// Bucket-tag enumeration helper. When a spec declares a
/// `time_series:` block plus `invalidate_on.tables: [...]`, the
/// registry can pre-derive every `bucket:<table>:<floor(t,bucket)>`
/// tag at registration time so the bucket-tag invalidator (wired in
/// v1) recognises a write firing one bucket key as touching one
/// cached entry — not every entry for the table.
///
/// This helper returns the **subscription tag prefix**
/// `bucket:<table>:` that the registry uses to map fan-out fires to
/// spec entries; the per-bucket fire happens at write time and the
/// match is by string prefix at invalidate time.
pub fn bucket_subscription_prefix(table: &str) -> String {
    format!("bucket:{table}:")
}

/// The fields inside the top-level `cache:` block.
#[derive(Debug, Clone, Deserialize)]
pub struct CacheSidecarBody {
    /// Human-readable duration string (e.g. `60s`, `5m`). Parsed
    /// at load time.
    pub ttl: String,
    /// Optional, defaults to `tenant`.
    #[serde(default)]
    pub scope: CacheScope,
    /// Optional. Empty means "TTL-only, no tag invalidation".
    #[serde(default)]
    pub invalidate_on: InvalidateOn,
    /// Optional SWR window string (e.g. `30s`).
    #[serde(default)]
    pub stale_while_revalidate: Option<String>,
    /// Optional override of default `max_stale` (`2 * ttl`).
    #[serde(default)]
    pub max_stale: Option<String>,
    /// Optional override of default `empty_ttl` (5s).
    #[serde(default)]
    pub empty_ttl: Option<String>,
    /// Optional toggle for caching empty results.
    #[serde(default)]
    pub cache_empty: Option<bool>,
    /// v2: optional `time_series:` block.
    #[serde(default)]
    pub time_series: Option<TimeSeriesBlock>,
    /// v2: optional `inner_scope:` (two-layer cache).
    #[serde(default)]
    pub inner_scope: Option<CacheScope>,
}

/// Errors raised when parsing a sidecar's TTL / structure.
#[derive(Debug, thiserror::Error)]
pub enum SpecParseError {
    /// The TTL string was not in `<n><unit>` shape.
    #[error("invalid ttl {0:?}: expected '<number><s|m|h>' (e.g. '60s')")]
    BadTtl(String),
    /// YAML deserialisation failed at a known line. `line` is
    /// 1-indexed so it matches what the operator sees in their
    /// editor and in `tail -n +<line>` output.
    #[error("yaml: line {line}: {message}")]
    Yaml {
        /// 1-indexed line number in the source YAML.
        line: usize,
        /// Human-readable description of what went wrong.
        message: String,
    },
}

impl SpecParseError {
    fn yaml_file(message: impl Into<String>) -> Self {
        Self::Yaml {
            line: 0,
            message: message.into(),
        }
    }

    fn yaml_at(line: usize, message: impl Into<String>) -> Self {
        Self::Yaml {
            line,
            message: message.into(),
        }
    }
}

impl CacheSidecar {
    /// Parse a sidecar from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, SpecParseError> {
        parse_sidecar_yaml(yaml)
    }

    /// Materialise a [`CacheSpec`] from the parsed sidecar.
    pub fn into_spec(self) -> Result<CacheSpec, SpecParseError> {
        let ttl = parse_duration(&self.cache.ttl)?;
        let swr = match self.cache.stale_while_revalidate.as_deref() {
            Some(s) => parse_duration(s)?,
            None => Duration::ZERO,
        };
        let max_stale = match self.cache.max_stale.as_deref() {
            Some(s) => parse_duration(s)?,
            None => ttl.saturating_mul(2),
        };
        let empty_ttl_raw = match self.cache.empty_ttl.as_deref() {
            Some(s) => parse_duration(s)?,
            None => std::cmp::min(Duration::from_secs(5), ttl),
        };
        let empty_ttl = std::cmp::min(empty_ttl_raw, ttl);
        let cache_empty = self.cache.cache_empty.unwrap_or(true);
        Ok(CacheSpec {
            ttl,
            scope: self.cache.scope,
            invalidate_on: self.cache.invalidate_on,
            stale_while_revalidate: swr,
            max_stale,
            empty_ttl,
            cache_empty,
            time_series: self.cache.time_series,
            inner_scope: self.cache.inner_scope,
        })
    }
}

fn parse_chrono_duration(s: &str) -> Result<chrono::Duration, SpecParseError> {
    let d = parse_duration(s)?;
    Ok(chrono::Duration::seconds(d.as_secs() as i64))
}

fn parse_duration(s: &str) -> Result<Duration, SpecParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(SpecParseError::BadTtl(s.to_string()));
    }
    let (num, unit) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit())
            .ok_or_else(|| SpecParseError::BadTtl(s.to_string()))?,
    );
    let n: u64 = num
        .parse()
        .map_err(|_| SpecParseError::BadTtl(s.to_string()))?;
    let mul = match unit.trim() {
        "s" | "sec" | "secs" | "seconds" => 1,
        "m" | "min" | "mins" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hours" => 3_600,
        _ => return Err(SpecParseError::BadTtl(s.to_string())),
    };
    Ok(Duration::from_secs(n * mul))
}

/// Hand-rolled YAML parser for the v2 sidecar shape. Keeps
/// `serde_yaml` out of the workspace dependency surface. Replaces
/// the v1 closure-heavy parser with a path/indent-stack walker so
/// the v2 `time_series:` + `inner_scope:` blocks slot in cleanly.
fn parse_sidecar_yaml(yaml: &str) -> Result<CacheSidecar, SpecParseError> {
    parse_sidecar_yaml_v2(yaml)
}

#[allow(dead_code)]
#[allow(unused_assignments)]
fn parse_sidecar_yaml_legacy(yaml: &str) -> Result<CacheSidecar, SpecParseError> {
    let mut ttl: Option<String> = None;
    let mut scope: CacheScope = CacheScope::default();
    let mut tables: Vec<String> = Vec::new();
    let mut events: Vec<String> = Vec::new();
    let mut buckets: Option<BucketTagSpec> = None;
    let mut swr: Option<String> = None;
    let mut max_stale: Option<String> = None;
    let mut empty_ttl: Option<String> = None;
    let mut cache_empty: Option<bool> = None;

    #[derive(PartialEq)]
    enum Section {
        TopLevel,
        Cache,
        InvalidateOn,
        Tables,
        Events,
        Buckets,
    }
    let mut section = Section::TopLevel;
    let mut cache_seen = false;
    let mut bucket_table: Option<String> = None;
    let mut bucket_gran: Option<String> = None;

    for (idx, raw) in yaml.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw);
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();

        // Convenience: any time we return to Section::Cache and the
        // first cache-field-level line appears, dispatch it.
        let try_parse_cache_field = |trimmed: &str,
                                     ttl: &mut Option<String>,
                                     scope: &mut CacheScope,
                                     swr: &mut Option<String>,
                                     max_stale: &mut Option<String>,
                                     empty_ttl: &mut Option<String>,
                                     cache_empty: &mut Option<bool>,
                                     line_no: usize|
         -> Result<Option<Section>, SpecParseError> {
            if let Some(v) = trimmed.strip_prefix("ttl:") {
                *ttl = Some(v.trim().trim_matches('"').to_string());
                Ok(None)
            } else if let Some(v) = trimmed.strip_prefix("stale_while_revalidate:") {
                *swr = Some(v.trim().trim_matches('"').to_string());
                Ok(None)
            } else if let Some(v) = trimmed.strip_prefix("max_stale:") {
                *max_stale = Some(v.trim().trim_matches('"').to_string());
                Ok(None)
            } else if let Some(v) = trimmed.strip_prefix("empty_ttl:") {
                *empty_ttl = Some(v.trim().trim_matches('"').to_string());
                Ok(None)
            } else if let Some(v) = trimmed.strip_prefix("cache_empty:") {
                *cache_empty = Some(matches!(v.trim(), "true" | "yes" | "on"));
                Ok(None)
            } else if let Some(v) = trimmed.strip_prefix("scope:") {
                *scope = match v.trim() {
                    "user" => CacheScope::User,
                    "tenant" => CacheScope::Tenant,
                    "global" => CacheScope::Global,
                    other => {
                        return Err(SpecParseError::yaml_at(
                            line_no,
                            format!("invalid scope {other:?}: expected user|tenant|global"),
                        ))
                    }
                };
                Ok(None)
            } else if trimmed.starts_with("invalidate_on:") {
                Ok(Some(Section::InvalidateOn))
            } else {
                Err(SpecParseError::yaml_at(
                    line_no,
                    format!("unknown cache field: {trimmed:?}"),
                ))
            }
        };

        if section == Section::TopLevel {
            if trimmed.starts_with("cache:") && indent == 0 {
                cache_seen = true;
                section = Section::Cache;
                continue;
            }
            return Err(SpecParseError::yaml_at(
                line_no,
                format!("unexpected top-level line: {raw:?} (only `cache:` allowed)"),
            ));
        }

        match section {
            Section::Cache => {
                if indent == 0 {
                    return Err(SpecParseError::yaml_at(
                        line_no,
                        format!("unexpected top-level line inside cache block: {raw:?}"),
                    ));
                }
                if let Some(next) = try_parse_cache_field(
                    trimmed,
                    &mut ttl,
                    &mut scope,
                    &mut swr,
                    &mut max_stale,
                    &mut empty_ttl,
                    &mut cache_empty,
                    line_no,
                )? {
                    section = next;
                }
            }
            Section::InvalidateOn => {
                if indent <= 2 {
                    // back out
                    section = Section::Cache;
                    if let Some(next) = try_parse_cache_field(
                        trimmed,
                        &mut ttl,
                        &mut scope,
                        &mut swr,
                        &mut max_stale,
                        &mut empty_ttl,
                        &mut cache_empty,
                        line_no,
                    )? {
                        section = next;
                    }
                    continue;
                }
                if trimmed.starts_with("tables:") {
                    section = Section::Tables;
                } else if trimmed.starts_with("events:") {
                    section = Section::Events;
                } else if trimmed.starts_with("buckets:") {
                    bucket_table = None;
                    bucket_gran = None;
                    section = Section::Buckets;
                } else {
                    return Err(SpecParseError::yaml_at(
                        line_no,
                        format!(
                            "unknown invalidate_on field: {trimmed:?} \
                             (supported: tables, events, buckets)"
                        ),
                    ));
                }
            }
            Section::Tables => {
                if let Some(item) = trimmed.strip_prefix("- ") {
                    tables.push(item.trim().trim_matches('"').to_string());
                } else if indent <= 4 {
                    section = Section::InvalidateOn;
                    if trimmed.starts_with("events:") {
                        section = Section::Events;
                    } else if trimmed.starts_with("buckets:") {
                        bucket_table = None;
                        bucket_gran = None;
                        section = Section::Buckets;
                    } else if let Some(next) = try_parse_cache_field(
                        trimmed,
                        &mut ttl,
                        &mut scope,
                        &mut swr,
                        &mut max_stale,
                        &mut empty_ttl,
                        &mut cache_empty,
                        line_no,
                    )? {
                        section = next;
                    } else {
                        section = Section::Cache;
                    }
                } else {
                    return Err(SpecParseError::yaml_at(
                        line_no,
                        format!("unknown tables entry: {raw:?}"),
                    ));
                }
            }
            Section::Events => {
                if let Some(item) = trimmed.strip_prefix("- ") {
                    events.push(item.trim().trim_matches('"').to_string());
                } else if indent <= 4 {
                    section = Section::InvalidateOn;
                    if trimmed.starts_with("tables:") {
                        section = Section::Tables;
                    } else if trimmed.starts_with("buckets:") {
                        bucket_table = None;
                        bucket_gran = None;
                        section = Section::Buckets;
                    } else if let Some(next) = try_parse_cache_field(
                        trimmed,
                        &mut ttl,
                        &mut scope,
                        &mut swr,
                        &mut max_stale,
                        &mut empty_ttl,
                        &mut cache_empty,
                        line_no,
                    )? {
                        section = next;
                    } else {
                        section = Section::Cache;
                    }
                } else {
                    return Err(SpecParseError::yaml_at(
                        line_no,
                        format!("unknown events entry: {raw:?}"),
                    ));
                }
            }
            Section::Buckets => {
                if indent <= 4 {
                    // closing the buckets block; finalise.
                    if let (Some(t), Some(g)) = (bucket_table.take(), bucket_gran.take()) {
                        buckets = Some(BucketTagSpec {
                            table: t,
                            granularity: g,
                            dimensions: Vec::new(),
                        });
                    } else if bucket_table.is_some() || bucket_gran.is_some() {
                        return Err(SpecParseError::yaml_at(
                            line_no,
                            "buckets block needs both `table` and `granularity`".to_string(),
                        ));
                    }
                    section = Section::InvalidateOn;
                    if trimmed.starts_with("tables:") {
                        section = Section::Tables;
                    } else if trimmed.starts_with("events:") {
                        section = Section::Events;
                    } else if let Some(next) = try_parse_cache_field(
                        trimmed,
                        &mut ttl,
                        &mut scope,
                        &mut swr,
                        &mut max_stale,
                        &mut empty_ttl,
                        &mut cache_empty,
                        line_no,
                    )? {
                        section = next;
                    } else {
                        section = Section::Cache;
                    }
                } else if let Some(v) = trimmed.strip_prefix("table:") {
                    bucket_table = Some(v.trim().trim_matches('"').to_string());
                } else if let Some(v) = trimmed.strip_prefix("granularity:") {
                    bucket_gran = Some(v.trim().trim_matches('"').to_string());
                } else {
                    return Err(SpecParseError::yaml_at(
                        line_no,
                        format!(
                            "unknown buckets field: {trimmed:?} (supported: table, granularity)"
                        ),
                    ));
                }
            }
            Section::TopLevel => unreachable!(),
        }
    }

    // close out any in-flight bucket block at EOF
    if let (Some(t), Some(g)) = (bucket_table, bucket_gran) {
        buckets = Some(BucketTagSpec {
            table: t,
            granularity: g,
            dimensions: Vec::new(),
        });
    }

    if !cache_seen {
        return Err(SpecParseError::yaml_file(
            "missing `cache:` block at top level",
        ));
    }
    let ttl = ttl.ok_or_else(|| SpecParseError::yaml_file("missing `ttl:` in cache block"))?;
    Ok(CacheSidecar {
        cache: CacheSidecarBody {
            ttl,
            scope,
            invalidate_on: InvalidateOn {
                tables,
                events,
                buckets,
            },
            stale_while_revalidate: swr,
            max_stale,
            empty_ttl,
            cache_empty,
            time_series: None,
            inner_scope: None,
        },
    })
}

/// v2 parser — path-stack walker. Accepts everything the v1 parser
/// accepted plus the v2 `time_series:` block and `inner_scope:`
/// field, and any future additive block without invasive rewrites.
fn parse_sidecar_yaml_v2(yaml: &str) -> Result<CacheSidecar, SpecParseError> {
    use std::collections::HashMap;

    // Walk the document into a nested map of scalar leaves and list
    // values. Indentation defines structure. Leaves are stored under
    // their full dotted path.
    #[derive(Debug, Default)]
    struct Doc {
        scalars: HashMap<String, String>,
        lists: HashMap<String, Vec<String>>,
        // path -> first line it appeared on (1-indexed).
        seen: HashMap<String, usize>,
    }
    let mut doc = Doc::default();

    // Stack of (indent, dotted_path).
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut current_list_path: Option<(usize, String)> = None;

    for (idx, raw) in yaml.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw);
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();

        // Close out the current list if we've outdented.
        if let Some((list_indent, _)) = &current_list_path {
            if indent <= *list_indent || !trimmed.starts_with("- ") {
                current_list_path = None;
            }
        }

        // Pop the path stack down to the matching indent level.
        while stack.last().map(|(i, _)| *i >= indent).unwrap_or(false) {
            stack.pop();
        }

        // List item under an open list block.
        if let Some(item) = trimmed.strip_prefix("- ") {
            let Some((_, path)) = &current_list_path else {
                return Err(SpecParseError::yaml_at(
                    line_no,
                    format!("unexpected list item outside a list: {trimmed:?}"),
                ));
            };
            doc.lists
                .entry(path.clone())
                .or_default()
                .push(item.trim().trim_matches('"').to_string());
            continue;
        }

        // `key: value?`.
        let (key, rest) = match trimmed.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim().trim_matches('"').to_string()),
            None => {
                return Err(SpecParseError::yaml_at(
                    line_no,
                    format!("expected `key: value`, got {trimmed:?}"),
                ));
            }
        };

        let parent = stack
            .last()
            .map(|(_, p)| format!("{p}."))
            .unwrap_or_default();
        let path = format!("{parent}{key}");
        if doc.seen.insert(path.clone(), line_no).is_some() {
            return Err(SpecParseError::yaml_at(
                line_no,
                format!("duplicate key {path:?}"),
            ));
        }

        if rest.is_empty() {
            // Nested block opens.
            stack.push((indent, path.clone()));
            current_list_path = Some((indent, path));
        } else {
            doc.scalars.insert(path, rest);
        }
    }

    // Path-based extraction. Everything outside the `cache.` prefix
    // is an error.
    let mut cache_seen = false;
    let mut ttl: Option<String> = None;
    let mut scope = CacheScope::default();
    let mut tables: Vec<String> = Vec::new();
    let mut events: Vec<String> = Vec::new();
    let mut buckets: Option<BucketTagSpec> = None;
    let mut swr: Option<String> = None;
    let mut max_stale: Option<String> = None;
    let mut empty_ttl: Option<String> = None;
    let mut cache_empty: Option<bool> = None;
    let mut time_series: Option<TimeSeriesBlock> = None;
    let mut inner_scope: Option<CacheScope> = None;

    for (path, line) in doc.seen.iter() {
        if path == "cache" {
            cache_seen = true;
        } else if !path.starts_with("cache.") {
            return Err(SpecParseError::yaml_at(
                *line,
                format!("unexpected top-level path: {path:?} (only `cache:` allowed)"),
            ));
        }
    }
    if !cache_seen {
        return Err(SpecParseError::yaml_file(
            "missing `cache:` block at top level",
        ));
    }

    // Direct cache fields.
    for (path, v) in &doc.scalars {
        match path.as_str() {
            "cache.ttl" => ttl = Some(v.clone()),
            "cache.stale_while_revalidate" => swr = Some(v.clone()),
            "cache.max_stale" => max_stale = Some(v.clone()),
            "cache.empty_ttl" => empty_ttl = Some(v.clone()),
            "cache.cache_empty" => cache_empty = Some(matches!(v.as_str(), "true" | "yes" | "on")),
            "cache.scope" => scope = parse_scope_at(v, doc.seen.get(path).copied().unwrap_or(0))?,
            "cache.inner_scope" => {
                inner_scope = Some(parse_scope_at(v, doc.seen.get(path).copied().unwrap_or(0))?)
            }
            "cache.invalidate_on.buckets.table" | "cache.invalidate_on.buckets.granularity" => { /* handled below */
            }
            "cache.time_series.time_param"
            | "cache.time_series.range_param"
            | "cache.time_series.bucket"
            | "cache.time_series.tail_ttl"
            | "cache.time_series.body_ttl"
            | "cache.time_series.align_to" => { /* handled below */ }
            other if other.starts_with("cache.invalidate_on.") => {
                return Err(SpecParseError::yaml_at(
                    doc.seen.get(other).copied().unwrap_or(0),
                    format!("unknown invalidate_on field: {other:?}"),
                ));
            }
            other if other.starts_with("cache.time_series.") => {
                return Err(SpecParseError::yaml_at(
                    doc.seen.get(other).copied().unwrap_or(0),
                    format!("unknown time_series field: {other:?}"),
                ));
            }
            other if other.starts_with("cache.") => {
                return Err(SpecParseError::yaml_at(
                    doc.seen.get(other).copied().unwrap_or(0),
                    format!("unknown cache field: {other:?}"),
                ));
            }
            _ => {}
        }
    }

    // List fields.
    for (path, items) in &doc.lists {
        match path.as_str() {
            "cache.invalidate_on.tables" => tables = items.clone(),
            "cache.invalidate_on.events" => events = items.clone(),
            "cache.invalidate_on.buckets.dimensions" => { /* read above */ }
            other => {
                return Err(SpecParseError::yaml_file(format!(
                    "unknown list field: {other:?}"
                )));
            }
        }
    }

    // Bucket subscription block.
    let bt = doc
        .scalars
        .get("cache.invalidate_on.buckets.table")
        .cloned();
    let bg = doc
        .scalars
        .get("cache.invalidate_on.buckets.granularity")
        .cloned();
    match (bt, bg) {
        (Some(t), Some(g)) => {
            let dims = doc
                .lists
                .get("cache.invalidate_on.buckets.dimensions")
                .cloned()
                .unwrap_or_default();
            buckets = Some(BucketTagSpec {
                table: t,
                granularity: g,
                dimensions: dims,
            })
        }
        (None, None) => {}
        _ => {
            return Err(SpecParseError::yaml_file(
                "buckets block needs both `table` and `granularity`".to_string(),
            ))
        }
    }

    // time_series block.
    let any_ts = [
        "cache.time_series.time_param",
        "cache.time_series.range_param",
        "cache.time_series.bucket",
        "cache.time_series.tail_ttl",
        "cache.time_series.body_ttl",
        "cache.time_series.align_to",
    ]
    .iter()
    .any(|k| doc.scalars.contains_key(*k));
    if any_ts {
        let req = |k: &str| -> Result<String, SpecParseError> {
            doc.scalars.get(k).cloned().ok_or_else(|| {
                SpecParseError::yaml_file(format!(
                    "time_series block missing `{}`",
                    k.rsplit('.').next().unwrap_or(k)
                ))
            })
        };
        time_series = Some(TimeSeriesBlock {
            time_param: req("cache.time_series.time_param")?,
            range_param: req("cache.time_series.range_param")?,
            bucket: req("cache.time_series.bucket")?,
            tail_ttl: req("cache.time_series.tail_ttl")?,
            body_ttl: req("cache.time_series.body_ttl")?,
            align_to: doc
                .scalars
                .get("cache.time_series.align_to")
                .cloned()
                .unwrap_or_else(|| "utc".to_string()),
        });
    }

    let ttl = ttl.ok_or_else(|| SpecParseError::yaml_file("missing `ttl:` in cache block"))?;

    Ok(CacheSidecar {
        cache: CacheSidecarBody {
            ttl,
            scope,
            invalidate_on: InvalidateOn {
                tables,
                events,
                buckets,
            },
            stale_while_revalidate: swr,
            max_stale,
            empty_ttl,
            cache_empty,
            time_series,
            inner_scope,
        },
    })
}

fn parse_scope_at(v: &str, line: usize) -> Result<CacheScope, SpecParseError> {
    match v.trim() {
        "user" => Ok(CacheScope::User),
        "tenant" => Ok(CacheScope::Tenant),
        "global" => Ok(CacheScope::Global),
        other => Err(SpecParseError::yaml_at(
            line,
            format!("invalid scope {other:?}: expected user|tenant|global"),
        )),
    }
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_sidecar() {
        let y = r#"
cache:
  ttl: 60s
  scope: user
  invalidate_on:
    tables:
      - com_nubeio_rubixos__readings
      - com_nubeio_rubixos__meters
"#;
        let s = CacheSidecar::from_yaml(y).unwrap().into_spec().unwrap();
        assert_eq!(s.ttl, Duration::from_secs(60));
        assert_eq!(s.scope, CacheScope::User);
        assert_eq!(
            s.derived_tags(),
            vec![
                "table:com_nubeio_rubixos__readings".to_string(),
                "table:com_nubeio_rubixos__meters".to_string(),
            ]
        );
    }

    #[test]
    fn parse_minimal_sidecar_defaults_scope_to_tenant() {
        let y = "cache:\n  ttl: 5m\n";
        let s = CacheSidecar::from_yaml(y).unwrap().into_spec().unwrap();
        assert_eq!(s.ttl, Duration::from_secs(300));
        assert_eq!(s.scope, CacheScope::Tenant);
        assert!(s.invalidate_on.tables.is_empty());
        // defaults
        assert_eq!(s.stale_while_revalidate, Duration::ZERO);
        assert_eq!(s.max_stale, Duration::from_secs(600));
        assert_eq!(s.empty_ttl, Duration::from_secs(5));
        assert!(s.cache_empty);
    }

    #[test]
    fn parse_rejects_unknown_field() {
        let y = "cache:\n  ttl: 1s\n  bogus: 30s\n";
        assert!(CacheSidecar::from_yaml(y).is_err());
    }

    #[test]
    fn parse_rejects_bad_ttl() {
        let y = "cache:\n  ttl: nope\n";
        let s = CacheSidecar::from_yaml(y).unwrap();
        assert!(s.into_spec().is_err());
    }

    #[test]
    fn parse_accepts_swr_and_empty_ttl_keys() {
        let y = r#"
cache:
  ttl: 60s
  stale_while_revalidate: 30s
  max_stale: 90s
  empty_ttl: 2s
  cache_empty: false
"#;
        let s = CacheSidecar::from_yaml(y).unwrap().into_spec().unwrap();
        assert_eq!(s.stale_while_revalidate, Duration::from_secs(30));
        assert_eq!(s.max_stale, Duration::from_secs(90));
        assert_eq!(s.empty_ttl, Duration::from_secs(2));
        assert!(!s.cache_empty);
    }

    #[test]
    fn parse_accepts_events_and_buckets() {
        let y = r#"
cache:
  ttl: 60s
  invalidate_on:
    tables:
      - readings
    events:
      - ingest.batch.committed
    buckets:
      table: readings
      granularity: 1h
"#;
        let s = CacheSidecar::from_yaml(y).unwrap().into_spec().unwrap();
        assert_eq!(s.invalidate_on.events, vec!["ingest.batch.committed"]);
        let b = s.invalidate_on.buckets.as_ref().unwrap();
        assert_eq!(b.table, "readings");
        assert_eq!(b.granularity, "1h");
        // derived tags include event:* and bucket:*
        let tags = s.derived_tags();
        assert!(tags.contains(&"table:readings".to_string()));
        assert!(tags.contains(&"event:ingest.batch.committed".to_string()));
        assert!(tags.contains(&"bucket:readings:1h".to_string()));
    }

    #[test]
    fn empty_ttl_clamped_to_ttl() {
        // empty_ttl default (5s) > ttl (1s) → clamp to 1s
        let y = "cache:\n  ttl: 1s\n";
        let s = CacheSidecar::from_yaml(y).unwrap().into_spec().unwrap();
        assert_eq!(s.empty_ttl, Duration::from_secs(1));
        // explicit override past ttl clamps too
        let y2 = "cache:\n  ttl: 1s\n  empty_ttl: 30s\n";
        let s2 = CacheSidecar::from_yaml(y2).unwrap().into_spec().unwrap();
        assert_eq!(s2.empty_ttl, Duration::from_secs(1));
    }

    #[test]
    fn parse_error_carries_line_number() {
        let y = "cache:\n  ttl: 60s\n  bogus: 30s\n";
        match CacheSidecar::from_yaml(y) {
            Err(SpecParseError::Yaml { line, message }) => {
                assert_eq!(line, 3, "expected error on line 3");
                assert!(
                    message.contains("unknown cache field"),
                    "message should name the problem: {message}"
                );
            }
            other => panic!("expected Yaml line error; got {other:?}"),
        }
    }

    #[test]
    fn parse_error_invalid_scope_carries_line_number() {
        let y = "cache:\n  ttl: 60s\n  scope: bogus\n";
        match CacheSidecar::from_yaml(y) {
            Err(SpecParseError::Yaml { line, message }) => {
                assert_eq!(line, 3);
                assert!(message.contains("scope"), "message: {message}");
            }
            other => panic!("expected Yaml line error; got {other:?}"),
        }
    }

    #[test]
    fn parse_error_missing_cache_block_uses_zero_line_sentinel() {
        let y = "";
        match CacheSidecar::from_yaml(y) {
            Err(SpecParseError::Yaml { line, message }) => {
                assert_eq!(line, 0);
                assert!(message.contains("missing `cache:`"));
            }
            other => panic!("expected Yaml file error; got {other:?}"),
        }
    }

    #[test]
    fn parse_error_display_includes_line_number() {
        let y = "cache:\n  ttl: 60s\n  bogus: 30s\n";
        let err = CacheSidecar::from_yaml(y).unwrap_err();
        let display = err.to_string();
        assert!(
            display.contains("line 3"),
            "display should include the line: {display}"
        );
    }
}
