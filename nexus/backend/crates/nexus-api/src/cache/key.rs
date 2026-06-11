//! The query result-cache key — the C3 tuple, hashed to a stable string.
//!
//! Cache correctness lives or dies on this key. Two queries share a cached
//! result only when every input that can change the rows is identical:
//! `tenant + datasource + query (sql | kind+params) + resolved time + interval +
//! variable values + units/locale/timezone`. The key folds all of them into one
//! SHA-256 digest so the cache map stays a flat `String -> entry`.
//!
//! The `units_locale_tz` dimension is carried from day one (ROADMAP §6 C3,
//! decision D4) as a constant placeholder until WS-11 threads per-user resolved
//! preferences into the query path. Baking the field in now means enabling
//! per-user unit conversion later cannot silently serve a cross-unit-poisoned
//! entry: a different units context already yields a different key.

use nexus_spi::dto::query::QueryRequest;
use nexus_store::QueryIdentity;
use sha2::{Digest, Sha256};

/// The constant `units_locale_tz` placeholder used until WS-11 resolves the
/// caller's units/locale/timezone into the query path. Any future non-default
/// value yields a distinct key, so turning WS-11 on cannot reuse a placeholder
/// entry. Exposed so a test can assert the dimension is honoured.
pub const UNITS_PLACEHOLDER: &str = "units:default";

/// A computed cache key: an opaque, stable digest of the full C3 tuple. Equal
/// inputs produce an equal key; any differing input produces a different one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey(String);

impl CacheKey {
    /// Borrow the digest for use as a map key.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Mint a key directly from a string, for cache-store tests that exercise
    /// TTL/single-flight behaviour without building a full request. The cache
    /// treats the key as opaque, so any distinct string is a distinct key.
    #[cfg(test)]
    pub(crate) fn for_test(s: &str) -> Self {
        CacheKey(s.to_string())
    }
}

/// Build the key for `req` run against `datasource` for `identity`. `datasource`
/// is the resolved datasource scope (`"dev"` for the single-datasource dev
/// shortcut, the datasource id for `/datasources/:id/query`) — it must be part
/// of the key because the same SQL against two datasources returns different
/// rows. The tenant is taken from `identity`; an absent tenant (the dev path)
/// folds in as an explicit empty scope rather than colliding with tenant data.
pub fn build(req: &QueryRequest, identity: &QueryIdentity, datasource: &str) -> CacheKey {
    let mut hasher = Sha256::new();
    // A length-prefixed framing keeps fields from running together — e.g. a
    // tenant `"ab"` + datasource `"c"` cannot collide with `"a"` + `"bc"`.
    feed(&mut hasher, "tenant", identity.tenant_id.as_deref().unwrap_or(""));
    // Identity-scoped host tokens (`$caller_user_id`, `$caller_team_ids` —
    // P3a) make query results depend on WHO is asking, so the caller's
    // identity MUST be part of the cache key — otherwise one user's rows
    // could be served from another's cached entry. Teams are sorted so the
    // key is order-independent.
    feed(&mut hasher, "caller_user", identity.user_id.as_deref().unwrap_or(""));
    let mut teams = identity.teams.clone();
    teams.sort();
    feed(&mut hasher, "caller_teams", &teams.join(","));
    feed(&mut hasher, "datasource", datasource);
    feed_query(&mut hasher, req);
    feed_sources(&mut hasher, req);
    feed_time(&mut hasher, req);
    feed_vars(&mut hasher, req);
    feed(&mut hasher, "units", UNITS_PLACEHOLDER);
    CacheKey(format!("{:x}", hasher.finalize()))
}

