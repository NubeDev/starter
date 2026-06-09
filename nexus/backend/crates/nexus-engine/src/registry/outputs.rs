//! Register nexus's custom output sinks with ArkFlow.

use arkflow_core::Error;

use crate::sink::{collector, postgres, sse};

/// Register nexus's custom outputs — the bounded `collector` for one-shot
/// queries, the `sse` broadcast sink for live streams, and the `postgres` sink
/// that lands ingestion-flow rows in a datasource DB. Called once from
/// [`super::register_all`].
pub fn register() -> Result<(), Error> {
    collector::init()?;
    sse::init()?;
    postgres::init()?;
    Ok(())
}
