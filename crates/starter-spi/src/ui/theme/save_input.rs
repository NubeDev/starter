//! [`ThemeSaveInput`] — the body of a `PUT /api/v1/ui/theme`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{ShellConfig, ThemeStyles};

/// Editor → server save payload. Mirrors the shape the frontend
/// `httpThemeTransport.save({ theme_styles, shell })` already sends.
///
/// Asset URLs are *not* in this DTO — uploads are separate
/// `POST /api/v1/ui/theme/{logo,favicon}` endpoints so the JSON
/// surface stays plain JSON (no multipart juggling).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ThemeSaveInput {
    /// Light + dark token maps.
    pub theme_styles: ThemeStyles,
    /// Branding sidecar.
    pub shell: ShellConfig,
}
