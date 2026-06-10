//! Typed capability grants.
//!
//! Per SCOPE.md **R6**, the manifest distinguishes two things:
//!
//! - `requires:` — capability *categories* the extension needs to function.
//! - `capabilities:` — runtime *grants* the operator supplies (allowlists,
//!   empty lists for neutralised grants, or scalars for boolean grants).
//!
//! This module models the *grant* side as a typed enum. The host normalises
//! the manifest's `capabilities:` map into a `Vec<Capability>` before any
//! adapter or SDK sees it. Untyped strings never reach the extension's
//! `Ctx`.

use serde::{Deserialize, Serialize};

/// One typed capability grant.
///
/// Variants are stable; adding a category is additive within the manifest
/// schema major. Operators see the category name (e.g. `http_out`) in YAML;
/// the host receives the parsed variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Capability {
    /// Secret store name prefixes the extension is allowed to read.
    /// `Secrets(vec![])` is a legal neutralised grant.
    Secrets {
        /// Allowed `SecretStore` name prefixes (e.g. `"weather:*"`).
        prefixes: Vec<String>,
    },

    /// Outbound HTTP authorities the extension is allowed to call. An empty
    /// vec is a legal neutralised grant: the extension loads, every call is
    /// denied at runtime.
    HttpOut {
        /// Allowed authorities (`host[:port]`).
        authorities: Vec<Authority>,
    },

    /// Filesystem path specs the extension is allowed to read/write. Empty
    /// vec is the neutralised form.
    Fs {
        /// Allowed path specs.
        paths: Vec<PathSpec>,
    },

    /// Permission to read wall-clock time. Boolean grant.
    WallClock {
        /// `true` to grant; the field exists so the YAML reads
        /// `wall_clock: true` rather than `wall_clock: {}`.
        granted: bool,
    },

    /// Named-template warehouse reads against the host's time-series
    /// store (`samples`, `events`, …). The grant enumerates the
    /// tables the extension's templates are allowed to touch — a
    /// template touching `events` cannot be invoked by an extension
    /// whose grant was `tables: [samples]`. An empty `tables` vec is
    /// the neutralised form: the extension loads, every query is
    /// denied.
    ///
    /// Per
    /// [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
    /// row 2, this is the lynchpin grant for Power-BI-style
    /// dashboard authoring.
    WarehouseRead {
        /// Allowed table names. Cross-checked against each
        /// invoked template's `TemplateSpec.tables` at the
        /// supervisor's capability gate.
        #[serde(default)]
        tables: Vec<String>,
    },

    /// Write rows into warehouse tables. Companion to
    /// [`Capability::WarehouseRead`]. The grant enumerates the
    /// unprefixed table names the extension may insert into; the
    /// host clamps every row's `tenant_id` to the caller's tenant
    /// and validates the row's columns against the table schema
    /// declared in `contributes.warehouse_tables[]`. Empty list is
    /// the neutralised form: the extension loads, every insert is
    /// denied.
    ///
    /// Per the Use-Case-B design (extensions producing ad-hoc
    /// schemas via cleaner flows / custom ingest), this is the
    /// only sanctioned write path — the existing
    /// `rubix.warehouse.ingest` tool bypasses capability gating and
    /// is host-side machinery, not extension-reachable.
    WarehouseWrite {
        /// Allowed unprefixed table names. The host resolves each
        /// to `<sanitized_extension_id>__<name>` before issuing the
        /// INSERT, so two extensions cannot collide on a name.
        #[serde(default)]
        tables: Vec<String>,
    },

    /// In-process publish/subscribe bus for cross-extension and
    /// extension↔frontend coordination. Per
    /// [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
    /// row 4: this is the cross-filter / live-update transport
    /// that replaces N-per-click HTTP loopback round-trips.
    ///
    /// The grant carries two topic allowlists — `publish` and
    /// `subscribe`. A topic in `publish: []` cannot be `.publish()`ed
    /// even if the extension declared the capability; same for
    /// `subscribe`. An empty list on either side is the legal
    /// neutralised form for that direction (matches every other
    /// allowlist capability).
    ///
    /// Topic strings are reverse-DNS. The supervisor enforces
    /// namespace ownership the same way it enforces tool ids: an
    /// extension may publish only on topics it owns. Subscription
    /// is open across namespaces — that's the whole point of the
    /// bus — but mediated by the topic allowlist on the grant.
    EventBus {
        /// Allowed publish topics (reverse-DNS, owned by the
        /// extension). Empty vec is the neutralised form: the
        /// extension loads, every publish is denied.
        #[serde(default)]
        publish: Vec<String>,
        /// Allowed subscribe topics (any reverse-DNS topic the
        /// host or another extension publishes). Empty vec is the
        /// neutralised form for subscriptions.
        #[serde(default)]
        subscribe: Vec<String>,
    },

    /// SDUI dashboard pages the extension is allowed to **read**.
    /// Per
    /// [`docs/scope/extensions-north-star`](../../../../rubix/docs/scope/extensions-north-star/README.md)
    /// row 5: extensions that compose flows over the host's
    /// dashboard store reach `ctx.dashboard().read(page_id)`;
    /// this grant is the per-page allowlist the host enforces
    /// before the read fans out. Page ids are stable strings the
    /// host's SDUI page provider already keys off; the
    /// `dashboards_definitions` `page_id` column is the source of
    /// truth. Empty vec is the neutralised form: the extension
    /// loads, every read is denied.
    DashboardRead {
        /// Allowed page ids. Cross-checked against
        /// `ctx.dashboard().read(page_id)` at the host backend.
        #[serde(default)]
        pages: Vec<String>,
    },

    /// SDUI dashboard pages the extension is allowed to **write**.
    /// Distinct from [`Capability::DashboardRead`] so an operator
    /// can grant read-only access (the common posture for AI-builder
    /// extensions that compose dashboards but should not mutate
    /// the operator's authored pages). Empty vec is the neutralised
    /// form: the extension loads, every write is denied.
    DashboardWrite {
        /// Allowed page ids. Cross-checked against
        /// `ctx.dashboard().write(page_id, body)` at the host
        /// backend.
        #[serde(default)]
        pages: Vec<String>,
    },

    /// Resource kinds the extension is allowed to ask the host's
    /// authz engine about via `ctx.authz().check(action, resource)`.
    ///
    /// The grant is a **resource-kind allowlist**, not an action
    /// allowlist: the authz engine itself decides which actions on
    /// which kinds the calling principal may perform — the
    /// extension's job is to *ask*. The kind allowlist exists so
    /// an operator can keep an extension from peeking at the
    /// engine for kinds outside its remit (e.g. an HVAC extension
    /// has no business probing `secrets` decisions). Empty vec is
    /// the neutralised form: the extension loads, every check is
    /// denied.
    AuthzCheck {
        /// Allowed resource kinds (the `kind` half of a
        /// `kind:id` resource reference). Cross-checked against
        /// `ctx.authz().check(...)` at the host backend.
        #[serde(default)]
        kinds: Vec<String>,
    },

    /// Feed data into / drain data out of a host data-plane flow via the
    /// `ingest.*` host methods (`ingest.write` / `ingest.read_batch`). The grant
    /// enumerates the contributed source/sink names (from
    /// `contributes.sources[]` / `contributes.sinks[]`) the extension may push
    /// into or drain. An empty vec is the neutralised form: the extension loads,
    /// every ingest call is denied.
    ///
    /// Tenancy is never the extension's to choose — the host stamps the caller's
    /// tenant onto every written row, so this grant gates *which named flows* an
    /// extension may reach, not *whose data* it may write.
    Ingest {
        /// Allowed contributed source/sink names. Cross-checked against the
        /// `source`/`sink` field of each `ingest.*` request at the host backend.
        #[serde(default)]
        names: Vec<String>,
    },

    /// An opaque, host-defined capability. Used as the escape hatch for
    /// consumer-specific grants the kernel does not know about.
    Custom {
        /// The capability name. Adapters interpret it.
        name: String,
        /// Optional opaque payload (allowlist, policy id, …).
        params: serde_json::Value,
    },
}

