//! user goal — REST DTOs + tool descriptors.
//!
//! Mirrors [`rubix-tools::user`] one-to-one: each verb has its
//! own file carrying request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).

pub mod create;
pub mod disable;
pub mod list;
