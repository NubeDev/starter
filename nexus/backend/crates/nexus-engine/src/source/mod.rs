//! Pipeline sources: the ArkFlow inputs nexus registers, and their native ports
//! onto the RW-01 [`crate::core::Source`] trait (additive while RW-02 runs; the
//! ArkFlow versions stay until RW-03 cuts the runners over).

pub mod generate;
pub mod http_poll;
pub mod http_poll_source;
pub mod interval;
pub mod memory;
pub mod sim;
pub mod simulator;
pub mod simulator_source;

pub use generate::GenerateSource;
pub use http_poll_source::HttpPollSource;
pub use memory::MemorySource;
pub use simulator_source::SimulatorSource;
