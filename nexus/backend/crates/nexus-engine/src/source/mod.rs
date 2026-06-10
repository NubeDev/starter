//! Pipeline sources over the [`crate::core::Source`] trait: a finite `memory`
//! replay and `generate` ticker for tests and the live seam, an `http_poll`
//! input for light ingestion flows, and a `simulator` of synthetic device
//! telemetry. `interval` and `sim` are the shared cadence parser and row builder
//! the polling and simulator sources compose.

pub mod generate;
pub mod http_poll_source;
pub mod interval;
pub mod memory;
pub mod sim;
pub mod simulator_source;

pub use generate::GenerateSource;
pub use http_poll_source::HttpPollSource;
pub use memory::MemorySource;
pub use simulator_source::SimulatorSource;
