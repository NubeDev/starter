//! Rubix-level error types. Composes starter-spi's `Error`.
//!
//! `Error::SkillForbidden` is a typed refusal carrying the active
//! skill id and the denied tool id, so the agent can self-correct on
//! the next turn. See
//! [docs/design/skills/](../../docs/design/skills/README.md).

use thiserror::Error;

/// Top-level rubix error type for tool and transport layers.
#[derive(Debug, Error)]
pub enum Error {
    /// The active skill's `allowed_tools` does not include the
    /// attempted tool. Surfaced as `agent.tool.error` with a
    /// localized MessageKey response. No silent drop, no auto
    /// skill-swap. See
    /// [docs/design/skills/](../../docs/design/skills/README.md).
    #[error("skill {skill} does not allow tool {tool}")]
    SkillForbidden { skill: String, tool: String },

    /// An upstream starter error.
    #[error(transparent)]
    Starter(#[from] starter_spi::Error),
}

/// Convenience alias used by every rubix-spi-typed function.
pub type Result<T> = std::result::Result<T, Error>;