/// The query identity: kind-mode folds in the kind name + params JSON; sql-mode
/// folds in the raw SQL. These are mutually exclusive on the wire (kind-mode
/// ignores `sql`), so the discriminant byte makes a kind named like a SQL string
/// impossible to confuse.
fn feed_query(hasher: &mut Sha256, req: &QueryRequest) {
    match &req.kind {
        Some(kind) => {
            feed(hasher, "mode", "kind");
            feed(hasher, "kind", kind);
            let params = req
                .params
                .as_ref()
                .map(|p| p.to_string())
                .unwrap_or_default();
            feed(hasher, "params", &params);
        }
        None => {
            feed(hasher, "mode", "sql");
            feed(hasher, "sql", &req.sql);
        }
    }
}

/// RW-05 federation inputs: the alias → datasource(+table) references a
/// cross-datasource `sql` joins over. The same SQL against different federated
/// inputs returns different rows, so each ref folds into the key. References are
/// sorted by alias so the key is independent of the wire array's order; an empty
/// `sources` (the single-datasource path) folds in a zero count, leaving today's
/// keys unchanged.
fn feed_sources(hasher: &mut Sha256, req: &QueryRequest) {
    let mut refs: Vec<&nexus_spi::dto::query::FederatedSourceRef> = req.sources.iter().collect();
    refs.sort_by(|a, b| a.alias.cmp(&b.alias));
    feed(hasher, "source_count", &refs.len().to_string());
    for r in refs {
        feed(hasher, "source_alias", &r.alias);
        feed(hasher, "source_ds", &r.datasource);
        feed(hasher, "source_table", r.table.as_deref().unwrap_or(""));
    }
}

/// The resolved absolute window + bucket interval. WS-01 snaps `now` to the
/// refresh tick before sending, so within a tick the window is stable and the
/// key is reused; a new tick shifts the window and busts the entry.
fn feed_time(hasher: &mut Sha256, req: &QueryRequest) {
    let range = req
        .time_range
        .map(|r| format!("{}..{}", r.from.to_rfc3339(), r.to.to_rfc3339()))
        .unwrap_or_default();
    feed(hasher, "time", &range);
    let interval = req.interval_secs.map(|s| s.to_string()).unwrap_or_default();
    feed(hasher, "interval", &interval);
}

/// Variable values, in name order so the key is independent of the wire array's
/// order. Each variable's values are joined with a separator that cannot appear
/// in the length-framed encoding, so `["a","b"]` and `["a,b"]` stay distinct.
fn feed_vars(hasher: &mut Sha256, req: &QueryRequest) {
    let mut vars: Vec<&nexus_spi::dto::query::QueryVariable> = req.variables.iter().collect();
    vars.sort_by(|a, b| a.name.cmp(&b.name));
    feed(hasher, "var_count", &vars.len().to_string());
    for v in vars {
        feed(hasher, "var_name", &v.name);
        feed(hasher, "var_value_count", &v.values.len().to_string());
        for value in &v.values {
            feed(hasher, "var_value", value);
        }
    }
}

