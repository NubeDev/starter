//! `?limit=` + `?cursor=` query extractor shared by every admin
//! list endpoint. Keeps the per-route handlers a one-liner —
//! they just call [`paginate`](crate::admin::paginate) with what
//! this extractor returns.

use rubix_spi::starter::paging::Cursor;
use serde::Deserialize;

use crate::admin::paging::{resolve_limit, PageError};

/// Raw query string carried by every `GET /admin/registry*` route.
#[derive(Debug, Default, Deserialize)]
pub(super) struct ListQuery {
    /// Optional page-size override. Defaults to
    /// [`crate::admin::paging::DEFAULT_PAGE_SIZE`].
    #[serde(default)]
    pub limit: Option<usize>,
    /// Opaque continuation cursor returned by a previous response.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Comma-separated list of kinds to include in
    /// `/admin/registry`. Ignored on per-kind sugar routes.
    #[serde(default)]
    pub kinds: Option<String>,
    /// Source filter: `builtin`, `starter`, or `extension:<id>`.
    #[serde(default)]
    pub source: Option<String>,
}

/// Decoded form of [`ListQuery`].
pub(super) struct DecodedQuery {
    pub limit: usize,
    pub cursor: Option<Cursor>,
    pub kinds: Option<Vec<String>>,
    pub source: Option<SourceFilter>,
}

/// Parsed `?source=` filter.
pub(super) enum SourceFilter {
    Builtin,
    Starter,
    Extension(String),
}

impl SourceFilter {
    /// `true` when the supplied
    /// [`ItemSource`](rubix_spi::dto::admin::ItemSource) matches
    /// this filter.
    pub fn matches(&self, source: &rubix_spi::dto::admin::ItemSource) -> bool {
        use rubix_spi::dto::admin::ItemSource;
        match (self, source) {
            (SourceFilter::Builtin, ItemSource::Builtin) => true,
            (SourceFilter::Starter, ItemSource::Starter) => true,
            (SourceFilter::Extension(want), ItemSource::Extension { id }) => want == id,
            _ => false,
        }
    }
}

impl ListQuery {
    /// Decode into a typed shape, surfacing query errors as
    /// [`PageError`] so the handler can map them onto a single
    /// 400 path.
    pub(super) fn decode(self) -> Result<DecodedQuery, PageError> {
        let limit = resolve_limit(self.limit)?;
        let cursor = self.cursor.map(Cursor::new);
        let kinds = self.kinds.map(|s| {
            s.split(',')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        });
        let source = match self.source.as_deref() {
            None => None,
            Some("builtin") => Some(SourceFilter::Builtin),
            Some("starter") => Some(SourceFilter::Starter),
            Some(s) if s.starts_with("extension:") => Some(SourceFilter::Extension(
                s.trim_start_matches("extension:").to_owned(),
            )),
            // Unknown source token — surface as invalid cursor
            // rather than inventing a new error variant. The 400
            // body will say which token failed.
            Some(_) => return Err(PageError::InvalidCursor),
        };
        Ok(DecodedQuery {
            limit,
            cursor,
            kinds,
            source,
        })
    }
}
