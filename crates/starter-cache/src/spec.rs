//! `CacheSpec` — the declarative shape an opt-in caller hands to the
//! cache layer. v0 of the [opt-in caching proposal][1] specifies three
//! knobs only: `ttl`, `scope`, `invalidate_on.tables`. Everything else
//! (SWR, `time_series:`, `inner_scope:`, dimension-scoped tags) is
//! deferred.
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

/// What invalidation tags the cache should subscribe an entry to.
/// v0 only takes a list of warehouse table names; the cache derives
/// `table:<name>` tags from these.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidateOn {
    /// Tables whose `table:<name>` tag invalidates this spec.
    #[serde(default)]
    pub tables: Vec<String>,
}

/// The v0 cache spec. Compose with builder methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheSpec {
    /// How long an entry lives before forced refresh.
    pub ttl: Duration,
    /// Who shares this entry.
    pub scope: CacheScope,
    /// Which tags drop this entry's bucket when fired.
    pub invalidate_on: InvalidateOn,
}

impl CacheSpec {
    /// Start a spec with a TTL. Scope defaults to `tenant`, no tags.
    pub fn ttl(ttl: Duration) -> Self {
        Self {
            ttl,
            scope: CacheScope::default(),
            invalidate_on: InvalidateOn::default(),
        }
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

    /// Derive the v0 tag set from this spec. Today: one `table:<name>`
    /// per declared table. The shape is `Vec<String>` so future tag
    /// kinds (`bucket:`, `event:`, …) slot in additively.
    pub fn derived_tags(&self) -> Vec<String> {
        self.invalidate_on
            .tables
            .iter()
            .map(|t| format!("table:{t}"))
            .collect()
    }
}

// ---- YAML wire shape for `kind.cache.yaml` ---------------------------------

/// Wire shape of a `kind.cache.yaml` sidecar.
///
/// ```yaml
/// cache:
///   ttl: 60s
///   scope: user
///   invalidate_on:
///     tables:
///       - com_nubeio_rubixos__readings
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct CacheSidecar {
    /// The single `cache:` block at the top of the sidecar.
    pub cache: CacheSidecarBody,
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
    /// Optional. Empty means "TTL-only, no tag invalidation" —
    /// surprising in production, but the parser allows it.
    #[serde(default)]
    pub invalidate_on: InvalidateOn,
}

/// Errors raised when parsing a sidecar's TTL / structure.
#[derive(Debug, thiserror::Error)]
pub enum SpecParseError {
    /// The TTL string was not in `<n><unit>` shape.
    #[error("invalid ttl {0:?}: expected '<number><s|m|h>' (e.g. '60s')")]
    BadTtl(String),
    /// YAML deserialisation failed.
    #[error("yaml: {0}")]
    Yaml(String),
}

impl CacheSidecar {
    /// Parse a sidecar from a YAML string. The shape is the one
    /// shown in the doc comment on [`CacheSidecar`].
    pub fn from_yaml(yaml: &str) -> Result<Self, SpecParseError> {
        // Avoid pulling serde_yaml into the workspace just for this:
        // do a tiny hand parser for the v0 shape. The shape is
        // dictated by the proposal and is intentionally small.
        parse_sidecar_yaml(yaml)
    }

