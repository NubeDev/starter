//! Canonical OpenAPI document for the routes this crate ships.
//!
//! Consumers merge this into their own utoipa-derived document the
//! same way they handle `starter_auth_users::openapi::openapi()`.

use starter_spi::dto::Problem;
use starter_spi::ui::theme::{ShellConfig, ThemeDocument, ThemeSaveInput, ThemeStyles};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "starter-ui-theme",
        description = "Org-level theme persistence routes shipped by starter-ui-theme.",
        version = env!("CARGO_PKG_VERSION"),
    ),
    paths(
        crate::routes::get_theme,
        crate::routes::put_theme,
        crate::routes::get_logo,
        crate::routes::post_logo,
        crate::routes::delete_logo,
        crate::routes::get_favicon,
        crate::routes::post_favicon,
        crate::routes::delete_favicon,
    ),
    components(schemas(ThemeDocument, ThemeSaveInput, ThemeStyles, ShellConfig, Problem)),
    tags((name = "ui-theme", description = "Org-level theme persistence")),
)]
/// utoipa entry point holding the path + component derives.
pub struct UiThemeApi;

/// Build the canonical OpenAPI document for this crate's routes.
pub fn openapi() -> utoipa::openapi::OpenApi {
    UiThemeApi::openapi()
}
