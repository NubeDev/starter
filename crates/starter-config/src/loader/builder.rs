//! Layered loader. Caller picks the sources; merge order is fixed:
//! defaults (first call) → file → env → manual overrides (last call).
//! Later calls win.

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::error::ConfigError;

/// Builder over a `figment::Figment`. Domain-agnostic — the consumer
/// supplies the type parameter, this crate never sees their fields.
pub struct Loader {
    figment: Figment,
}

impl Loader {
    /// Start with a typed default value as the lowest-priority layer.
    pub fn with_defaults<T: Serialize>(defaults: T) -> Self {
        Self {
            figment: Figment::new().merge(Serialized::defaults(defaults)),
        }
    }

    /// Layer a TOML file on top. Missing files are not an error —
    /// the loader simply skips them. Parse errors are.
    pub fn with_file(mut self, path: impl Into<String>) -> Self {
        let path = path.into();
        self.figment = self.figment.merge(Toml::file(path));
        self
    }

    /// Layer environment variables on top, restricted to the given
    /// prefix (e.g. `"APP_"` matches `APP_SERVER__PORT`).
    ///
    /// Double-underscore is treated as nested key separator —
    /// `APP_SERVER__PORT` → `server.port`.
    pub fn with_env(mut self, prefix: &str) -> Self {
        self.figment = self.figment.merge(Env::prefixed(prefix).split("__"));
        self
    }

    /// Final consume — deserialize into the consumer's struct.
    pub fn load<T: DeserializeOwned>(self) -> Result<T, ConfigError> {
        self.figment.extract().map_err(ConfigError::Shape)
    }
}
