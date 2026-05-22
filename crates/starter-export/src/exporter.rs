//! The [`Exporter`] trait and the request / result / error types
//! every backend speaks. Transport-agnostic by design (SCOPE.md R3):
//! REST, CLI, MCP and gRPC all funnel through the same trait.

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::page::PageOptions;

#[cfg(feature = "axum-router")]
use utoipa::ToSchema;

/// Output format the consumer is asking for.
///
/// Backends advertise which formats they support via
/// [`Exporter::supports`]; the router uses that to refuse early with
/// [`ExportError::UnsupportedFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "axum-router", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Portable Document Format.
    Pdf,
    /// Self-contained HTML document.
    Html,
    /// Comma-separated values.
    Csv,
    /// Pretty-printed JSON.
    Json,
    /// Markdown text.
    Markdown,
}

impl ExportFormat {
    /// IANA media type, suitable for a `Content-Type` header.
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Html => "text/html; charset=utf-8",
            Self::Csv => "text/csv; charset=utf-8",
            Self::Json => "application/json",
            Self::Markdown => "text/markdown; charset=utf-8",
        }
    }

    /// Conventional filename extension (no leading dot).
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Html => "html",
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Markdown => "md",
        }
    }
}

/// What the consumer wants exported.
///
/// `payload` is intentionally opaque (`serde_json::Value`): each
/// backend interprets it according to its own contract. The
/// [`pdf::PrintpdfExporter`](crate::pdf::PrintpdfExporter), for
/// example, expects `{ "title": "...", "sections": [...] }`; the CSV
/// backend expects `{ "headers": [...], "rows": [[...]] }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "axum-router", derive(ToSchema))]
pub struct ExportRequest {
    /// Desired output format.
    pub format: ExportFormat,
    /// Page-level options (size / orientation / margins). Ignored by
    /// backends for which they don't apply (CSV, JSON).
    #[serde(default)]
    pub page: PageOptions,
    /// Backend-specific document body. See each backend's docs for
    /// the schema it expects.
    #[cfg_attr(feature = "axum-router", schema(value_type = Object))]
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Suggested filename stem (no extension). Used by HTTP handlers
    /// to set `Content-Disposition`. Defaults to `"export"`.
    #[serde(default)]
    pub filename: Option<String>,
}

/// Raw bytes plus the metadata a transport needs to deliver them.
#[derive(Debug, Clone)]
pub struct ExportResult {
    /// The format these bytes are in (echoed from the request).
    pub format: ExportFormat,
    /// Encoded bytes. Owned `Bytes` so cheap to hand to axum.
    pub bytes: Bytes,
    /// Filename stem the transport should suggest to the client.
    pub filename: String,
}

impl ExportResult {
    /// Suggested filename with the right extension.
    pub fn full_filename(&self) -> String {
        format!("{}.{}", self.filename, self.format.extension())
    }
}

/// Errors any backend can raise. Mapped to HTTP by [`routes`] when
/// the `axum-router` feature is on.
#[derive(Debug, Error)]
pub enum ExportError {
    /// The backend recognised the request but the format isn't
    /// implemented by this binary. Typically a missing crate feature.
    #[error("unsupported export format: {0:?}")]
    UnsupportedFormat(ExportFormat),

    /// `payload` didn't match the schema this backend expects.
    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    /// Anything else (I/O, library bug, etc.).
    #[error("export failed: {0}")]
    Backend(String),
}

#[cfg(feature = "axum-router")]
impl From<ExportError> for starter_spi::Error {
    fn from(value: ExportError) -> Self {
        use starter_spi::Error as SpiErr;
        match value {
            ExportError::UnsupportedFormat(_) => SpiErr::Invalid {
                message: value.to_string(),
            },
            ExportError::InvalidPayload(msg) => SpiErr::Invalid { message: msg },
            ExportError::Backend(_) => SpiErr::Internal {
                source: Box::new(value),
            },
        }
    }
}

/// Anything that turns an [`ExportRequest`] into bytes.
///
/// Implementors must be `Send + Sync` so the router can keep a
/// `dyn Exporter` behind an `Arc`.
#[async_trait]
pub trait Exporter: Send + Sync {
    /// Returns `true` if this backend can produce `format`. The
    /// default implementation returns `false`; concrete backends
    /// override with a literal match.
    fn supports(&self, format: ExportFormat) -> bool {
        let _ = format;
        false
    }

    /// Produce the export bytes.
    async fn export(&self, request: ExportRequest) -> Result<ExportResult, ExportError>;
}