/// An HTTP authority allowlist entry (`host[:port]`).
///
/// Newtype rather than `String` so adapters can attach validation/parsing
/// without changing the wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Authority(pub String);

impl Authority {
    /// Borrow the raw `host[:port]` string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A filesystem path spec entry.
///
/// Kept as an opaque string in `starter-ext-spi`; `starter-ext-host` is the
/// crate that resolves it (relative-to-bundle vs. absolute, read-only vs.
/// read-write, glob support). Adapters that enforce capability use see only
/// the string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PathSpec(pub String);

impl PathSpec {
    /// Borrow the raw path-spec string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_round_trip() {
        let cap = Capability::Secrets {
            prefixes: vec!["weather:*".to_string()],
        };
        let j = serde_json::to_value(&cap).unwrap();
        assert_eq!(j["kind"], "secrets");
        let back: Capability = serde_json::from_value(j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn http_out_empty_allowlist_is_legal() {
        let cap = Capability::HttpOut {
            authorities: vec![],
        };
        // SCOPE R6: empty list is the "neutralised grant" form. Just confirm
        // it serialises round-trip; runtime denial happens at the adapter.
        let j = serde_json::to_string(&cap).unwrap();
        let back: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn custom_carries_opaque_params() {
        let cap = Capability::Custom {
            name: "kv".to_string(),
            params: serde_json::json!({ "bucket": "weather" }),
        };
        let j = serde_json::to_string(&cap).unwrap();
        let back: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn warehouse_read_round_trip() {
        let cap = Capability::WarehouseRead {
            tables: vec!["samples".into(), "events".into()],
        };
        let j = serde_json::to_value(&cap).unwrap();
        assert_eq!(j["kind"], "warehouse_read");
        let back: Capability = serde_json::from_value(j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn warehouse_read_empty_tables_is_legal() {
        let cap = Capability::WarehouseRead { tables: vec![] };
        let j = serde_json::to_string(&cap).unwrap();
        let back: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn warehouse_write_round_trip() {
        let cap = Capability::WarehouseWrite {
            tables: vec!["solar_panels".into(), "battery_state".into()],
        };
        let j = serde_json::to_value(&cap).unwrap();
        assert_eq!(j["kind"], "warehouse_write");
        let back: Capability = serde_json::from_value(j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn warehouse_write_empty_tables_is_legal() {
        let cap = Capability::WarehouseWrite { tables: vec![] };
        let j = serde_json::to_string(&cap).unwrap();
        let back: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn event_bus_round_trip() {
        let cap = Capability::EventBus {
            publish: vec!["com.acme.charts.filter".into()],
            subscribe: vec!["com.acme.charts.*".into()],
        };
        let j = serde_json::to_value(&cap).unwrap();
        assert_eq!(j["kind"], "event_bus");
        let back: Capability = serde_json::from_value(j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn event_bus_empty_lists_are_legal() {
        let cap = Capability::EventBus {
            publish: vec![],
            subscribe: vec![],
        };
        let j = serde_json::to_string(&cap).unwrap();
        let back: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn dashboard_read_round_trip() {
        let cap = Capability::DashboardRead {
            pages: vec!["dashboard.disk-overview".into()],
        };
        let j = serde_json::to_value(&cap).unwrap();
        assert_eq!(j["kind"], "dashboard_read");
        let back: Capability = serde_json::from_value(j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn dashboard_write_round_trip() {
        let cap = Capability::DashboardWrite {
            pages: vec!["dashboard.disk-overview".into()],
        };
        let j = serde_json::to_value(&cap).unwrap();
        assert_eq!(j["kind"], "dashboard_write");
        let back: Capability = serde_json::from_value(j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn dashboard_read_empty_pages_is_legal() {
        let cap = Capability::DashboardRead { pages: vec![] };
        let j = serde_json::to_string(&cap).unwrap();
        let back: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn authz_check_round_trip() {
        let cap = Capability::AuthzCheck {
            kinds: vec!["rubix.dashboard.page".into()],
        };
        let j = serde_json::to_value(&cap).unwrap();
        assert_eq!(j["kind"], "authz_check");
        let back: Capability = serde_json::from_value(j).unwrap();
        assert_eq!(back, cap);
    }

    #[test]
    fn authz_check_empty_kinds_is_legal() {
        let cap = Capability::AuthzCheck { kinds: vec![] };
        let j = serde_json::to_string(&cap).unwrap();
        let back: Capability = serde_json::from_str(&j).unwrap();
        assert_eq!(back, cap);
    }
}
