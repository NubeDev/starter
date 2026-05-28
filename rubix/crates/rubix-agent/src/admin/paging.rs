//! Pagination — apply `cursor` + `limit` to a deterministically
//! ordered list of [`RegistryItem`]s.
//!
//! Every projector in this module sorts its output by `id`; this
//! function then slices "items strictly after the cursor id, up to
//! `limit` rows" and emits a `next_cursor` when more rows remain.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rubix_spi::dto::admin::RegistryItem;
use rubix_spi::starter::paging::{Cursor, Page};

/// Default page size when the caller does not pass `?limit=`.
pub const DEFAULT_PAGE_SIZE: usize = 50;

/// Hard ceiling on `?limit=`. Requests above this are rejected by
/// the transport layer.
pub const MAX_PAGE_SIZE: usize = 200;

/// Pagination failure surfaced as a 400 by the transport.
#[derive(Debug, thiserror::Error)]
pub enum PageError {
    /// `cursor` was syntactically malformed.
    #[error("invalid cursor")]
    InvalidCursor,
    /// `limit` was outside the accepted range.
    #[error("limit must be between 1 and {max}")]
    InvalidLimit {
        /// The clamp ceiling.
        max: usize,
    },
}

/// Decode an opaque cursor back into the id-after marker.
pub fn decode_cursor(cursor: &Cursor) -> Result<String, PageError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor.as_str())
        .map_err(|_| PageError::InvalidCursor)?;
    String::from_utf8(bytes).map_err(|_| PageError::InvalidCursor)
}

/// Encode an id as the next-cursor.
pub fn encode_cursor(id: &str) -> Cursor {
    Cursor::new(URL_SAFE_NO_PAD.encode(id.as_bytes()))
}

/// Resolve the effective page size from an optional caller value,
/// rejecting out-of-range values.
pub fn resolve_limit(requested: Option<usize>) -> Result<usize, PageError> {
    match requested {
        None => Ok(DEFAULT_PAGE_SIZE),
        Some(0) => Err(PageError::InvalidLimit {
            max: MAX_PAGE_SIZE,
        }),
        Some(n) if n > MAX_PAGE_SIZE => Err(PageError::InvalidLimit {
            max: MAX_PAGE_SIZE,
        }),
        Some(n) => Ok(n),
    }
}

/// Slice `items` (sorted by `id`) per `cursor` + `limit` and
/// return the page envelope. Items are consumed by value to avoid
/// cloning the full vector; callers that need to retain the input
/// should clone before invoking.
pub fn paginate(
    mut items: Vec<RegistryItem>,
    cursor: Option<&Cursor>,
    limit: usize,
) -> Result<Page<RegistryItem>, PageError> {
    // Ensure deterministic order independent of projection order.
    items.sort_by(|a, b| a.id.cmp(&b.id));

    if let Some(c) = cursor {
        let after = decode_cursor(c)?;
        items.retain(|item| item.id > after);
    }

    if items.len() > limit {
        items.truncate(limit + 1);
        let next = items.pop().unwrap();
        let cursor_id = items.last().expect("non-empty after truncate").id.clone();
        // The truncated `next` row is the one beyond the page;
        // discard it and emit the last in-page id as the cursor.
        let _ = next;
        Ok(Page::with_next(items, encode_cursor(&cursor_id)))
    } else {
        Ok(Page::final_page(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubix_spi::dto::admin::ItemSource;

    fn item(id: &str) -> RegistryItem {
        RegistryItem::new(id, ItemSource::Builtin)
    }

    #[test]
    fn paginate_emits_cursor_when_more_remain() {
        let page = paginate(
            vec![item("a"), item("b"), item("c"), item("d")],
            None,
            2,
        )
        .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].id, "a");
        assert_eq!(page.items[1].id, "b");
        assert!(page.next_cursor.is_some());
    }

    #[test]
    fn paginate_no_cursor_when_terminal() {
        let page = paginate(vec![item("a"), item("b")], None, 50).unwrap();
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn cursor_resumes_after_id() {
        let cursor = encode_cursor("b");
        let page = paginate(
            vec![item("a"), item("b"), item("c"), item("d")],
            Some(&cursor),
            50,
        )
        .unwrap();
        assert_eq!(page.items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), vec!["c", "d"]);
    }

    #[test]
    fn invalid_limit_rejected() {
        assert!(matches!(
            resolve_limit(Some(0)),
            Err(PageError::InvalidLimit { .. })
        ));
        assert!(matches!(
            resolve_limit(Some(MAX_PAGE_SIZE + 1)),
            Err(PageError::InvalidLimit { .. })
        ));
        assert_eq!(resolve_limit(None).unwrap(), DEFAULT_PAGE_SIZE);
        assert_eq!(resolve_limit(Some(10)).unwrap(), 10);
    }
}
