//! Admin introspection projections.
//!
//! This module contains the pure functions that walk the
//! rubix-agent's in-process registries (tools, node-kinds, rules,
//! warehouse templates and tables, skills, extensions) and emit
//! [`RegistryItem`](rubix_spi::dto::admin::RegistryItem) rows in
//! the wire envelope. The HTTP transport lives one layer up under
//! [`crate::routes::admin`] and consumes these functions exclusively.
//!
//! LAYER: domain (pure projection — no I/O). See
//! [docs/design/admin/](../../../docs/design/admin/README.md).

pub mod extensions;
pub mod nodes;
pub mod overview;
pub mod paging;
pub mod rules;
pub mod skills;
pub mod source;
pub mod state;
pub mod tables;
pub mod templates;
pub mod tools;

pub use extensions::extension_items;
pub use nodes::node_items;
pub use overview::overview;
pub use paging::paginate;
pub use rules::rule_items;
pub use skills::skill_items;
pub use source::item_source;
pub use state::AdminState;
pub use tables::table_items;
pub use templates::template_items;
pub use tools::tool_items;
