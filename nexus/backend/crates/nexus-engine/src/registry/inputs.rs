//! Register nexus's custom inputs with ArkFlow.

use arkflow_core::Error;

use crate::source::{http_poll, simulator};

/// Register nexus's custom inputs — the `http_poll` source that drives light
/// ingestion flows, and the `simulator` source that emits synthetic device
/// telemetry for testing. Called once from [`super::register_all`].
pub fn register() -> Result<(), Error> {
    http_poll::init()?;
    simulator::init()?;
    Ok(())
}
