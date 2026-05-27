//! dataflow goal — REST DTOs + tool descriptors.
//!
//! Synthesis tools live here. Each verb has its own file carrying
//! request/response DTO structs and a static
//! [`ToolDescriptor`](crate::descriptor::ToolDescriptor).
//!
//! See `rubix/docs/sessions/data-flow/01-producer.md` for the
//! framework split: synthesis is a tool, delivery is a flow.

pub mod synth;
