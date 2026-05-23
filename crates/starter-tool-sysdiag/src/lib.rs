//! # starter-tool-sysdiag
//!
//! Host-local system diagnostics. Each probe is a pure function that
//! reads the local filesystem / kernel state and returns a typed
//! result. Consumers wrap each probe in their own
//! [`Tool`](starter_spi::tool::Tool) implementation when they want to
//! expose it over MCP / REST.
//!
//! ## What this crate is, and is not
//!
//! - **Is.** Cross-platform local-host probes built on `sysinfo`.
//! - **Is not.** A Tool registry, a metrics exporter, or a thresholding
//!   layer. Severity decisions (e.g. "80% full triggers a warn key")
//!   belong in the consumer where the domain language lives.
//!
//! ## Why a separate crate
//!
//! Several starter consumers want the same probe surface — anything
//! with an operator UX, a maintenance CLI, or a "is the box healthy?"
//! tool. Pulling each probe out lets the heavy `sysinfo` dep stay off
//! the consumer's graph unless they actually need it.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cpu;
pub mod disk;
pub mod memory;

pub use cpu::{cpu_usage, CpuProbeError, CpuUsage};
pub use disk::{disk_usage, DiskProbeError, DiskUsage};
pub use memory::{memory_usage, MemoryProbeError, MemoryUsage};
