//! [`ExtensionRegistry`] — the read-only view a host wires into adapters.
//!
//! The registry is a small wrapper around `HashMap<String, ExtensionRecord>`
//! keyed by the extension id (or, for records whose manifest failed to
//! parse, by a synthetic `<unparsed:dir-name>` key). Until [`Self::seal`] is
//! called the registry is mutable — `Loader::commit` installs records via
//! [`Self::install`]; after the consumer's `ServerBuilder.build()` no
//! further mutation is permitted (SCOPE.md: "Immutable after the
//! consumer's `ServerBuilder.build()`").
//!
//! The registry returns *records*, not raw manifests, so adapters can
//! filter on lifecycle state (skip `Failed` entries, surface them as
//! diagnostics) using the same shape the supervisor will populate in
//! later phases.

use std::collections::HashMap;

use starter_ext_spi::{ExtensionId, LifecycleState};

use crate::record::ExtensionRecord;

/// Read-mostly registry of every discovered extension.
#[derive(Debug, Default)]
pub struct ExtensionRegistry {
    records: Vec<ExtensionRecord>,
    by_id: HashMap<String, usize>,
    sealed: bool,
}

impl ExtensionRegistry {
    /// Empty, unsealed registry. The expected lifecycle is
    /// `new -> Loader::commit -> seal`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace every record. Called by [`crate::Loader::commit`] under the
    /// two-phase commit (R3 / "Decisions made: two-phase manifest commit"):
    /// the registry never lands in a partial state because installation
    /// happens in one shot.
    ///
    /// Panics if called after [`Self::seal`].
    pub fn install(&mut self, records: HashMap<String, ExtensionRecord>) {
        assert!(
            !self.sealed,
            "starter-ext-host: ExtensionRegistry is sealed; mutation after seal is forbidden"
        );
        let mut ordered: Vec<(String, ExtensionRecord)> = records.into_iter().collect();
        // Sort by key so iteration order is deterministic across runs —
        // adapters that mount routes by walking the registry should not see
        // reorderings between boots.
        ordered.sort_by(|a, b| a.0.cmp(&b.0));

        self.records.clear();
        self.by_id.clear();
        for (key, rec) in ordered {
            let idx = self.records.len();
            self.records.push(rec);
            self.by_id.insert(key, idx);
        }
    }

    /// Mark the registry immutable. Subsequent calls to [`Self::install`]
    /// panic. Adapters that hold a `&ExtensionRegistry` can rely on the
    /// contents being stable for the host's lifetime.
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// `true` once [`Self::seal`] has been called.
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// All records, in deterministic order (by id / id-hint).
    pub fn list(&self) -> &[ExtensionRecord] {
        &self.records
    }

    /// Look up a record by parsed [`ExtensionId`].
    pub fn get(&self, id: &ExtensionId) -> Option<&ExtensionRecord> {
        self.by_id
            .get(id.as_str())
            .and_then(|i| self.records.get(*i))
    }

    /// Look up a record by id string. Convenience wrapper for adapter code
    /// that already holds a `&str`; failed records (whose id never
    /// validated) are *not* findable here — query [`Self::list`] and
    /// filter by `id_hint` for those.
    pub fn get_by_id_str(&self, id: &str) -> Option<&ExtensionRecord> {
        self.by_id.get(id).and_then(|i| self.records.get(*i))
    }

    /// Convenience accessor used by the `GET /extensions/<id>` route in
    /// Phase 2. Returns `LifecycleState::Failed` for unknown ids so adapter
    /// callers do not need to distinguish "missing" from "failed" at the
    /// state-machine level.
    pub fn state(&self, id: &ExtensionId) -> LifecycleState {
        self.get(id)
            .map(|r| r.state)
            .unwrap_or(LifecycleState::Failed)
    }

    /// Iterator over records in the `Validated` state. Adapters typically
    /// only mount routes for these; failed records are diagnostics.
    pub fn iter_validated(&self) -> impl Iterator<Item = &ExtensionRecord> {
        self.records.iter().filter(|r| r.is_validated())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::{ExtensionId, LifecycleState};
    use std::path::PathBuf;

    fn validated_record(id: &str) -> ExtensionRecord {
        ExtensionRecord {
            id: Some(ExtensionId::new(id).unwrap()),
            id_hint: id.to_string(),
            bundle_dir: PathBuf::from("/tmp"),
            state: LifecycleState::Validated,
            manifest: None,
            failure: None,
            origin: crate::BundleOrigin::default(),
        }
    }

    #[test]
    fn install_then_lookup_returns_record() {
        let mut reg = ExtensionRegistry::new();
        let mut m = HashMap::new();
        m.insert("com.acme.a".to_string(), validated_record("com.acme.a"));
        reg.install(m);
        assert!(reg.get_by_id_str("com.acme.a").is_some());
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    #[should_panic(expected = "sealed")]
    fn install_after_seal_panics() {
        let mut reg = ExtensionRegistry::new();
        reg.seal();
        reg.install(HashMap::new());
    }

    #[test]
    fn list_is_deterministic_sorted_by_key() {
        let mut reg = ExtensionRegistry::new();
        let mut m = HashMap::new();
        m.insert("com.acme.b".to_string(), validated_record("com.acme.b"));
        m.insert("com.acme.a".to_string(), validated_record("com.acme.a"));
        reg.install(m);
        let names: Vec<_> = reg.list().iter().map(|r| r.id_hint.clone()).collect();
        assert_eq!(names, vec!["com.acme.a", "com.acme.b"]);
    }
}
