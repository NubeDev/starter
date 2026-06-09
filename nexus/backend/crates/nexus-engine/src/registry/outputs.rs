//! Register nexus's custom output sinks with ArkFlow.

use arkflow_core::Error;

use crate::sink::{collector, sse};

/// Register nexus's custom outputs — the bounded `collector` for one-shot
/// queries and the `sse` broadcast sink for live streams. Called once from
/// [`super::register_all`].
pub fn register() -> Result<(), Error> {
    collector::init()?;
    sse::init()?;
    Ok(())
}
