//! In-memory collector output: capture a pipeline's rows for the UI.

mod sink;
mod store;

pub use sink::init;
pub use store::{open, take};
