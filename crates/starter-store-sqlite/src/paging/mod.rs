//! Cursor encoding/decoding for SQLite-backed list queries.

mod cursor_codec;

pub use cursor_codec::{decode, encode};
