//! Print-friendly HTML export.
//!
//! Produces a single self-contained `<!doctype html>` document with
//! a `@page` rule that honours the [`PageOptions`]. The intended
//! consumer hands the bytes to `window.print()` (or saves the file
//! and double-clicks it) — the browser handles pagination.
//!
//! **Payload schema:**
//!
//! ```json
//! { "title": "Q1 report", "body_html": "<h1>...</h1>" }
//! ```
//!
//! `body_html` is inserted verbatim. Callers are responsible for
//! making sure it is trusted: this crate does **not** sanitise.

use async_trait::async_trait;
use bytes::Bytes;

use crate::exporter::{ExportError, ExportFormat, ExportRequest, ExportResult, Exporter};
use crate::page::Orientation;

/// Wraps an HTML body in a print-ready document.
#[derive(Debug, Default, Clone, Copy)]
pub struct HtmlExporter;

#[async_trait]
impl Exporter for HtmlExporter {
    fn supports(&self, format: ExportFormat) -> bool {
        matches!(format, ExportFormat::Html)
    }

    async fn export(&self, request: ExportRequest) -> Result<ExportResult, ExportError> {
        if request.format != ExportFormat::Html {
            return Err(ExportError::UnsupportedFormat(request.format));
        }

        let title = request
            .payload
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Export");
        let body = request
            .payload
            .get("body_html")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ExportError::InvalidPayload("missing `body_html` string field".to_string())
            })?;

        let (w, h) = request.page.size.dimensions_mm();
        let orientation = match request.page.orientation {
            Orientation::Portrait => "portrait",
            Orientation::Landscape => "landscape",
        };
        let m = &request.page.margins;

        let html = format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title>\
             <style>@page {{ size: {w}mm {h}mm {orientation}; \
             margin: {top}mm {right}mm {bottom}mm {left}mm; }} \
             body {{ font-family: system-ui, sans-serif; }}</style></head>\
             <body>{body}</body></html>",
            title = html_escape(title),
            w = w,
            h = h,
            orientation = orientation,
            top = m.top_mm,
            right = m.right_mm,
            bottom = m.bottom_mm,
            left = m.left_mm,
            body = body,
        );

        Ok(ExportResult {
            format: ExportFormat::Html,
            bytes: Bytes::from(html),
            filename: request.filename.unwrap_or_else(|| "export".to_string()),
        })
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
