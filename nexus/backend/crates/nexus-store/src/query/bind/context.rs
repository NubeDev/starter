//! The binder's input context: the resolved time range, interval, variables,
//! kind params, and host-bound tokens a single `bind()` call draws from.
//!
//! This is the **versioned C2 contract** (ROADMAP §6 C2, decision D3): v1 ships
//! `time_range` + `interval` + `variables` (consumed by WS-01/WS-02), and the
//! `params` + `host_tokens` fields are populated by the kinds layer (WS-10).
//! Adding a field is additive — callers build a `BindCtx` with `Default` and set
//! only what they have. See docs/design/query/ for the macro grammar.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Utc};

/// The absolute time window a `$__timeFilter`/`$__timeFrom`/`$__timeTo` binds to.
/// Relative ranges (`now-6h`) are resolved to absolute UTC instants *before* the
/// binder runs — the binder never interprets `now`, it only binds instants, so
/// the cache key (C3) and the bound query agree on one snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    /// Inclusive lower bound.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound (`col < to`), matching Grafana's half-open window.
    pub to: DateTime<Utc>,
}

/// A dashboard-variable value (WS-02). A variable is single- or multi-valued;
/// `$var` uses the first value, `$__sqlIn(var)`/`${var:csv}` expand the list.
#[derive(Debug, Clone, PartialEq)]
pub enum VarValue {
    /// A single scalar (text/number/bool) rendered per the interpolation site.
    Single(ScalarValue),
    /// A multi-select / "All" expansion. Empty is allowed and expands to a
    /// no-match guard so a panel with "All" selected and nothing chosen is inert.
    Multi(Vec<ScalarValue>),
}

/// A kind named parameter (WS-10), schema-validated upstream by the kind loader.
/// Always bound as a `$N` arg — never inlined — so a param is injection-inert by
/// construction.
pub type ParamValue = ScalarValue;

/// A scalar carried by a variable or a kind param. Mirrors the bound
/// [`super::SqlValue`] set without the timestamp/null binder internals.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarValue {
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// Host-bound tokens (WS-10): values fixed from the authenticated `Principal`
/// that the caller can never supply. `$caller_tenant_id` is the structural
/// tenant-isolation primitive for the data-side DBs, which have no RLS.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostTokens {
    /// The caller's tenant id, bound from `Principal.tenant_id`.
    pub caller_tenant_id: Option<String>,
    /// The caller's user id, bound from `Principal.subject`.
    pub caller_user_id: Option<String>,
}

/// Everything one `bind()` call needs beyond the SQL text itself. Build with
/// [`Default`] and set the fields you have — absent context makes the macros
/// that need it error (a `$__timeFilter` with no `time_range` is a 4xx), never
/// silently expand to nothing.
#[derive(Debug, Clone, Default)]
pub struct BindCtx {
    // --- v1 (ships first; unblocks WS-01/02) ---
    /// The resolved absolute window for time macros.
    pub time_range: Option<TimeRange>,
    /// The bucket width for `$__timeGroup(col, $__interval)` and `$__interval`.
    pub interval: Option<Duration>,
    /// Dashboard variables, by name without the `$`.
    pub variables: BTreeMap<String, VarValue>,
    // --- WS-10 (kinds) ---
    /// Kind named params, by name without the `$`. Schema-validated upstream.
    pub params: BTreeMap<String, ParamValue>,
    /// Host-bound tokens, never sourced from caller input.
    pub host_tokens: HostTokens,
}