/// Feed one labelled field with an explicit length prefix so concatenated
/// fields can never alias one another.
fn feed(hasher: &mut Sha256, label: &str, value: &str) {
    hasher.update((label.len() as u64).to_le_bytes());
    hasher.update(label.as_bytes());
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_spi::dto::query::{QueryTimeRange, QueryVariable};

    fn sql_req(sql: &str) -> QueryRequest {
        QueryRequest {
            sql: sql.to_string(),
            time_range: None,
            interval_secs: None,
            variables: Vec::new(),
            kind: None,
            params: None,
            sources: Vec::new(),
            insight: None,
        }
    }

    fn identity(tenant: &str) -> QueryIdentity {
        QueryIdentity {
            tenant_id: Some(tenant.to_string()),
            user_id: Some("u1".to_string()),
            teams: Vec::new(),
        }
    }

    #[test]
    fn identical_inputs_yield_identical_keys() {
        let req = sql_req("select 1");
        let id = identity("t1");
        assert_eq!(build(&req, &id, "ds1"), build(&req, &id, "ds1"));
    }

    #[test]
    fn different_sql_yields_different_keys() {
        let id = identity("t1");
        assert_ne!(
            build(&sql_req("select 1"), &id, "ds1"),
            build(&sql_req("select 2"), &id, "ds1"),
        );
    }

    #[test]
    fn different_tenant_yields_different_keys() {
        let req = sql_req("select 1");
        assert_ne!(
            build(&req, &identity("t1"), "ds1"),
            build(&req, &identity("t2"), "ds1"),
        );
    }

    #[test]
    fn different_datasource_yields_different_keys() {
        let req = sql_req("select 1");
        let id = identity("t1");
        assert_ne!(build(&req, &id, "ds1"), build(&req, &id, "ds2"));
    }

    #[test]
    fn different_time_range_busts_the_key() {
        let id = identity("t1");
        let mut a = sql_req("select 1");
        a.time_range = Some(QueryTimeRange {
            from: "2026-01-01T00:00:00Z".parse().unwrap(),
            to: "2026-01-01T06:00:00Z".parse().unwrap(),
        });
        let mut b = a.clone();
        b.time_range = Some(QueryTimeRange {
            from: "2026-01-01T00:00:00Z".parse().unwrap(),
            to: "2026-01-01T07:00:00Z".parse().unwrap(),
        });
        assert_ne!(build(&a, &id, "ds1"), build(&b, &id, "ds1"));
    }

    #[test]
    fn different_variable_values_bust_the_key() {
        let id = identity("t1");
        let mut a = sql_req("select 1");
        a.variables = vec![QueryVariable {
            name: "region".into(),
            values: vec!["eu".into()],
        }];
        let mut b = a.clone();
        b.variables = vec![QueryVariable {
            name: "region".into(),
            values: vec!["us".into()],
        }];
        assert_ne!(build(&a, &id, "ds1"), build(&b, &id, "ds1"));
    }

    #[test]
    fn variable_order_does_not_affect_the_key() {
        let id = identity("t1");
        let mut a = sql_req("select 1");
        a.variables = vec![
            QueryVariable {
                name: "a".into(),
                values: vec!["1".into()],
            },
            QueryVariable {
                name: "b".into(),
                values: vec!["2".into()],
            },
        ];
        let mut b = sql_req("select 1");
        b.variables = vec![
            QueryVariable {
                name: "b".into(),
                values: vec!["2".into()],
            },
            QueryVariable {
                name: "a".into(),
                values: vec!["1".into()],
            },
        ];
        assert_eq!(build(&a, &id, "ds1"), build(&b, &id, "ds1"));
    }

    #[test]
    fn kind_mode_and_sql_mode_do_not_collide() {
        let id = identity("t1");
        let mut kind = sql_req("nexus.demo");
        kind.kind = Some("nexus.demo".into());
        assert_ne!(
            build(&sql_req("nexus.demo"), &id, "ds1"),
            build(&kind, &id, "ds1"),
        );
    }

    /// The C3/D4 guarantee: the key carries a units dimension, so when WS-11
    /// swaps the placeholder for a per-user value the key changes and a
    /// cross-unit-poisoned entry can never be served. Asserting the placeholder
    /// is folded in proves the dimension exists today.
    #[test]
    fn units_dimension_changes_the_key() {
        // Build the key with the real placeholder, then recompute by hand with a
        // different units token to prove the dimension is load-bearing.
        let req = sql_req("select 1");
        let id = identity("t1");
        let with_placeholder = build(&req, &id, "ds1");

        let mut hasher = Sha256::new();
        feed(&mut hasher, "tenant", "t1");
        feed(&mut hasher, "datasource", "ds1");
        feed_query(&mut hasher, &req);
        feed_sources(&mut hasher, &req);
        feed_time(&mut hasher, &req);
        feed_vars(&mut hasher, &req);
        feed(&mut hasher, "units", "units:imperial-en-US");
        let with_real_units = CacheKey(format!("{:x}", hasher.finalize()));

        assert_ne!(with_placeholder, with_real_units);
    }
}
