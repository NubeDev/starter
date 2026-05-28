//! Which in-process registry an [`RegistryItem`](super::item::RegistryItem)
//! was projected from.
//!
//! Wire shape is lowercase singular (`tool`, `node`, `rule`,
//! `template`, `table`, `skill`, `extension`). The string form is
//! reused as the query value for `GET /admin/registry?kinds=`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The set of registries the admin surface projects.
///
/// Names are lowercase singular so a comma-separated query string
/// (`?kinds=tool,node`) is unambiguous and round-trips through
/// `serde` and `Display`/`FromStr` identically.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum RegistryKind {
    /// MCP / REST tools advertised by the agent.
    Tool,
    /// Flow node-kinds known to the engine.
    Node,
    /// Cleaner anomaly rules.
    Rule,
    /// Warehouse-read templates.
    Template,
    /// Warehouse tables contributed by extensions.
    Table,
    /// Skill bundles loaded by the skill registry.
    Skill,
    /// Installed extensions (`/api/v1/extensions` is the canonical
    /// detail surface — this projection only carries the summary
    /// row each extension exposes).
    Extension,
}

impl RegistryKind {
    /// Every kind, in admin-console display order.
    pub const ALL: &'static [RegistryKind] = &[
        RegistryKind::Tool,
        RegistryKind::Node,
        RegistryKind::Rule,
        RegistryKind::Template,
        RegistryKind::Table,
        RegistryKind::Skill,
        RegistryKind::Extension,
    ];

    /// Lowercase string form used on the wire and in URL paths.
    pub fn as_str(self) -> &'static str {
        match self {
            RegistryKind::Tool => "tool",
            RegistryKind::Node => "node",
            RegistryKind::Rule => "rule",
            RegistryKind::Template => "template",
            RegistryKind::Table => "table",
            RegistryKind::Skill => "skill",
            RegistryKind::Extension => "extension",
        }
    }
}

impl fmt::Display for RegistryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RegistryKind {
    type Err = UnknownKind;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tool" => Ok(RegistryKind::Tool),
            "node" => Ok(RegistryKind::Node),
            "rule" => Ok(RegistryKind::Rule),
            "template" => Ok(RegistryKind::Template),
            "table" => Ok(RegistryKind::Table),
            "skill" => Ok(RegistryKind::Skill),
            "extension" => Ok(RegistryKind::Extension),
            _ => Err(UnknownKind(s.to_owned())),
        }
    }
}

/// Returned by [`RegistryKind::from_str`] when the input string did
/// not match any known kind. Carries the offending input so the
/// transport layer can surface a precise 400.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKind(pub String);

impl fmt::Display for UnknownKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown registry kind: {}", self.0)
    }
}

impl std::error::Error for UnknownKind {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_str_and_serde() {
        for kind in RegistryKind::ALL {
            let s = kind.as_str();
            assert_eq!(*kind, RegistryKind::from_str(s).unwrap());
            let json = serde_json::to_string(kind).unwrap();
            assert_eq!(json, format!("\"{s}\""));
        }
    }

    #[test]
    fn unknown_kind_carries_input() {
        let err = RegistryKind::from_str("nope").unwrap_err();
        assert_eq!(err.0, "nope");
    }
}
