//! Signup subsystem: mode configuration, input validation, password
//! blocklist, and rate limiting.

pub mod blocklist;
pub mod mode;
pub mod rate_limit;
pub mod validate;

pub use mode::SignupMode;
pub use rate_limit::{MemoryRateLimiter, NoRateLimit, RateLimited, SignupRateLimiter};
pub use validate::{validate_signup_input, ValidationError};
