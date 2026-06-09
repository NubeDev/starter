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

// Vendored upstream code is kept byte-for-byte where possible to ease re-sync;
// a couple of now-unused imports remain in the trimmed output sinks.
#![allow(unused_imports)]

//! Connector-trimmed vendoring of arkflow-plugin.
//!
//! Upstream `arkflow-plugin` registers every connector unconditionally, dragging
//! in DuckDB (a ~15 GB static C++ build), librdkafka (needs system curl), PyO3,
//! and more — none of which the Nexus query/live seam uses. This copy keeps only
//! the pure-DataFusion pieces: the `memory`/`generate` inputs, the `sql` and
//! `json_to_arrow` processors, the `drop`/`stdout` outputs, and their support
//! modules. Re-sync against the pinned upstream rev on every ArkFlow bump.

pub mod buffer;
pub mod codec;
pub mod component;
pub mod context_pool;
pub mod expr;
pub mod input;
pub mod output;
pub mod processor;
pub mod temporary;
pub mod time;
pub mod udf;
