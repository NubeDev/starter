//! Mirrors `starter_store_sqlite::paging::cursor_codec`. Encoding
//! is identical so a cursor produced by either store decodes on
//! either store.

use starter_spi::Cursor;

/// Encode a `(sort_key, id)` pair into an opaque cursor.
pub fn encode(sort_key: &str, id: &str) -> Cursor {
    Cursor::new(format!("{sort_key}|{id}"))
}

/// Decode an opaque cursor.
pub fn decode(cursor: &Cursor) -> Option<(String, String)> {
    let raw = cursor.as_str();
    let (sort, id) = raw.split_once('|')?;
    Some((sort.to_string(), id.to_string()))
}
