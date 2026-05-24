//! `rubix.analytics.report` — tool dispatch.
//!
//! Stitches a list of named analytics-query templates into a single
//! rendered artifact (`html` / `csv` / `json`), pushes the bytes
//! through `starter-export`, and persists the result through a
//! caller-supplied [`BlobStore`] (typically `starter-blob-fs`).
//!
//! Per-format payload contracts:
//!
//! * **html** — one `<h2>` + `<table>` block per query (columns
//!   ordered by the union of seen JSON keys, rows in template
//!   order). Wrapped by [`HtmlExporter`] with default `PageOptions`.
//! * **csv** — single CSV file. Headers are the union of column
//!   names across all queries; rows are concatenated in template
//!   order with missing cells emitted as empty strings.
//! * **json** — pretty-printed `{ "<query>": [ rows... ], ... }`.
//! * **pdf** — refused at run-time with
//!   `rubix.analytics.report.format_unsupported`. Server-side PDF
//!   rendering is deferred to the frontend export path; see
//!   the `starter-export` crate-level docs.
//!
//! Reversible — the verb mints exactly one blob per successful
//! call. The [`ReversibleTool`] adapter emits a
//! `kind = "rubix.analytics.report.blob"` `Op::Create` draft
//! carrying the minted locator; the matching
//! [`AnalyticsReportReversible`] inverse calls
//! [`BlobStore::delete`].

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use rubix_spi::dto::analytics::report::{
    AnalyticsReportRequest, AnalyticsReportResponse, ReportFormat,
};
use serde_json::{json, Map, Value};
use starter_export::{
    csv_backend::CsvExporter, html::HtmlExporter, json_backend::JsonExporter, ExportFormat,
    ExportRequest, Exporter,
};
use starter_spi::authz::ResourceRef;
use starter_spi::blob::{BlobKey, BlobRef, BlobRefInternal, BlobStore, PresignOp, PutOptions};
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_clickhouse::ChClient;
use starter_undo::ChangeDraft;
use uuid::Uuid;

use crate::analytics::query::{lookup_template, run_query};
use crate::undo::dispatch::ReversibleTool;

/// `Reversible::kind()` for the blob mint side-effect of
/// `rubix.analytics.report`. Public so binaries can register the
/// [`AnalyticsReportReversible`] inverse with their
/// `starter_undo::ReversibleRegistry`.
pub const REPORT_BLOB_KIND: &str = "rubix.analytics.report.blob";

/// Concrete [`Tool`] for `rubix.analytics.report`.
///
/// Holds the shared `ChClient` (for running each named query)
/// alongside a `BlobStore` trait object — the verb is
/// engine-agnostic; production wires this to `starter-blob-fs`,
/// tests wire it to a tempdir-backed `FsBlobStore`.
#[derive(Clone)]
pub struct AnalyticsReportTool {
    client: Arc<ChClient>,
    store: Arc<dyn BlobStore>,
    /// TTL handed to [`BlobStore::presign`] when minting the
    /// returned URL. A short window is fine — the agent forwards
    /// the URL straight to the caller; cold reads should re-presign.
    presign_ttl: Duration,
}

impl AnalyticsReportTool {
    /// Wrap the shared `ChClient` + `BlobStore`. Defaults
    /// `presign_ttl` to 15 minutes — the value the rubix agent
    /// uses in production. Override with [`Self::with_presign_ttl`]
    /// when the caller has a different requirement (e.g. emailed
    /// reports that need a longer window).
    pub fn new(client: Arc<ChClient>, store: Arc<dyn BlobStore>) -> Self {
        Self {
            client,
            store,
            presign_ttl: Duration::from_secs(15 * 60),
        }
    }

    /// Override the presign TTL. Chained from [`Self::new`].
    #[must_use]
    pub fn with_presign_ttl(mut self, ttl: Duration) -> Self {
        self.presign_ttl = ttl;
        self
    }
}

