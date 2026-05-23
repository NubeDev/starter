//! system goal — REST DTOs + tool descriptors.
//!
//! Mirrors [`rubix-tools::system`] one-to-one: each verb has its
//! own file carrying request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).

pub mod disk;
pub mod db;
pub mod flow_errors;
pub mod alert_send;
