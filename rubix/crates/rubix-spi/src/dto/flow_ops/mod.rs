//! flow_ops goal — REST DTOs + tool descriptors.
//!
//! Mirrors [`rubix-tools::flow_ops`] one-to-one: each verb has its
//! own file carrying request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).

pub mod deploy;
pub mod duplicate;
pub mod kinds;
pub mod lint;
pub mod list;
pub mod validate;
