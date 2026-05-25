//! flow_ops goal — REST DTOs + tool descriptors.
//!
//! Mirrors [`rubix-tools::flow_ops`] one-to-one: each verb has its
//! own file carrying request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).

pub mod deploy;
pub mod validate;
pub mod lint;
pub mod list;
pub mod kinds;
pub mod duplicate;
