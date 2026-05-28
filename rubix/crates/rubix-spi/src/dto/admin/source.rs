//! Where a [`RegistryItem`](super::item::RegistryItem) came from.
//!
//! Tagged union (serde `tag = "kind"`) so a consumer can switch on
//! `source.kind` without parsing strings. `extension` carries the
//! reverse-DNS id so the console can deep-link to the extension's
//! detail page.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Provenance of a registry item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ItemSource {
    /// Provided by the rubix-agent binary itself.
    Builtin,
    /// Provided by an upstream `starter-*` crate (not by rubix and
    /// not by an extension).
    Starter,
    /// Provided by an installed extension. `id` is the
    /// reverse-DNS extension identifier.
    Extension {
        /// Reverse-DNS extension id (e.g. `com.rubix.example`).
        id: String,
    },
}

impl ItemSource {
    /// `true` when the source is a specific extension.
    pub fn is_extension(&self) -> bool {
        matches!(self, ItemSource::Extension { .. })
    }

    /// Extension id if the source is [`ItemSource::Extension`], else
    /// `None`.
    pub fn extension_id(&self) -> Option<&str> {
        match self {
            ItemSource::Extension { id } => Some(id.as_str()),
            _ => None,
        }
    }
}
