//! Default [`ResourceRegistry`] implementation: a `RwLock<HashMap>`
//! keyed by `kind`. Append-only at boot; double-registration
//! panics (SCOPE.md "two extensions register the same kind").

use std::collections::HashMap;
use std::sync::RwLock;

use starter_spi::authz::{ResourceRegistry, ResourceSpec};

use crate::error::{Error, Result};

/// In-memory registry. Construct one at boot, hand `Arc<Self>` to
/// the engine and to each extension's `init` hook.
#[derive(Debug, Default)]
pub struct StaticRegistry {
    inner: RwLock<HashMap<String, ResourceSpec>>,
}

impl StaticRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Non-panicking sibling of [`Self::register`]. Returns
    /// [`Error::DuplicateResource`] instead of panicking — useful
    /// in tests that exercise the duplicate path.
    pub fn try_register(&self, spec: ResourceSpec) -> Result<()> {
        let mut g = self.inner.write().expect("registry lock poisoned");
        if g.contains_key(&spec.kind) {
            return Err(Error::DuplicateResource { kind: spec.kind });
        }
        g.insert(spec.kind.clone(), spec);
        Ok(())
    }

    /// Convenience: register and return `&self`. Suitable for
    /// chains at boot.
    pub fn register_spec(&self, spec: ResourceSpec) -> &Self {
        self.register(spec);
        self
    }
}

impl ResourceRegistry for StaticRegistry {
    fn register(&self, spec: ResourceSpec) {
        // SCOPE.md "Extension story": double-registration panics
        // because silent shadowing is the worst failure mode.
        if let Err(Error::DuplicateResource { kind }) = self.try_register(spec) {
            panic!("authz: resource `{kind}` registered twice");
        }
    }

    fn known(&self) -> Vec<ResourceSpec> {
        let g = self.inner.read().expect("registry lock poisoned");
        let mut out: Vec<ResourceSpec> = g.values().cloned().collect();
        // Stable order for the admin UI and the dry-run endpoint.
        out.sort_by(|a, b| a.kind.cmp(&b.kind));
        out
    }

    fn lookup(&self, kind: &str) -> Option<ResourceSpec> {
        let g = self.inner.read().expect("registry lock poisoned");
        g.get(kind).cloned()
    }
}
