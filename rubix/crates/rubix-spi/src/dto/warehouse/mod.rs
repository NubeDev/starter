//! warehouse goal — REST DTOs + tool descriptors.
//!
//! Mirrors [`rubix-tools::warehouse`] one-to-one: each verb has its
//! own file carrying request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).

pub mod mart_create;
pub mod mart_drop;
pub mod mart_list;
pub mod retention_set;
pub mod rule_list;
pub mod rule_write;
pub mod tables_list;
