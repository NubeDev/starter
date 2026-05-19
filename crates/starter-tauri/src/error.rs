//! Serde-friendly error wrapper for `#[tauri::command]` returns.
//!
//! Tauri requires command errors to be `serde::Serialize` so it can ship
//! them to the frontend. Domain errors usually aren't, so wrap them in
//! `CommandError` at the boundary:
//!
//! ```ignore
//! #[tauri::command]
//! async fn do_thing(state: State<'_, AppState>) -> CommandResult<Thing> {
//!     state.svc.do_thing().await.map_err(CommandError::from_display)
//! }
//! ```
//!
//! A blanket `From<E: Display>` would conflict with the standard
//! reflexive `From<T> for T`, so use `from_display` or `?` against the
//! specific `From` impls below.

use serde::Serialize;

/// Boundary error: any `Display` source collapses to a string the
/// frontend can render. Optional `code` lets callers distinguish kinds
/// without parsing messages.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl CommandError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Build from any `Display` source. Use with `.map_err(...)` since a
    /// blanket `From<E: Display>` impl is forbidden by coherence.
    pub fn from_display<E: std::fmt::Display>(e: E) -> Self {
        Self::new(e.to_string())
    }
}

impl From<String> for CommandError {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for CommandError {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<std::io::Error> for CommandError {
    fn from(e: std::io::Error) -> Self {
        Self::new(e.to_string()).with_code("io")
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
