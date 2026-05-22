//! Pure-Rust PDF backend via [`printpdf`].
//!
//! Good for: tabular reports, invoices, structured text documents
//! that fit a "title + sections of paragraphs" layout.
//!
//! Not good for: pixel-perfect reproductions of a web page, charts,
//! custom typography, CSS-driven layout. For those, generate the
//! PDF on the frontend with `@nube/starter-ui-export` and either let
//! the browser save it locally or `POST` the bytes through your own
//! handler — the [`Exporter`] trait does not care where bytes come
//! from.
//!
//! **Payload schema:**
//!
//! ```json
//! {
//!   "title": "Q1 report",
//!   "sections": [
//!     { "heading": "Summary", "body": "Lorem ipsum..." },
//!     { "heading": "Details", "body": "More text..." }
//!   ]
//! }
//! ```

use async_trait::async_trait;
use bytes::Bytes;
use printpdf::{BuiltinFont, Mm, PdfDocument};

use crate::exporter::{ExportError, ExportFormat, ExportRequest, ExportResult, Exporter};

/// PDF backend built on the pure-Rust `printpdf` crate.
///
/// Single-pass renderer: text that overflows the bottom margin is
/// truncated. For multi-page flowing layouts use the frontend path.
#[derive(Debug, Default, Clone, Copy)]
pub struct PrintpdfExporter;

#[async_trait]
impl Exporter for PrintpdfExporter {
    fn supports(&self, format: ExportFormat) -> bool {
        matches!(format, ExportFormat::Pdf)
    }

    async fn export(&self, request: ExportRequest) -> Result<ExportResult, ExportError> {
        if request.format != ExportFormat::Pdf {
            return Err(ExportError::UnsupportedFormat(request.format));
        }

        let title = request
            .payload
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Export")
            .to_string();

        let sections: Vec<Section> = request
            .payload
            .get("sections")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(Section::from_value).collect())
            .unwrap_or_default();

        let (page_w, page_h) = request.page.dimensions_mm();
        let margins = request.page.margins;

        let (doc, page1, layer1) = PdfDocument::new(&title, Mm(page_w), Mm(page_h), "Layer 1");
        let current_layer = doc.get_page(page1).get_layer(layer1);

        let bold = doc
            .add_builtin_font(BuiltinFont::HelveticaBold)
            .map_err(|e| ExportError::Backend(e.to_string()))?;
        let regular = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| ExportError::Backend(e.to_string()))?;

        // Cursor in mm, measured from the bottom-left origin printpdf
        // uses. We render from the top down, so start near the top.
        let mut cursor_y = page_h - margins.top_mm;
        let left = margins.left_mm;
        let bottom = margins.bottom_mm;

        // Title.
        current_layer.use_text(&title, 22.0, Mm(left), Mm(cursor_y), &bold);
        cursor_y -= 12.0;

        for section in &sections {
            if cursor_y < bottom + 10.0 {
                break;
            }
            current_layer.use_text(&section.heading, 14.0, Mm(left), Mm(cursor_y), &bold);
            cursor_y -= 7.0;

            for line in section.body.lines() {
                if cursor_y < bottom {
                    break;
                }
                current_layer.use_text(line, 11.0, Mm(left), Mm(cursor_y), &regular);
                cursor_y -= 5.0;
            }
            cursor_y -= 4.0;
        }

        let bytes = doc
            .save_to_bytes()
            .map_err(|e| ExportError::Backend(e.to_string()))?;

        Ok(ExportResult {
            format: ExportFormat::Pdf,
            bytes: Bytes::from(bytes),
            filename: request.filename.unwrap_or_else(|| "export".to_string()),
        })
    }
}

struct Section {
    heading: String,
    body: String,
}

impl Section {
    fn from_value(v: &serde_json::Value) -> Self {
        Self {
            heading: v
                .get("heading")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            body: v
                .get("body")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        }
    }
}
