//! Theme persistence contracts. Wire-shaped DTOs (matching the JSON
//! the frontend theme editor already speaks — see
//! `DOCS/frontend/theme/README.md`), the [`ThemeStore`] trait every
//! backend implements, and a [`validate_token_value`] helper every
//! transport calls before accepting a new style.
//!
//! One responsibility per file: each public item lives in its own
//! module, the barrel re-exports them.

mod document;
mod save_input;
mod shell_config;
mod store;
mod styles;
mod validator;

pub use document::ThemeDocument;
pub use save_input::ThemeSaveInput;
pub use shell_config::ShellConfig;
pub use store::ThemeStore;
pub use styles::ThemeStyles;
pub use validator::{validate_token_value, validate_save_input, TokenValueError};
