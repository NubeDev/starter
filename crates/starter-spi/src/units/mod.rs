//! Units of measurement — the wire surface (`Quantity`, `Unit`),
//! the registry trait (`UnitRegistry` + `StaticRegistry`), and the
//! canonical-storage normaliser ([`normalize_for_storage`]).
//!
//! Per R4 of `DOCS/user/scope/SCOPE.md`:
//!
//! > `starter-spi` owns the `Quantity` and `Unit` enums and the
//! > `UnitRegistry` trait + `StaticRegistry` impl. The enums are
//! > **closed** — extensions cannot add variants — because every wire
//! > identifier and every UI label must be known to the platform.
//! > […] Conversion factors are delegated to `uom` internally. The
//! > registry is the thin serialisable veneer; we never hand-write
//! > conversion math.
//!
//! Module layout follows the workspace "one responsibility per file"
//! rule (R1): each public type / function lives in its own file and
//! is re-exported here.

mod convert;
mod error;
mod quantity;
mod registry;
mod unit;

pub use convert::normalize_for_storage;
pub use error::UnitError;
pub use quantity::Quantity;
pub use registry::{QuantityDef, StaticRegistry, UnitRegistry};
pub use unit::Unit;

#[cfg(test)]
mod tests;
