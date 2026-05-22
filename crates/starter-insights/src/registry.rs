//! [`RuleRegistry`] and [`QualityFlagRegistry`] — process-wide
//! registries of `(namespace, name, major)` ids, populated at
//! extension load time (R-ins-2, R-ins-11).

use std::collections::BTreeMap;
use std::sync::Arc;

use starter_spi::insights::{QualityFlagId, Rule, RuleId};
use thiserror::Error;

/// Registry insertion failure.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RegistryError {
    /// `(namespace, name, major)` triple is already registered. A
    /// breaking change requires a new major.
    #[error("duplicate registration: {0}")]
    Duplicate(String),
}

/// Static description carried alongside a registered
/// [`QualityFlagId`] (R-ins-11). Rendered by the explainer agent
/// and the frontend without per-domain code.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct QualityFlagInfo {
    /// Short human description.
    pub description: String,
    /// Operator-facing remediation hint.
    pub remediation: String,
}

impl QualityFlagInfo {
    /// Construct a [`QualityFlagInfo`].
    pub fn new(description: impl Into<String>, remediation: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            remediation: remediation.into(),
        }
    }
}

/// Rule registry — one per host (R-ins-2).
///
/// Phase 1 ships an in-memory `BTreeMap`-backed registry. The
/// engine's contributor (`block.yaml::contributes.tools` /
/// `register()`) populates this at boot; the registry is read-only
/// thereafter except for hot-reload, which is out of scope here.
#[derive(Default)]
pub struct RuleRegistry {
    rules: BTreeMap<RuleId, Arc<dyn Rule>>,
}

impl RuleRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a rule. Rejects `(namespace, name, major)`
    /// duplicates per R-ins-2; a breaking change requires a new
    /// major.
    pub fn register(&mut self, rule: Arc<dyn Rule>) -> Result<(), RegistryError> {
        let id = rule.schema().id.clone();
        if self.rules.contains_key(&id) {
            return Err(RegistryError::Duplicate(id.to_string()));
        }
        self.rules.insert(id, rule);
        Ok(())
    }

    /// Look up a rule by id.
    pub fn get(&self, id: &RuleId) -> Option<Arc<dyn Rule>> {
        self.rules.get(id).cloned()
    }

    /// List every registered rule id, sorted by `(namespace, name,
    /// major)`.
    pub fn list(&self) -> Vec<RuleId> {
        self.rules.keys().cloned().collect()
    }

    /// Number of registered rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Quality-flag registry (R-ins-11). Same shape as
/// [`RuleRegistry`]; populated by extensions via `register()`.
#[derive(Default)]
pub struct QualityFlagRegistry {
    flags: BTreeMap<QualityFlagId, QualityFlagInfo>,
}

impl QualityFlagRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a flag.
    pub fn register(
        &mut self,
        id: QualityFlagId,
        info: QualityFlagInfo,
    ) -> Result<(), RegistryError> {
        if self.flags.contains_key(&id) {
            return Err(RegistryError::Duplicate(id.to_string()));
        }
        self.flags.insert(id, info);
        Ok(())
    }

    /// Look up info for a registered flag id.
    pub fn get(&self, id: &QualityFlagId) -> Option<&QualityFlagInfo> {
        self.flags.get(id)
    }

    /// List every registered flag id.
    pub fn list(&self) -> Vec<QualityFlagId> {
        self.flags.keys().cloned().collect()
    }

    /// Whether `id` is registered.
    pub fn contains(&self, id: &QualityFlagId) -> bool {
        self.flags.contains_key(id)
    }

    /// Number of registered flags.
    pub fn len(&self) -> usize {
        self.flags.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    /// Pre-populate with the six built-in `starter.quality.*` flags
    /// (R-ins-11 "Built-ins in `starter-spi`").
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        let builtins: &[(&str, &str, &str)] = &[
            (
                "gap",
                "samples missing in a window",
                "check the source's sample cadence; tighten the gap tolerance or fill gaps with a derivation rule",
            ),
            (
                "stuck",
                "N consecutive samples identical",
                "investigate the source; a stuck reading often means a frozen sensor or a cached upstream value",
            ),
            (
                "out-of-range",
                "value outside the rule's declared bounds",
                "verify the sensor calibration; widen the bounds if the new range is legitimate",
            ),
            (
                "rule-error",
                "rule body could not produce an opinion",
                "wire rule.ai-debug downstream of branch(severity=Error) and notify the ops channel",
            ),
            (
                "join-all-inputs-errored",
                "every input to verdict.join errored",
                "inspect each upstream rule's diagnosis; verdict.join cannot synthesise an opinion when no input has one",
            ),
            (
                "tags-truncated",
                "over the 32-tag cap",
                "shed low-cardinality tags at the pipeline-node layer; keep critical routing tags",
            ),
        ];
        for (name, desc, rem) in builtins {
            r.register(
                QualityFlagId::new("starter.quality", *name, 1),
                QualityFlagInfo::new(*desc, *rem),
            )
            .expect("builtin flag ids are unique");
        }
        r
    }
}
