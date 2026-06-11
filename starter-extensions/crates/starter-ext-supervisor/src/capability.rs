//! Capability enforcement at the JSON-RPC wire boundary (SCOPE R8).
//!
//! Per **R8**:
//!
//! > The process supervisor (`starter-ext-supervisor`) cannot enforce
//! > capability use inside the child. It enforces at the **JSON-RPC wire
//! > boundary**: if a child calls a host method it did not declare, the
//! > host returns an error and logs a `capability_violation` event.
//! > Resource caps (cgroups/rlimits) are deferred to v0.2 — they are not
//! > load-bearing for v0.1's trust model.
//!
//! This module is the *gate*: given the set of capability categories the
//! manifest declared and the JSON-RPC method the child invoked on the host,
//! [`CapabilityGate::check`] returns either a green-light or a typed
//! `Error::Capability` that the supervisor's wire loop returns to the
//! child verbatim. The violation is also counted on the per-extension
//! [`CapabilityViolationCounter`] so the admin endpoint can surface it.
//!
//! ## Method ↔ capability mapping
//!
//! Host methods are namespaced (`secrets.get`, `http.request`, `fs.read`,
//! `wall_clock.now_ms`, `tracing.event`). The leading segment is the
//! capability category — so the mapping is "split on `.`, take the first
//! segment, look it up in [`CAPABILITY_HOST_METHODS`]". A method whose
//! leading segment is not a known category (e.g. `init`, `shutdown`,
//! `health`) is *always* allowed — it is part of the substrate, not a
//! capability-gated host call.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

use starter_ext_spi::{Capability, Error};

/// Known capability-gated host methods, grouped by leading namespace
/// segment. Any method not appearing under one of these prefixes is
/// substrate-level (init, shutdown, health, stream.*) and not gated.
///
/// Extending the list is additive within a minor — older extensions never
/// invoke new methods, so adding a new entry cannot break them.
pub const CAPABILITY_HOST_METHODS: &[(&str, &str)] = &[
    ("secrets", "secrets"),
    ("http", "http_out"),
    ("fs", "fs"),
    ("wall_clock", "wall_clock"),
    ("tracing", "tracing"),
    ("warehouse", "warehouse_read"),
    ("event_bus", "event_bus"),
    ("ingest", "ingest"),
    ("datasource", "datasource"),
];

/// Full-method-name overrides, checked **before** the namespace-prefix table.
///
/// A namespace can split into more than one capability category: the
/// `warehouse.*` namespace is read by default (`warehouse.query`), but the
/// mutating methods (`warehouse.write` / `.update` / `.delete`) need the
/// stronger `warehouse_write` grant — an extension that may read its tables
/// should not implicitly be able to write them. So these specific method names
/// map to `warehouse_write`, while every other `warehouse.*` method falls
/// through to `warehouse_read` via [`CAPABILITY_HOST_METHODS`]. Same shape for
/// `datasource.execute` (write) vs `datasource.query` (read).
///
/// Additive within a minor: an older extension never invokes a new method, so a
/// new entry cannot retroactively gate a call it was already making.
pub const CAPABILITY_HOST_METHOD_OVERRIDES: &[(&str, &str)] = &[
    ("warehouse.write", "warehouse_write"),
    ("warehouse.update", "warehouse_write"),
    ("warehouse.delete", "warehouse_write"),
];

/// Per-extension capability gate. Cheap to clone — the granted set is
/// captured once at supervisor construction.
#[derive(Debug, Clone)]
pub struct CapabilityGate {
    granted: HashSet<String>,
}

impl CapabilityGate {
    /// Build from the manifest's `capabilities:` block. Each
    /// [`Capability`] variant maps onto the category name used by
    /// [`CAPABILITY_HOST_METHODS`] — i.e. the same `requires:` category
    /// the extension declares.
    pub fn from_manifest(caps: &[Capability]) -> Self {
        let mut granted = HashSet::new();
        for c in caps {
            granted.insert(category_of(c).to_string());
        }
        Self { granted }
    }

    /// `true` if every method under `category` is allowed. Note this is a
    /// pure *declaration* check: the host returns `Error::Capability`
    /// even when the declared allowlist is empty (`http_out: []`) — that
    /// is enforced one layer up at request time, not here.
    pub fn allows_category(&self, category: &str) -> bool {
        self.granted.contains(category)
    }