#[async_trait]
impl Tool for AnalyticsReportTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.analytics.report".to_owned(),
            description: rubix_spi::dto::analytics::report::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "template": { "type": "string", "minLength": 1 },
                    "queries":  {
                        "type": "array",
                        "items": { "type": "string", "minLength": 1 }
                    },
                    "format":   {
                        "type": "string",
                        "enum": ["html", "csv", "json", "pdf"]
                    }
                },
                "required": ["template", "format"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: AnalyticsReportRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("AnalyticsReportRequest: {e}"),
            })?;

        // pdf is the one format we name in the catalogue but refuse
        // at run-time. Surface the dedicated MessageKey so callers
        // do not have to parse a free-form error string.
        if req.format == ReportFormat::Pdf {
            let key = MessageKey::parse("rubix.analytics.report.format_unsupported")
                .expect("hard-coded key parses");
            return Err(Error::Invalid {
                message: format!("{}: format=pdf", key.as_str()),
            });
        }

        // Run every requested query through the same code path
        // `rubix.analytics.query` uses — same template lookup, same
        // CH bind, same row shape. Empty params: report queries are
        // deliberately self-contained (the templates close the
        // window themselves).
        let empty_params = std::collections::BTreeMap::new();
        let mut per_query: Vec<(String, Vec<Value>)> = Vec::with_capacity(req.queries.len());
        let mut total_rows: u64 = 0;
        for name in &req.queries {
            let sql = lookup_template(name)?;
            let rows = run_query(&self.client, sql, &empty_params).await?;
            total_rows += rows.len() as u64;
            per_query.push((name.clone(), rows));
        }

        // Render to bytes via starter-export.
        let bytes = render(&req.template, req.format, &per_query).await?;

        // Persist through the BlobStore. Key carries the template
        // label + a fresh UUID so two concurrent reports never
        // collide; the extension is informational only (B2 — the
        // locator is opaque to consumers).
        let ext = match req.format {
            ReportFormat::Html => "html",
            ReportFormat::Csv => "csv",
            ReportFormat::Json => "json",
            ReportFormat::Pdf => unreachable!("pdf rejected above"),
        };
        let locator = format!(
            "reports/{template}/{uuid}.{ext}",
            template = req.template,
            uuid = Uuid::new_v4().simple(),
        );
        let key = BlobKey::new(&locator).map_err(|e| Error::Invalid {
            message: format!("blob key {locator}: {e}"),
        })?;
        let blob_ref = self
            .store
            .put_bytes(
                &key,
                Bytes::from(bytes),
                PutOptions::with_content_type(content_type_for(req.format)),
            )
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;
        let byte_count = blob_ref.size();

        let presigned = self
            .store
            .presign(&blob_ref, PresignOp::Get, self.presign_ttl)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;

        let code = if total_rows == 0 {
            "rubix.analytics.report.empty"
        } else {
            "rubix.analytics.report.rendered"
        };
        let summary = Diagnostic::new(
            MessageKey::parse(code).expect("hard-coded key parses"),
        )
        .with_param("template", DiagnosticParam::String(req.template.clone()))
        .with_param("format", DiagnosticParam::String(format_label(req.format)))
        .with_param("bytes", DiagnosticParam::I64(byte_count as i64))
        .with_param("queries", DiagnosticParam::I64(req.queries.len() as i64));

        let response = AnalyticsReportResponse {
            summary,
            blob_id: blob_ref.opaque_locator().to_owned(),
            url: presigned.url,
            byte_count,
            format: req.format,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for AnalyticsReportTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: AnalyticsReportResponse = serde_json::from_value(output.clone()).ok()?;
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: REPORT_BLOB_KIND.into(),
                id: Some(resp.blob_id.clone()),
                owner: None,
                tenant: None,
            },
            op: Op::Create,
            before: None,
            after: Some(json!({
                "blob_id": resp.blob_id,
                "byte_count": resp.byte_count,
                "format": resp.format,
            })),
            resource_version: None,
            correlation: None,
        })
    }
}

/// [`Reversible`] sidecar for the `rubix.analytics.report.blob` kind.
/// The inverse of "we wrote a blob" is "delete that blob" — the
/// `Op::Create` undo path calls [`BlobStore::delete`]; `apply_forward`
/// refuses because re-rendering would mint a fresh blob under a
/// different locator (silent redo would orphan the changelog snapshot).
pub struct AnalyticsReportReversible {
    store: Arc<dyn BlobStore>,
}

impl AnalyticsReportReversible {
    /// Wrap the shared `BlobStore`.
    pub fn new(store: Arc<dyn BlobStore>) -> Self {
        Self { store }
    }

