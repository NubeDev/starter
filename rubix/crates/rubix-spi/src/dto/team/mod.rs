//! team goal — REST DTOs + tool descriptors.
//!
//! Mirrors [`rubix-tools::team`] one-to-one: each verb has its
//! own file carrying request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).

pub mod assign;
pub mod create;
pub mod delete;
pub mod unassign;
pub mod update;
