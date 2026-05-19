//! [`ThemeDocument`] — the full theme record the editor `GET`s and
//! the runtime bootstraps from.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{ShellConfig, ThemeStyles};

/// The full org-level theme: style maps, branding sidecar, and the
/// (optional) URLs of the uploaded logo / favicon.
///
/// Asset URLs are `None` until the consumer uploads each asset; the
/// frontend treats `None` as "fall back to your bundled defaults".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ThemeDocument {
    /// Light + dark token maps.
    pub theme_styles: ThemeStyles,
    /// Branding sidecar.
    pub shell: ShellConfig,
    /// Where the server serves the logo, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    /// Where the server serves the favicon, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon_url: Option<String>,
}
