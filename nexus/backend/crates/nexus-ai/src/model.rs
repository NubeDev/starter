//! Shared model addressing. The single biggest ergonomic win of the facade is
//! that callers say "give me a large model" without caring which provider is
//! wired up. `ModelRef` carries either a concrete id or a size alias; the active
//! provider resolves it. Both libraries already have this concept (genai model
//! aliasing, zag's small/medium/large) — here it is normalised to one type.

use serde::{Deserialize, Serialize};

/// A capability tier. Maps to a concrete model per provider at resolve time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Size {
    Small,
    Medium,
    Large,
}

/// How a caller names the model they want.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ModelRef {
    /// A provider-native id, e.g. `"claude-opus-4-8"`. Passed through verbatim.
    Concrete(String),
    /// A size tier resolved per provider.
    Alias(Size),
}

impl ModelRef {
    pub fn concrete(id: impl Into<String>) -> Self {
        Self::Concrete(id.into())
    }

    pub const fn small() -> Self {
        Self::Alias(Size::Small)
    }
    pub const fn medium() -> Self {
        Self::Alias(Size::Medium)
    }
    pub const fn large() -> Self {
        Self::Alias(Size::Large)
    }
}

impl From<&str> for ModelRef {
    fn from(s: &str) -> Self {
        Self::Concrete(s.to_string())
    }
}

impl From<String> for ModelRef {
    fn from(s: String) -> Self {
        Self::Concrete(s)
    }
}

impl From<Size> for ModelRef {
    fn from(s: Size) -> Self {
        Self::Alias(s)
    }
}

/// Per-provider mapping of size tiers to concrete model ids. A provider impl
/// holds one of these and uses it to resolve [`ModelRef::Alias`]. Defaulting to
/// Claude ids since nexus defaults to Claude models.
#[derive(Debug, Clone)]
pub struct AliasMap {
    pub small: String,
    pub medium: String,
    pub large: String,
}

impl Default for AliasMap {
    fn default() -> Self {
        Self {
            small: "claude-haiku-4-5".to_string(),
            medium: "claude-sonnet-4-6".to_string(),
            large: "claude-opus-4-8".to_string(),
        }
    }
}

impl AliasMap {
    /// Resolve a [`ModelRef`] to a concrete model id string.
    pub fn resolve(&self, m: &ModelRef) -> String {
        match m {
            ModelRef::Concrete(id) => id.clone(),
            ModelRef::Alias(Size::Small) => self.small.clone(),
            ModelRef::Alias(Size::Medium) => self.medium.clone(),
            ModelRef::Alias(Size::Large) => self.large.clone(),
        }
    }
}