    fn blob_ref_for(&self, ch: &Change) -> Result<BlobRef> {
        let locator = ch
            .resource
            .id
            .clone()
            .ok_or_else(|| Error::Invalid {
                message: "AnalyticsReportReversible: Change::resource.id is None".to_owned(),
            })?;
        // We do not persist the BackendId or etag in the changelog
        // snapshot — the BlobStore only needs the locator to route a
        // delete. Mint a synthetic ref with placeholders.
        Ok(BlobRef::mint(
            self.store.backend_id().clone(),
            locator,
            starter_spi::blob::Etag::new(""),
            0,
        ))
    }
}

#[async_trait]
impl Reversible for AnalyticsReportReversible {
    fn kind(&self) -> &'static str {
        REPORT_BLOB_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        match ch.op {
            Op::Create => {
                let blob_ref = self.blob_ref_for(ch)?;
                self.store
                    .delete(&blob_ref)
                    .await
                    .map_err(|e| Error::Internal {
                        source: Box::new(e),
                    })
            }
            Op::Update | Op::Delete | Op::Custom(_) => Err(Error::Invalid {
                message: format!(
                    "AnalyticsReportReversible: unsupported op {:?} \
                     (report writes are Create-only)",
                    ch.op
                ),
            }),
        }
    }

    async fn apply_forward(&self, _ch: &Change) -> Result<()> {
        Err(Error::Invalid {
            message: "AnalyticsReportReversible: redo is not supported \
                      (re-running the report would mint a new blob)"
                .to_owned(),
        })
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: Value,
    ) -> Result<Vec<ResourceRef>> {
        Err(Error::Invalid {
            message: "rubix.analytics.report.blob does not support clone".to_owned(),
        })
    }
}

/// Map a [`ReportFormat`] to its IANA content type. Mirrors
/// [`ExportFormat::content_type`] but for the subset we expose.
fn content_type_for(format: ReportFormat) -> &'static str {
    match format {
        ReportFormat::Html => ExportFormat::Html.content_type(),
        ReportFormat::Csv => ExportFormat::Csv.content_type(),
        ReportFormat::Json => ExportFormat::Json.content_type(),
        ReportFormat::Pdf => ExportFormat::Pdf.content_type(),
    }
}

/// Lowercase label used inside [`Diagnostic`] params.
fn format_label(format: ReportFormat) -> String {
    match format {
        ReportFormat::Html => "html",
        ReportFormat::Csv => "csv",
        ReportFormat::Json => "json",
        ReportFormat::Pdf => "pdf",
    }
    .to_owned()
}

/// Render the rows for every query as the requested format. Returns
/// the encoded bytes; the caller persists them.
async fn render(
    template: &str,
    format: ReportFormat,
    per_query: &[(String, Vec<Value>)],
) -> Result<Vec<u8>> {
    let result = match format {
        ReportFormat::Html => {
            let payload = json!({
                "title":     template,
                "body_html": render_html_body(template, per_query),
            });
            HtmlExporter::default()
                .export(req(payload, ExportFormat::Html, template))
                .await
        }
        ReportFormat::Csv => {
            let (headers, rows) = csv_payload(per_query);
            let payload = json!({ "headers": headers, "rows": rows });
            CsvExporter::default()
                .export(req(payload, ExportFormat::Csv, template))
                .await
        }
        ReportFormat::Json => {
            let map: Map<String, Value> = per_query
                .iter()
                .map(|(name, rows)| (name.clone(), Value::Array(rows.clone())))
                .collect();
            JsonExporter::default()
                .export(req(Value::Object(map), ExportFormat::Json, template))
                .await
        }
        ReportFormat::Pdf => unreachable!("caller rejects pdf before reaching render"),
    };
    let result = result.map_err(|e| Error::Internal {
        source: Box::new(e),
    })?;
    Ok(result.bytes.to_vec())
}

fn req(payload: Value, format: ExportFormat, filename: &str) -> ExportRequest {
    ExportRequest {
        format,
        page: Default::default(),
        payload,
        filename: Some(filename.to_owned()),
    }
}

