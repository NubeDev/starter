//! The runners that drive ArkFlow `Stream`s for the product's two shapes:
//! one-shot queries (here) and live subscriptions (added with the SSE seam).

pub mod cancel;
pub mod query;

pub use query::{QueryOutcome, QueryRunner};
