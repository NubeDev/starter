//! Warehouse node kinds (W9). Each kind is a `NodeBehavior` impl
//! plus a `NodeDescriptor` constant. Consumers register the
//! descriptors via `starter_flow_nodes::node_registry`'s
//! `StaticNodeKindRegistry` builder; the warehouse crate exposes
//! [`descriptors`] as the canonical aggregate.
//!
//! Per W9 every node here is invoked through the flow engine —
//! `mart.define`, `mart.read`, etc., are flow runs, not direct
//! function calls. The REST surface forwards into the same
//! [`runtime::WarehouseRuntime`] the node bodies use, so the two
//! call sites stay byte-identical.

pub mod bulk_import;
pub mod cleaner_define;
pub mod cleaner_drop;
pub mod cleaner_promote;
pub mod curate_write;
pub mod mart_define;
pub mod mart_drop;
pub mod mart_promote;
pub mod mart_read;
pub mod runtime;
pub mod sandbox_define;
pub mod sandbox_drop;
pub mod sandbox_redefine;
pub mod tap_write;

use starter_flow_spi::node::NodeDescriptor;

/// Every warehouse descriptor in a stable order. Consumers fold
/// this into a `StaticNodeKindRegistry`.
pub fn descriptors() -> Vec<&'static NodeDescriptor> {
    vec![
        &tap_write::DESCRIPTOR,
        &curate_write::DESCRIPTOR,
        &bulk_import::DESCRIPTOR,
        &sandbox_define::DESCRIPTOR,
        &sandbox_redefine::DESCRIPTOR,
        &sandbox_drop::DESCRIPTOR,
        &cleaner_define::DESCRIPTOR,
        &cleaner_promote::DESCRIPTOR,
        &cleaner_drop::DESCRIPTOR,
        &mart_define::DESCRIPTOR,
        &mart_read::DESCRIPTOR,
        &mart_promote::DESCRIPTOR,
        &mart_drop::DESCRIPTOR,
    ]
}

pub use runtime::{ReadResult, RuntimeError, WarehouseRuntime};
