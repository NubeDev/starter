//! Errors the binder raises while rewriting a query.
//!
//! Every variant is the caller's fault (a bad macro, an unknown variable, a
//! forbidden host token) and maps to a 4xx — the binder never fails for an
//! internal reason. See docs/design/query/ for the macro grammar.

use starter_spi::Error;
use thiserror::Error as ThisError;

/// A failure to rewrite the input SQL into a bound query. Carrying the offending
/// token lets the editor point at it; mapping to [`starter_spi::Error::Invalid`]
/// keeps the HTTP surface a plain 4xx.
#[derive(Debug, Clone, PartialEq, Eq, ThisError)]
pub enum BindError {
    /// A `$__macro(...)` whose name is not one the engine knows.
    #[error("unknown macro: ${{__}}{0}")]
    UnknownMacro(String),

    /// A macro was used with the wrong number/shape of arguments.
    #[error("macro $__{macro_name} misused: {detail}")]
    MalformedMacro { macro_name: String, detail: String },

    /// A `$var` / `${var}` the request did not supply a value for.
    #[error("undefined variable: {0}")]
    UndefinedVariable(String),

    /// A `$param` (kind named parameter) the request did not supply.
    #[error("undefined parameter: {0}")]
    UndefinedParameter(String),

    /// An identifier (e.g. the column in `$__timeFilter(col)`) failed the strict
    /// allowlist. Identifiers are the only text ever inserted into SQL, so a
    /// rejection here is the tenant-isolation/injection guard firing.
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),

    /// A `$caller_tenant_id` / `$caller_user_id` appeared in caller-supplied
    /// input (a variable or param value). Host tokens are bound from the
    /// `Principal` and can never originate from the request — see WS-10.
    #[error("host token cannot be supplied by the caller: {0}")]
    HostTokenInInput(String),

    /// A macro needs context the request didn't carry (e.g. `$__timeFilter` with
    /// no time range, `$__interval` with no interval).
    #[error("macro $__{macro_name} needs {missing} but it was not provided")]
    MissingContext { macro_name: String, missing: String },

    /// An unterminated `${...}` / `$__macro(` in the input.
    #[error("unterminated token starting at byte {0}")]
    Unterminated(usize),
}

impl From<BindError> for Error {
    fn from(e: BindError) -> Self {
        Error::Invalid {
            message: e.to_string(),
        }
    }
}
