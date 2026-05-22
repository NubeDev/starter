//! Pretty-printed JSON export.
//!
//! The payload is serialised verbatim with two-space indentation.
//! Useful as a uniform "download as file" target for arbitrary
//! data the consumer already has shaped server-side.

use async_trait::async_trait;
use bytes::Bytes;

use crate::exporter::{ExportError, ExportFormat, ExportRequest, ExportResult, Exporter};

/// Pretty-printed JSON exporter.
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonExporter;

#[async_trait]
impl Exporter for JsonExporter {
    fn supports(&self, format: ExportFormat) -> bool {
        matches!(format, ExportFormat::Json)
    }

    async fn export(&self, request: ExportRequest) -> Result<ExportResult, ExportError> {
        if request.format != ExportFormat::Json {
            return Err(ExportError::UnsupportedFormat(request.format));
        }

        let bytes = serde_json::to_vec_pretty(&request.payload)
            .map_err(|e| ExportError::Backend(e.to_string()))?;

        Ok(ExportResult {
            format: ExportFormat::Json,
            bytes: Bytes::from(bytes),
            filename: request.filename.unwrap_or_else(|| "export".to_string()),
        })
    }
}
