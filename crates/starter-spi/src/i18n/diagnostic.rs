//! `Diagnostic` and `DiagnosticParam` — the wire shape a translatable
//! error or info message takes on its way through the HTTP / event
//! surfaces.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::MessageKey;

/// A typed value interpolated into a translated message.
///
/// The variants are intentionally narrow: anything that survives a
/// JSON wire trip plus the R1-mandated `Timestamp` (UTC epoch ms). The
/// JSON form is externally tagged in snake_case (e.g.
/// `{"string": "hello"}`, `{"timestamp": 1_700_000_000_000}`), which
/// keeps the deserialiser simple and the schema discoverable in
/// `openapi.json` per workspace R7.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticParam {
    /// A textual value, e.g. a user-supplied label.
    String(String),
    /// A signed 64-bit integer, e.g. a count or an ID.
    I64(i64),
    /// A 64-bit float, e.g. a measured value.
    F64(f64),
    /// A boolean flag.
    Bool(bool),
    /// A UTC instant carried as epoch milliseconds. Per R1 of
    /// `DOCS/user/scope/SCOPE.md` (Hard rules), timestamps on the wire
    /// are always epoch milliseconds in UTC; the client renders into
    /// the resolved timezone.
    Timestamp(i64),
}

/// A translatable diagnostic — the code plus the typed parameters the
/// translation interpolates.
///
/// `params` is a [`BTreeMap`] (not a [`HashMap`]) so the JSON wire form
/// is deterministic — keys serialise in lexicographic order. This
/// matches the posture `starter-flow-spi` takes on `SlotMap` and makes
/// snapshot tests stable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Diagnostic {
    /// Reverse-DNS-style message key, e.g. `"auth.token.expired"`.
    pub code: MessageKey,
    /// Named parameters the translation interpolates. Order on the
    /// wire is the natural ordering of `BTreeMap<String, …>`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, DiagnosticParam>,
}

impl Diagnostic {
    /// Construct a diagnostic carrying just a code and no parameters.
    pub fn new(code: MessageKey) -> Self {
        Self {
            code,
            params: BTreeMap::new(),
        }
    }

    /// Builder-style: attach a parameter by name.
    pub fn with_param(mut self, name: impl Into<String>, value: DiagnosticParam) -> Self {
        self.params.insert(name.into(), value);
        self
    }
}
