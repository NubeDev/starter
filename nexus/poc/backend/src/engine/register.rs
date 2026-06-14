//! Register every ArkFlow component builder exactly once at startup.
//!
//! The engine keeps global builder registries; registering a type twice errors.
//! Call [`register_all`] a single time before serving requests.

use arkflow_core::Error;

use super::collector;

/// Populate the built-in input/output/processor/buffer registries plus our
/// custom `collector` output.
pub fn register_all() -> Result<(), Error> {
    arkflow_plugin::input::init()?;
    arkflow_plugin::output::init()?;
    arkflow_plugin::processor::init()?;
    arkflow_plugin::buffer::init()?;
    arkflow_plugin::temporary::init()?;
    arkflow_plugin::codec::init()?;

    collector::init()?;
    Ok(())
}
