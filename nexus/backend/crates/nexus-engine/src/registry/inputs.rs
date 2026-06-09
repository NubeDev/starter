//! Register nexus's custom inputs with ArkFlow.

use arkflow_core::Error;

use crate::source::http_poll;

/// Register nexus's custom inputs — the `http_poll` source that drives light
/// ingestion flows. Called once from [`super::register_all`].
pub fn register() -> Result<(), Error> {
    http_poll::init()?;
    Ok(())
}
