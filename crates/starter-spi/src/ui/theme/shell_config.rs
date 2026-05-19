//! [`ShellConfig`] — branding sidecar shipped alongside the token
//! maps.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Non-token branding choices that travel with the theme.
///
/// `hide_features` carries consumer-defined string ids — starter
/// itself attaches no meaning to them; the consumer's shell layer
/// reads the list and hides the matching nav items.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ShellConfig {
    /// Display name shown in the top nav.
    #[serde(default)]
    pub nav_title: String,
    /// Consumer-defined feature ids the admin wants hidden.
    #[serde(default)]
    pub hide_features: Vec<String>,
}
