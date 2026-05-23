//! clickhouse goal — REST DTOs + tool descriptors.
//!
//! Mirrors [`rubix-tools::clickhouse`] one-to-one: each verb has its
//! own file carrying request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).

pub mod rule_write;
pub mod mart_create;
pub mod retention_set;
