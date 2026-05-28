//! tenant goal — REST DTOs + tool descriptors.
//!
//! Mirrors [`rubix-tools::tenant`] one-to-one: each verb has its
//! own file carrying request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).

pub mod create;
pub mod delete;
pub mod list;
pub mod update;
