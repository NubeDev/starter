//! Cursor-based paging primitives. Used by every list endpoint in
//! the starter ecosystem so clients see one consistent shape.

mod cursor;
mod page;

pub use cursor::Cursor;
pub use page::Page;
