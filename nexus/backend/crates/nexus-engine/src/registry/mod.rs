//! One-time registration of every ArkFlow component builder.
//!
//! ArkFlow keeps global builder registries and errors if a type is registered
//! twice. Registration therefore runs exactly once per process behind a guard,
//! so constructing several runners (or running many tests in one binary) is
//! safe.

pub mod inputs;
pub mod outputs;

use std::sync::OnceLock;

/// Register the built-in ArkFlow input/output/processor/buffer builders plus
/// nexus's custom inputs (`http_poll`) and outputs (`collector`, `sse`,
/// `postgres`). Idempotent: the first call populates the registries, later calls
/// are no-ops and return the first call's result.
pub fn register_all() -> Result<(), String> {
    static DONE: OnceLock<Result<(), String>> = OnceLock::new();
    DONE.get_or_init(register_once).clone()
}

fn register_once() -> Result<(), String> {
    arkflow_plugin::input::init().map_err(|e| e.to_string())?;
    arkflow_plugin::output::init().map_err(|e| e.to_string())?;
    arkflow_plugin::processor::init().map_err(|e| e.to_string())?;
    arkflow_plugin::buffer::init().map_err(|e| e.to_string())?;
    arkflow_plugin::temporary::init().map_err(|e| e.to_string())?;
    arkflow_plugin::codec::init().map_err(|e| e.to_string())?;

    inputs::register().map_err(|e| e.to_string())?;
    outputs::register().map_err(|e| e.to_string())?;
    Ok(())
}
