//! dashboard goal — REST DTOs + tool descriptors.
//!
//! Mirrors [`rubix-tools::dashboard`] one-to-one: each verb has its
//! own file carrying request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).

pub mod create;
pub mod delete;
pub mod duplicate;
pub mod get;
pub mod list;
pub mod page_set;
pub mod update;