    /// Check a JSON-RPC `method` string against the granted capabilities.
    ///
    /// Returns `Ok(None)` for substrate methods (init, shutdown, health,
    /// stream.*), `Ok(Some(category))` for an allowed capability-gated
    /// method, and `Err(Error::Capability)` for a refused one.
    pub fn check(&self, method: &str) -> Result<Option<&'static str>, Error> {
        // Substrate prefixes: never gated.
        if SUBSTRATE_PREFIXES
            .iter()
            .any(|p| method == *p || method.starts_with(&format!("{p}.")))
        {
            return Ok(None);
        }

        // Full-method-name overrides win over the namespace-prefix table, so a
        // namespace that splits read/write (warehouse.*, datasource.*) routes
        // each mutating method to its stronger category.
        let category = CAPABILITY_HOST_METHOD_OVERRIDES
            .iter()
            .find(|(name, _)| *name == method)
            .map(|(_, cat)| *cat)
            .or_else(|| {
                let head = method.split('.').next().unwrap_or(method);
                CAPABILITY_HOST_METHODS
                    .iter()
                    .find(|(prefix, _)| *prefix == head)
                    .map(|(_, cat)| *cat)
            });

        match category {
            None => {
                // Unknown method namespace. Refuse — the supervisor cannot
                // tell what capability would gate it, and silently
                // forwarding to the host risks bypassing R8.
                Err(Error::capability(format!(
                    "host method {method:?} is not a known capability-gated method"
                )))
            }
            Some(cat) if self.granted.contains(cat) => Ok(Some(cat)),
            Some(cat) => Err(Error::capability(format!(
                "extension did not declare capability {cat:?} required by host method {method:?}"
            ))),
        }
    }
}

/// Substrate method names / prefixes that bypass the gate. These belong
/// to the kernel — extensions cannot opt out of them.
const SUBSTRATE_PREFIXES: &[&str] = &["init", "ready", "shutdown", "health", "stream"];

/// Per-extension counter of refused capability calls. Atomic so the
/// reader task (which increments) and the admin endpoint (which reads)
/// can race without locking.
#[derive(Debug, Default)]
pub struct CapabilityViolationCounter {
    inner: AtomicU64,
}

impl CapabilityViolationCounter {
    /// Increment by one.
    pub fn inc(&self) {
        self.inner.fetch_add(1, Ordering::Relaxed);
    }
    /// Current value.
    pub fn get(&self) -> u64 {
        self.inner.load(Ordering::Relaxed)
    }
}

