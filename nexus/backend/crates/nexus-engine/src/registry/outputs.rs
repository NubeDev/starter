//! Register nexus's custom output sinks with ArkFlow.

use arkflow_core::Error;

use crate::sink::collector;

/// Register the `collector` output (and, as it lands, `sse`). Called once from
/// [`super::register_all`].
pub fn register() -> Result<(), Error> {
    collector::init()?;
    Ok(())
}
