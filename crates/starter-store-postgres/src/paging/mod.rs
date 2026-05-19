//! Cursor codec for Postgres-backed list queries. Identical
//! encoding to the sqlite crate's so cursors are pool-portable.

mod cursor_codec;

pub use cursor_codec::{decode, encode};