/// Category name for a [`Capability`] variant — the leading namespace
/// segment that [`CapabilityGate::check`] keys off.
fn category_of(c: &Capability) -> &'static str {
    match c {
        Capability::Secrets { .. } => "secrets",
        Capability::HttpOut { .. } => "http_out",
        Capability::Fs { .. } => "fs",
        Capability::WallClock { .. } => "wall_clock",
        Capability::WarehouseRead { .. } => "warehouse_read",
        Capability::WarehouseWrite { .. } => "warehouse_write",
        Capability::EventBus { .. } => "event_bus",
        Capability::Ingest { .. } => "ingest",
        Capability::DashboardRead { .. } => "dashboard_read",
        Capability::DashboardWrite { .. } => "dashboard_write",
        Capability::AuthzCheck { .. } => "authz_check",
        // Both datasource.query (read) and datasource.execute (write) gate on the
        // single `datasource` category — the read/write split is enforced
        // host-side (the prefix rule + allow_foreign_tables), not by a second
        // capability, since one grant names the datasources and the flag widens
        // it. The method-name override below routes `datasource.execute` to the
        // same category; both still require this one grant.
        Capability::Datasource { .. } => "datasource",
        Capability::Custom { .. } => "custom",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::Capability;

    fn gate(cats: &[&str]) -> CapabilityGate {
        // Build by directly mapping category names to representative
        // Capability variants so we don't depend on every constructor
        // shape.
        let caps: Vec<Capability> = cats
            .iter()
            .map(|c| match *c {
                "secrets" => Capability::Secrets { prefixes: vec![] },
                "http_out" => Capability::HttpOut {
                    authorities: vec![],
                },
                "fs" => Capability::Fs { paths: vec![] },
                "wall_clock" => Capability::WallClock { granted: true },
                "warehouse_read" => Capability::WarehouseRead { tables: vec![] },
                "warehouse_write" => Capability::WarehouseWrite { tables: vec![] },
                "event_bus" => Capability::EventBus {
                    publish: vec![],
                    subscribe: vec![],
                },
                "ingest" => Capability::Ingest { names: vec![] },
                other => Capability::Custom {
                    name: other.to_string(),
                    params: serde_json::Value::Null,
                },
            })
            .collect();
        CapabilityGate::from_manifest(&caps)
    }

    #[test]
    fn substrate_methods_pass_without_grant() {
        let g = gate(&[]);
        for m in [
            "init",
            "ready",
            "shutdown",
            "health",
            "stream.event",
            "stream.end",
            "stream.error",
            "stream.cancel",
        ] {
            assert!(g.check(m).is_ok(), "expected {m} to bypass gate");
        }
    }

    #[test]
    fn ungranted_capability_refused() {
        let g = gate(&["secrets"]);
        let err = g.check("http.request").unwrap_err();
        assert!(matches!(err, Error::Capability(_)));
    }

    #[test]
    fn granted_capability_passes() {
        let g = gate(&["http_out"]);
        let cat = g.check("http.request").unwrap().unwrap();
        assert_eq!(cat, "http_out");
    }

    #[test]
    fn unknown_namespace_refused() {
        let g = gate(&["http_out", "secrets", "fs"]);
        let err = g.check("rampage.do_a_thing").unwrap_err();
        assert!(matches!(err, Error::Capability(_)));
    }

    #[test]
    fn ingest_namespace_gated() {
        // Granted: ingest.* passes and maps to the ingest category.
        let g = gate(&["ingest"]);
        assert_eq!(g.check("ingest.write").unwrap().unwrap(), "ingest");
        assert_eq!(g.check("ingest.read_batch").unwrap().unwrap(), "ingest");
        // Ungranted: refused.
        let g = gate(&["secrets"]);
        assert!(matches!(
            g.check("ingest.write").unwrap_err(),
            Error::Capability(_)
        ));
    }

    #[test]
    fn counter_increments() {
        let c = CapabilityViolationCounter::default();
        assert_eq!(c.get(), 0);
        c.inc();
        c.inc();
        assert_eq!(c.get(), 2);
    }

    #[test]
    fn warehouse_read_namespace_gated() {
        // Granted: warehouse.* passes.
        let g = gate(&["warehouse_read"]);
        let cat = g.check("warehouse.query").unwrap().unwrap();
        assert_eq!(cat, "warehouse_read");
        // Ungranted: same call refused.
        let g = gate(&["secrets"]);
        let err = g.check("warehouse.query").unwrap_err();
        assert!(matches!(err, Error::Capability(_)));
    }

    #[test]
    fn warehouse_write_methods_need_write_grant() {
        // warehouse.query maps to warehouse_read; warehouse.write/.update/.delete
        // map to warehouse_write — a read grant does not authorise a write.
        let read_only = gate(&["warehouse_read"]);
        assert_eq!(read_only.check("warehouse.query").unwrap().unwrap(), "warehouse_read");
        for m in ["warehouse.write", "warehouse.update", "warehouse.delete"] {
            assert!(
                matches!(read_only.check(m).unwrap_err(), Error::Capability(_)),
                "{m} must be refused with only warehouse_read"
            );
        }
        // With the write grant, the mutating methods pass and map to the write
        // category; the read method is independent (needs its own grant).
        let writer = gate(&["warehouse_write"]);
        for m in ["warehouse.write", "warehouse.update", "warehouse.delete"] {
            assert_eq!(writer.check(m).unwrap().unwrap(), "warehouse_write");
        }
        assert!(matches!(
            writer.check("warehouse.query").unwrap_err(),
            Error::Capability(_)
        ));
    }

    #[test]
    fn event_bus_namespace_gated() {
        let g = gate(&["event_bus"]);
        let cat = g.check("event_bus.publish").unwrap().unwrap();
        assert_eq!(cat, "event_bus");
        let cat = g.check("event_bus.subscribe").unwrap().unwrap();
        assert_eq!(cat, "event_bus");
        let g = gate(&["secrets"]);
        let err = g.check("event_bus.publish").unwrap_err();
        assert!(matches!(err, Error::Capability(_)));
    }
}
