//! Generic paged result.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::cursor::Cursor;

/// One page of `T`s. `next_cursor` is `Some` iff more pages exist.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[aliases(PageOfString = Page<String>)]
pub struct Page<T> {
    /// The items in this page, in sort order.
    pub items: Vec<T>,

    /// Cursor to pass back as `?cursor=...` for the next page.
    /// `None` means there are no more pages.
    pub next_cursor: Option<Cursor>,
}

impl<T> Page<T> {
    /// Build a terminal page (no further results).
    pub fn final_page(items: Vec<T>) -> Self {
        Self {
            items,
            next_cursor: None,
        }
    }

    /// Build a non-terminal page with a continuation cursor.
    pub fn with_next(items: Vec<T>, next: Cursor) -> Self {
        Self {
            items,
            next_cursor: Some(next),
        }
    }
}
