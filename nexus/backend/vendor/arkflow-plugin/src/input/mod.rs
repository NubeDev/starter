/*
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        http://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

//! Input component module
//!
//! This is a connector-trimmed vendoring of arkflow-plugin: only the inputs the
//! Nexus query/live seam uses are kept, so the heavy native connectors (Kafka,
//! Pulsar, NATS, Redis, MQTT, Modbus, DuckDB-backed SQL) are not compiled in.
//! Restore a connector by copying its module back from the pinned upstream rev
//! and re-adding its dependency.

use arkflow_core::Error;

pub mod codec_helper;
pub mod generate;
pub mod memory;

pub fn init() -> Result<(), Error> {
    generate::init()?;
    memory::init()?;
    Ok(())
}
