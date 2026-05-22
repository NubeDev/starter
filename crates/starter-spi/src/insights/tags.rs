//! [`Tags`] — first-class metadata on every `Verdict`/`Dataset`
//! (Insights SCOPE R-ins-8).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Maximum number of tags per emission before truncation kicks in.
pub const MAX_TAGS: usize = 32;

/// A single tag value — either a bare-tag flag (presence is the
/// signal) or a `key:value` pair.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TagValue {
    /// Bare tag — presence is the signal, e.g. `critical`.
    Flag,
    /// `key:value` form, e.g. `building:hq-london`.
    Value(String),
}

impl TagValue {
    /// Construct a `key:value` form.
    pub fn value(v: impl Into<String>) -> Self {
        TagValue::Value(v.into())
    }

    /// Construct a bare-flag form.
    pub fn flag() -> Self {
        TagValue::Flag
    }
}

/// A bag of tags. Ordered (`BTreeMap`) so equality + serialisation
/// are stable for the verdict-log tag index (R-ins-8 mechanical).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tags(pub BTreeMap<String, TagValue>);

impl Tags {
    /// Empty tag set.
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }

    /// Insert a `key:value` tag. Returns `self` for chaining.
    pub fn with_value(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.insert(key.into(), TagValue::Value(value.into()));
        self
    }

    /// Insert a bare-flag tag. Returns `self` for chaining.
    pub fn with_flag(mut self, key: impl Into<String>) -> Self {
        self.0.insert(key.into(), TagValue::Flag);
        self
    }

    /// Insert in place.
    pub fn insert(&mut self, key: impl Into<String>, value: TagValue) {
        self.0.insert(key.into(), value);
    }

    /// Borrow the value for a key.
    pub fn get(&self, key: &str) -> Option<&TagValue> {
        self.0.get(key)
    }

    /// Number of tags in the bag.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the bag is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Union-merge two tag bags. On key collision, `other` wins
    /// (R-ins-8: pipeline-node tags override pack tags). Truncates
    /// at [`MAX_TAGS`]; returns `(merged, truncated)`.
    pub fn merge(mut self, other: Tags) -> (Tags, bool) {
        for (k, v) in other.0 {
            self.0.insert(k, v);
        }
        let truncated = self.0.len() > MAX_TAGS;
        if truncated {
            // Keep the first MAX_TAGS by lexicographic order (the
            // BTreeMap iteration order); stable across runs.
            let kept: BTreeMap<_, _> = self.0.into_iter().take(MAX_TAGS).collect();
            self.0 = kept;
        }
        (self, truncated)
    }

    /// Iterate `(key, value)` pairs in lexicographic key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &TagValue)> {
        self.0.iter()
    }
}
