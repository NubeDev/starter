//! Cursor codec: encode/decode `(sort_value, id)` into a
//! `starter_spi::Cursor`. Opaque to clients; just bytes round-tripped.
//!
//! Wire format: `base64url_nopad(version_byte || json([sort, id]))`.
//! The leading version byte lets us evolve the inner representation
//! without breaking old clients — bump `CURSOR_VERSION` and add a
//! match arm in [`decode`].

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use starter_spi::Cursor;

/// Current cursor format version. Increment when the inner shape
/// changes; keep the old branch in [`decode`] for one release.
pub const CURSOR_VERSION: u8 = 1;

/// Encode a `(sort_key, id)` pair into an opaque cursor.
pub fn encode(sort_key: &str, id: &str) -> Cursor {
    let payload =
        serde_json::to_vec(&(sort_key, id)).expect("serializing two &str to JSON cannot fail");
    let mut buf = Vec::with_capacity(1 + payload.len());
    buf.push(CURSOR_VERSION);
    buf.extend_from_slice(&payload);
    Cursor::new(URL_SAFE_NO_PAD.encode(&buf))
}

/// Decode an opaque cursor back into `(sort_key, id)`.
///
/// Returns `None` for cursors that weren't produced by [`encode`] at
/// a recognised version.
pub fn decode(cursor: &Cursor) -> Option<(String, String)> {
    let bytes = URL_SAFE_NO_PAD.decode(cursor.as_str()).ok()?;
    let (version, payload) = bytes.split_first()?;
    match *version {
        CURSOR_VERSION => {
            let (sort, id): (String, String) = serde_json::from_slice(payload).ok()?;
            Some((sort, id))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let c = encode("created_at", "abc-123");
        assert_eq!(decode(&c), Some(("created_at".into(), "abc-123".into())));
    }

    #[test]
    fn round_trip_with_unicode_and_separators() {
        let c = encode("name|with|pipes", "id/with/slashes 🚀");
        assert_eq!(
            decode(&c),
            Some(("name|with|pipes".into(), "id/with/slashes 🚀".into()))
        );
    }

    #[test]
    fn garbage_decodes_to_none() {
        assert_eq!(decode(&Cursor::new("not base64!")), None);
    }

    #[test]
    fn unknown_version_decodes_to_none() {
        let bytes = [&[99u8][..], b"[\"a\",\"b\"]"].concat();
        let c = Cursor::new(URL_SAFE_NO_PAD.encode(&bytes));
        assert_eq!(decode(&c), None);
    }
}
