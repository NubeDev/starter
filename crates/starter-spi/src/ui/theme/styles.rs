//! [`ThemeStyles`] — the two token maps the editor writes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Light + dark CSS-custom-property maps.
///
/// Keys are the unprefixed token names (`"primary"`, not
/// `"--primary"`); the frontend stamps the `--` prefix at apply
/// time. Values are CSS strings — colours, lengths, font stacks —
/// whatever the corresponding token expects. See the token surface
/// table in `DOCS/frontend/theme/README.md` for the canonical
/// 38-key set.
///
/// `BTreeMap` rather than `HashMap`: serialised output is
/// deterministic, which keeps OpenAPI snapshots and integration
/// tests stable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ThemeStyles {
    /// Tokens applied when the resolved mode is `light`.
    #[serde(default)]
    pub light: BTreeMap<String, String>,
    /// Tokens applied when the resolved mode is `dark`.
    #[serde(default)]
    pub dark: BTreeMap<String, String>,
}
