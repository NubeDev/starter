//! Rubix contracts hub.
//!
//! Wire types for the six rubix goals: REST DTOs (utoipa-decorated for
//! OpenAPI), MCP tool descriptors, and re-exports of starter-spi types
//! rubix consumers will want to name (`Id`, `Error`, `Principal`,
//! `Tool`, `Quantity`, etc.).
//!
//! Zero internal deps, zero runtime logic, zero HTTP, zero SQL. Every
//! tool DTO ships with a descriptor (purpose, when-to-use,
//! when-NOT-to-use, example, siblings). See
//! [docs/design/overview/](../../docs/design/overview/README.md) and
//! [docs/design/tools/](../../docs/design/tools/README.md).

pub use starter_spi as starter;

pub mod dashboard;
pub mod descriptor;
pub mod dto;
pub mod error;
pub mod events;
pub mod flow_def;
pub mod i18n;

pub use descriptor::ToolDescriptor;
pub use error::{Error, Result};
