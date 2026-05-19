//! Sort-order primitives. List endpoints accept a `Sort` to choose
//! direction; the field is named by the endpoint's own schema.

mod direction;

pub use direction::Direction;
