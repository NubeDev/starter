//! Convenience `Result` alias bound to [`super::Error`].

/// `Result<T, starter_spi::Error>`. Use this everywhere in the
/// starter ecosystem so callers don't have to spell out the error.
pub type Result<T> = std::result::Result<T, super::Error>;
