//! Cursor codec: encode/decode `(sort_value, last_id)` into a
//! `starter_spi::Cursor`. Opaque to clients; just bytes round-tripped.

use starter_spi::Cursor;

/// Encode a `(sort_key, id)` pair into an opaque cursor.
///
/// The encoding is base64(json([sort_key, id])). Clients never see
/// this — they round-trip the cursor string.
pub fn encode(sort_key: &str, id: &str) -> Cursor {
    // TODO(ap): pull in base64 / serde once the first store query
    // needs to issue a real cursor. Stubbed shape only.
    let raw = format!("{sort_key}|{id}");
    Cursor::new(raw)
}

/// Decode an opaque cursor back into `(sort_key, id)`.
///
/// Returns `None` for a cursor that wasn't produced by [`encode`].
pub fn decode(cursor: &Cursor) -> Option<(String, String)> {
    let raw = cursor.as_str();
    let (sort, id) = raw.split_once('|')?;
    Some((sort.to_string(), id.to_string()))
}