    /// Materialise a [`CacheSpec`] from the parsed sidecar.
    pub fn into_spec(self) -> Result<CacheSpec, SpecParseError> {
        Ok(CacheSpec {
            ttl: parse_duration(&self.cache.ttl)?,
            scope: self.cache.scope,
            invalidate_on: self.cache.invalidate_on,
        })
    }
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
    let n: u64 = num.parse().map_err(|_| SpecParseError::BadTtl(s.to_string()))?;
    let mul = match unit.trim() {
        "s" | "sec" | "secs" | "seconds" => 1,
        "m" | "min" | "mins" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hours" => 3_600,
        _ => return Err(SpecParseError::BadTtl(s.to_string())),
    };
    Ok(Duration::from_secs(n * mul))
}

/// Hand-rolled YAML parser for the exact v0 sidecar shape — keeps
/// `serde_yaml` out of the workspace dependency surface. Anything
/// beyond the documented shape is rejected. v0 supports:
///
/// ```yaml
/// cache:
///   ttl: 60s
///   scope: user
///   invalidate_on:
///     tables:
///       - foo
///       - bar
/// ```
fn parse_sidecar_yaml(yaml: &str) -> Result<CacheSidecar, SpecParseError> {
    let mut ttl: Option<String> = None;
    let mut scope: CacheScope = CacheScope::default();
    let mut tables: Vec<String> = Vec::new();

    #[derive(PartialEq)]
    enum Section {
        TopLevel,
        Cache,
        InvalidateOn,
        Tables,
    }
    let mut section = Section::TopLevel;
    let mut cache_seen = false;

    for raw in yaml.lines() {
        let line = strip_comment(raw);
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();

        if section == Section::TopLevel {
            if trimmed.starts_with("cache:") && indent == 0 {
                cache_seen = true;
                section = Section::Cache;
                continue;
            }
            return Err(SpecParseError::Yaml(format!(
                "unexpected top-level line: {raw:?} (only `cache:` allowed)"
            )));
        }

        match section {
            Section::Cache => {
                if indent == 0 {
                    return Err(SpecParseError::Yaml(format!(
                        "unexpected top-level line inside cache block: {raw:?}"
                    )));
                }
                if let Some(v) = trimmed.strip_prefix("ttl:") {
                    ttl = Some(v.trim().trim_matches('"').to_string());
                } else if let Some(v) = trimmed.strip_prefix("scope:") {
                    scope = match v.trim() {
                        "user" => CacheScope::User,
                        "tenant" => CacheScope::Tenant,
                        "global" => CacheScope::Global,
                        other => {
                            return Err(SpecParseError::Yaml(format!(
                                "invalid scope {other:?}: expected user|tenant|global"
                            )))
                        }
                    };
                } else if trimmed.starts_with("invalidate_on:") {
                    section = Section::InvalidateOn;
                } else {
                    return Err(SpecParseError::Yaml(format!(
                        "unknown cache field: {raw:?}"
                    )));
                }
            }
            Section::InvalidateOn => {
                if indent <= 2 {
                    // back out to the cache section to handle this line
                    section = Section::Cache;
                    if let Some(v) = trimmed.strip_prefix("ttl:") {
                        ttl = Some(v.trim().trim_matches('"').to_string());
                    } else if trimmed.starts_with("scope:") {
                        // re-walk
                        return Err(SpecParseError::Yaml(format!(
                            "indentation regression at {raw:?}"
                        )));
                    }
                    continue;
                }
                if trimmed.starts_with("tables:") {
                    section = Section::Tables;
                } else {
                    return Err(SpecParseError::Yaml(format!(
                        "unknown invalidate_on field: {raw:?} (v0 supports `tables:` only)"
                    )));
                }
            }
            Section::Tables => {
                if let Some(item) = trimmed.strip_prefix("- ") {
                    tables.push(item.trim().trim_matches('"').to_string());
                } else if indent <= 4 {
                    // exiting the list — re-dispatch this line
                    section = Section::Cache;
                    if trimmed.starts_with("invalidate_on:") {
                        section = Section::InvalidateOn;
                    } else if let Some(v) = trimmed.strip_prefix("ttl:") {
                        ttl = Some(v.trim().trim_matches('"').to_string());
                    }
                } else {
                    return Err(SpecParseError::Yaml(format!(
                        "unknown tables entry: {raw:?}"
                    )));
                }
            }
            Section::TopLevel => unreachable!(),
        }
    }

    if !cache_seen {
        return Err(SpecParseError::Yaml(
            "missing `cache:` block at top level".into(),
        ));
    }
    let ttl = ttl.ok_or_else(|| SpecParseError::Yaml("missing `ttl:` in cache block".into()))?;
    Ok(CacheSidecar {
        cache: CacheSidecarBody {
            ttl,
            scope,
            invalidate_on: InvalidateOn { tables },
        },
    })
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
    }

    #[test]
    fn parse_rejects_unknown_field() {
        let y = "cache:\n  ttl: 1s\n  swr: 30s\n";
        assert!(CacheSidecar::from_yaml(y).is_err());
    }

    #[test]
    fn parse_rejects_bad_ttl() {
        let y = "cache:\n  ttl: nope\n";
        let s = CacheSidecar::from_yaml(y).unwrap();
        assert!(s.into_spec().is_err());
    }
}
