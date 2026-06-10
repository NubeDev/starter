//! The flow-builder node palette: a self-description of every node type the
//! native registry can construct.
//!
//! The engine builds nodes from a config `Value` via [`crate::native_registry`];
//! that registry knows how to *construct* a node but carries no description of
//! the config it accepts. The visual flow builder needs the opposite — to
//! *describe* each node so it can offer a palette and a schema-driven config
//! form — so [`descriptor`] hand-keeps a descriptor next to each built-in, kept
//! in lockstep with the builder registrations.

pub mod descriptor;

pub use descriptor::{describe, NodeCategory, NodeDescriptor};
