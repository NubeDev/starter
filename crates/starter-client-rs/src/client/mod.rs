//! `Client` and its builder. Split out so an AI editing builder
//! options doesn't reload the runtime methods.

mod builder;
mod handle;

pub use builder::ClientBuilder;
pub use handle::Client;
