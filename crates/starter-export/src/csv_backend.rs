//! CSV export backed by the `csv` crate.
//!
//! **Payload schema:**
//!
//! ```json
//! {
//!   "headers": ["col1", "col2"],
//!   "rows": [["a", "b"], ["c", "d"]]
//! }
//! ```
//!
//! `headers` is optional; `rows` must be a JSON array of arrays of
//! values. Non-string scalar values are stringified with
//! `serde_json::to_string`; objects / arrays nested inside a cell are
//! rejected with [`ExportError::InvalidPayload`].

use async_trait::async_trait;
use bytes::Bytes;
use serde_json::Value;

use crate::exporter::{ExportError, ExportFormat, ExportRequest, ExportResult, Exporter};

/// Renders tabular data as CSV.
#[derive(Debug, Default, Clone, Copy)]
pub struct CsvExporter;

#[async_trait]
impl Exporter for CsvExporter {
    fn supports(&self, format: ExportFormat) -> bool {
        matches!(format, ExportFormat::Csv)
    }

    async fn export(&self, request: ExportRequest) -> Result<ExportResult, ExportError> {
        if request.format != ExportFormat::Csv {
            return Err(ExportError::UnsupportedFormat(request.format));
        }

        let mut writer = csv::Writer::from_writer(vec![]);

        if let Some(headers) = request.payload.get("headers").and_then(|v| v.as_array()) {
            let row: Vec<String> = headers
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| v.to_string())
                })
                .collect();
            writer
                .write_record(row)
                .map_err(|e| ExportError::Backend(e.to_string()))?;
        }

        let rows = request
            .payload
            .get("rows")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ExportError::InvalidPayload("missing `rows` array of arrays".to_string())
            })?;

        for (idx, row) in rows.iter().enumerate() {
            let cells = row
                .as_array()
                .ok_or_else(|| ExportError::InvalidPayload(format!("row {idx} is not an array")))?;
            let record: Result<Vec<String>, ExportError> = cells
                .iter()
                .map(|cell| match cell {
                    Value::Null => Ok(String::new()),
                    Value::String(s) => Ok(s.clone()),
                    Value::Bool(_) | Value::Number(_) => Ok(cell.to_string()),
                    Value::Array(_) | Value::Object(_) => Err(ExportError::InvalidPayload(
                        format!("row {idx}: nested array/object in cell"),
                    )),
                })
                .collect();
            writer
                .write_record(record?)
                .map_err(|e| ExportError::Backend(e.to_string()))?;
        }

        let bytes = writer
            .into_inner()
            .map_err(|e| ExportError::Backend(e.to_string()))?;

        Ok(ExportResult {
            format: ExportFormat::Csv,
            bytes: Bytes::from(bytes),
            filename: request.filename.unwrap_or_else(|| "export".to_string()),
        })
    }
}
