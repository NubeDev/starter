//! `rubix.analytics.report` — request/response DTOs and tool descriptor.
//!
//! Stitches one or more named [`AnalyticsQueryRequest`](super::query::AnalyticsQueryRequest)
//! templates into a single rendered artifact (`html` / `csv` / `json`),
//! pushes the bytes through `starter-export`, and persists them via a
//! caller-supplied `BlobStore` (typically `starter-blob-fs`). The
//! response carries the minted blob's opaque locator, a presigned URL,
//! the byte count and the echoed format.
//!
//! See [docs/design/analytics/](../../../../docs/design/analytics/README.md)
//! and [docs/design/reports/](../../../../docs/design/reports/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.analytics.report`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsReportRequest {
    /// Report template label — used as the blob-key prefix and the
    /// suggested filename stem. Free-form, but conventionally a
    /// short kebab-case identifier (e.g. `weekly-ops`).
    pub template: String,
    /// Named analytics-query templates to run. Each name must
    /// resolve through the same closed catalogue as
    /// `rubix.analytics.query` — unknown names yield
    /// `rubix.analytics.query.unknown_template`.
    #[serde(default)]
    pub queries: Vec<String>,
    /// Rendered output format. `pdf` always errors with
    /// `rubix.analytics.report.format_unsupported`: server-side PDF
    /// rendering is deferred to the frontend export path.
    #[serde(default = "default_format")]
    pub format: ReportFormat,
}

fn default_format() -> ReportFormat {
    ReportFormat::Html
}

/// Renderable output formats for a report. Mirrors the subset of
/// `starter_export::ExportFormat` we expose; `pdf` is intentionally
/// listed but rejected at run-time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    /// Self-contained HTML document with one `<table>` per query.
    Html,
    /// CSV (headers + rows). When more than one query is supplied,
    /// rows from every query are concatenated under the union of
    /// their column names.
    Csv,
    /// Pretty-printed JSON object keyed by query name.
    Json,
    /// Reserved — always errors with
    /// `rubix.analytics.report.format_unsupported`.
    Pdf,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyticsReportResponse {
    /// Outcome — either `rubix.analytics.report.rendered` (data
    /// present) or `rubix.analytics.report.empty` (every query
    /// returned zero rows; the blob is still written for an audit
    /// trail).
    pub summary: Diagnostic,
    /// Opaque blob locator — the string the originating `BlobStore`
    /// minted. Round-trips back into a later `BlobStore::get` via
    /// `BlobRefInternal::mint`.
    pub blob_id: String,
    /// Presigned read URL for the rendered artifact.
    pub url: String,
    /// Encoded payload size in bytes.
    pub byte_count: u64,
    /// Echoed format.
    pub format: ReportFormat,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "analytics.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Render one or more named analytics queries into a single HTML/CSV/JSON artifact and persist it to the blob store.",
    when_to_use: concat!(
        "Use when the agent needs a downloadable, shareable report ",
        "stitched from several named templates (e.g. the weekly ",
        "operations digest) and the caller will surface a link, ",
        "attach it to an email, or hand the bytes to a scheduled flow."
    ),
    when_not_to_use: concat!(
        "Do not use for ad-hoc single-query lookups (call ",
        "rubix.analytics.query — it returns rows in-process). Do not ",
        "use for server-side PDF: the `pdf` format always errors with ",
        "rubix.analytics.report.format_unsupported; render PDFs on the ",
        "frontend via @nube/starter-ui-export."
    ),
    example: concat!(
        "Input:  { \"template\": \"weekly-ops\", \"queries\": ",
        "[\"disk_history_weekly\"], \"format\": \"html\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.analytics.report.rendered\" }, ",
        "\"blob_id\": \"reports/weekly-ops/01J...html\", \"url\": \"file://?token=...\", ",
        "\"byte_count\": 1843, \"format\": \"html\" }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.analytics.query",
            wins_when: "the caller only needs in-memory rows for one named template, not a rendered artifact.",
        },
    ],
};
