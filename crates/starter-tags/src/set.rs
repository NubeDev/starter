//! `TagSet` — flat `Map<String, TagValue>`, the workspace's shared tag
//! vocabulary value type (T2).

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::TagSetError;

/// A flat tag bag. Keys are arbitrary non-empty strings; values are
/// either `Bool` or `Str` (T2 — no nesting, no arrays, no floats).
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TagSet(pub BTreeMap<String, TagValue>);

/// A tag value. T2 restricts this to `Bool | Str` — there is deliberately
/// no `Num` variant. See [`crate::error::TagSetError::ReservedBoolString`]
/// for the reserved-string rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TagValue {
    Bool(bool),
    Str(String),
}

impl<'de> Deserialize<'de> for TagValue {
    /// Custom deserialiser that mirrors [`TagSet::insert`]: booleans pass
    /// through, integer JSON numbers coerce to their canonical decimal
    /// string, and non-integer / non-finite numbers are rejected.
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        let v = serde_json::Value::deserialize(d)?;
        match v {
            serde_json::Value::Bool(b) => Ok(TagValue::Bool(b)),
            serde_json::Value::String(s) => {
                if is_reserved_bool_string(&s) {
                    return Err(D::Error::custom(format!(
                        "tag value {s:?} is the reserved bool form; use a JSON boolean (T2/M-2)"
                    )));
                }
                Ok(TagValue::Str(s))
            }
            serde_json::Value::Number(n) => {
                let s = canonical_number_string(&n).map_err(D::Error::custom)?;
                Ok(TagValue::Str(s))
            }
            other => Err(D::Error::custom(format!(
                "tag value must be bool|string|integer, got {other:?}"
            ))),
        }
    }
}

impl TagValue {
    /// Convenience: build a `Str` from anything string-ish.
    pub fn str(s: impl Into<String>) -> Self {
        TagValue::Str(s.into())
    }
}

impl TagSet {
    /// New empty set.
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Insert a tag, applying T2 invariants:
    ///
    /// * empty key → [`TagSetError::EmptyKey`]
    /// * `Str("true" | "false" | "TRUE" | "False" | …)` →
    ///   [`TagSetError::ReservedBoolString`] (M-2 closes this footgun).
    ///
    /// This is the only typed-input path; chaotic ingest (e.g.
    /// `raw_events`) does not go through here — see Warehouse SCOPE W7.
    pub fn insert(&mut self, key: impl Into<String>, value: TagValue) -> Result<(), TagSetError> {
        let key = key.into();
        if key.is_empty() {
            return Err(TagSetError::EmptyKey);
        }
        if let TagValue::Str(s) = &value {
            if is_reserved_bool_string(s) {
                return Err(TagSetError::ReservedBoolString {
                    key,
                    value: s.clone(),
                });
            }
        }
        self.0.insert(key, value);
        Ok(())
    }

    /// Bare-tag sugar (T3): `set.insert_bare("sensor")` ≡
    /// `set.insert("sensor", TagValue::Bool(true))`.
    pub fn insert_bare(&mut self, key: impl Into<String>) -> Result<(), TagSetError> {
        self.insert(key, TagValue::Bool(true))
    }

    /// Insert a JSON value as a tag, coercing integer numbers to their
    /// canonical decimal string and rejecting non-integer / non-finite
    /// numbers per T2.
    pub fn insert_json(
        &mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Result<(), TagSetError> {
        let key = key.into();
        let v: TagValue = match value {
            serde_json::Value::Bool(b) => TagValue::Bool(b),
            serde_json::Value::String(s) => TagValue::Str(s),
            serde_json::Value::Number(n) => {
                if let Some(s) = n.as_i64() {
                    TagValue::Str(s.to_string())
                } else if let Some(u) = n.as_u64() {
                    TagValue::Str(u.to_string())
                } else if let Some(f) = n.as_f64() {
                    if !f.is_finite() {
                        return Err(TagSetError::NonFiniteNumber { key });
                    }
                    // Real (non-integer) float — reject per T2.
                    return Err(TagSetError::NonIntegerNumber {
                        key,
                        value: f.to_string(),
                    });
                } else {
                    return Err(TagSetError::NonIntegerNumber {
                        key,
                        value: n.to_string(),
                    });
                }
            }
            other => {
                return Err(TagSetError::NonIntegerNumber {
                    key,
                    value: other.to_string(),
                });
            }
        };
        self.insert(key, v)
    }

    /// Merge another set into this one, last write wins (T2 idiomatic).
    pub fn merge(&mut self, other: &TagSet) {
        for (k, v) in &other.0 {
            self.0.insert(k.clone(), v.clone());
        }
    }

    /// Run a [`crate::query::TagQuery`] against this set in-process
    /// (T8c). The same semantics are produced by the PG and CH
    /// compilers — see `tests/semantic_parity.rs`.
    pub fn matches(&self, q: &crate::query::TagQuery) -> bool {
        crate::compile_match::matches(q, self)
    }

    /// Get a tag value by key.
    pub fn get(&self, key: &str) -> Option<&TagValue> {
        self.0.get(key)
    }

    /// Number of tags in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True if there are no tags.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The single canonical `TagValue → String` conversion used by both
/// `compile_ch` (binding query literals) and `starter-store-warehouse`
/// (writing rows). Defined here so it has exactly one definition (T2).
pub fn tag_value_to_ch_string(v: &TagValue) -> String {
    match v {
        TagValue::Bool(true) => "true".to_owned(),
        TagValue::Bool(false) => "false".to_owned(),
        TagValue::Str(s) => s.clone(),
    }
}

/// `"true"`, `"TRUE"`, `"True"`, `"false"`, `"FALSE"`, `"False"`, … —
/// any case variant of the two reserved boolean strings (M-2).
pub(crate) fn is_reserved_bool_string(s: &str) -> bool {
    s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("false")
}

fn canonical_number_string(n: &serde_json::Number) -> Result<String, String> {
    if let Some(i) = n.as_i64() {
        return Ok(i.to_string());
    }
    if let Some(u) = n.as_u64() {
        return Ok(u.to_string());
    }
    if let Some(f) = n.as_f64() {
        if !f.is_finite() {
            return Err("non-finite JSON number rejected at TagSet construction".to_owned());
        }
        return Err(format!(
            "non-integer JSON number {f} rejected (T2); use a typed column or quote it"
        ));
    }
    Err(format!("uncoercible JSON number {n}"))
}