/// Build the HTML body: one `<h2>` + `<table>` per query, with the
/// union of column names as the table headers and missing cells
/// rendered as the empty string.
fn render_html_body(template: &str, per_query: &[(String, Vec<Value>)]) -> String {
    let mut out = String::new();
    out.push_str(&format!("<h1>{}</h1>", html_escape(template)));
    for (name, rows) in per_query {
        out.push_str(&format!("<h2>{}</h2>", html_escape(name)));
        if rows.is_empty() {
            out.push_str("<p><em>No data.</em></p>");
            continue;
        }
        let headers = union_headers(rows.iter());
        out.push_str("<table><thead><tr>");
        for h in &headers {
            out.push_str(&format!("<th>{}</th>", html_escape(h)));
        }
        out.push_str("</tr></thead><tbody>");
        for row in rows {
            out.push_str("<tr>");
            for h in &headers {
                let cell = row.get(h).map(cell_to_string).unwrap_or_default();
                out.push_str(&format!("<td>{}</td>", html_escape(&cell)));
            }
            out.push_str("</tr>");
        }
        out.push_str("</tbody></table>");
    }
    out
}

/// Build the CSV `{ headers, rows }` payload starter-export wants
/// (rows as arrays of scalars). Cells are stringified through
/// [`cell_to_string`] so the CSV backend's "no nested arrays /
/// objects" rule does not bite on numeric or boolean cells.
fn csv_payload(per_query: &[(String, Vec<Value>)]) -> (Vec<Value>, Vec<Value>) {
    let headers = union_headers(per_query.iter().flat_map(|(_, r)| r.iter()));
    let header_values: Vec<Value> =
        headers.iter().map(|h| Value::String(h.clone())).collect();
    let mut rows: Vec<Value> = Vec::new();
    for (_, query_rows) in per_query {
        for row in query_rows {
            let cells: Vec<Value> = headers
                .iter()
                .map(|h| {
                    row.get(h)
                        .map(|v| Value::String(cell_to_string(v)))
                        .unwrap_or_else(|| Value::String(String::new()))
                })
                .collect();
            rows.push(Value::Array(cells));
        }
    }
    (header_values, rows)
}

fn union_headers<'a>(rows: impl Iterator<Item = &'a Value>) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut ordered: Vec<String> = Vec::new();
    for row in rows {
        if let Some(obj) = row.as_object() {
            for k in obj.keys() {
                if seen.insert(k.clone()) {
                    ordered.push(k.clone());
                }
            }
        }
    }
    ordered
}

fn cell_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(_) | Value::Number(_) => v.to_string(),
        Value::Array(_) | Value::Object(_) => v.to_string(),
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn render_html_includes_one_table_per_query_with_union_headers() {
        let per_query = vec![(
            "q1".to_owned(),
            vec![json!({"a": 1, "b": "x"}), json!({"a": 2, "c": "y"})],
        )];
        let bytes = render("weekly", ReportFormat::Html, &per_query).await.unwrap();
        let html = String::from_utf8(bytes).unwrap();
        assert!(html.contains("<h2>q1</h2>"), "html: {html}");
        assert!(html.contains("<th>a</th>"), "html: {html}");
        assert!(html.contains("<th>b</th>"), "html: {html}");
        assert!(html.contains("<th>c</th>"), "html: {html}");
        assert!(html.contains("<td>1</td>"), "html: {html}");
    }

    #[tokio::test]
    async fn render_empty_html_says_no_data() {
        let per_query = vec![("q1".to_owned(), Vec::new())];
        let bytes = render("weekly", ReportFormat::Html, &per_query).await.unwrap();
        let html = String::from_utf8(bytes).unwrap();
        assert!(html.contains("No data"), "html: {html}");
    }

    #[tokio::test]
    async fn render_json_keys_by_query_name() {
        let per_query = vec![
            ("q1".to_owned(), vec![json!({"a": 1})]),
            ("q2".to_owned(), vec![json!({"b": 2})]),
        ];
        let bytes = render("weekly", ReportFormat::Json, &per_query).await.unwrap();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["q1"][0]["a"], 1);
        assert_eq!(v["q2"][0]["b"], 2);
    }

    #[tokio::test]
    async fn render_csv_concatenates_rows_under_union_headers() {
        let per_query = vec![
            ("q1".to_owned(), vec![json!({"a": 1, "b": "x"})]),
            ("q2".to_owned(), vec![json!({"a": 2, "c": "y"})]),
        ];
        let bytes = render("weekly", ReportFormat::Csv, &per_query).await.unwrap();
        let csv = String::from_utf8(bytes).unwrap();
        let mut lines = csv.lines();
        let header = lines.next().unwrap();
        assert!(header.contains("a"), "header: {header}");
        assert!(header.contains("b"), "header: {header}");
        assert!(header.contains("c"), "header: {header}");
        // Two data lines.
        assert_eq!(lines.count(), 2);
    }
}
