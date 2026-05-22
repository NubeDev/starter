//! # starter-export
//!
//! Reusable, transport-agnostic export pipeline.
//!
//! The crate is intentionally split into two layers (SCOPE.md R1):
//!
//! * [`page`] — the page-options vocabulary ([`Orientation`],
//!   [`PageSize`], [`Margins`], [`PageOptions`]). Always available.
//! * [`exporter`] — the [`Exporter`] trait + [`ExportRequest`] /
//!   [`ExportResult`] / [`ExportError`] types. Always available.
//!
//! Concrete backends are feature-gated per SCOPE.md R5:
//!
//! | feature       | backend                                            |
//! |---------------|----------------------------------------------------|
//! | `pdf`         | [`pdf::PrintpdfExporter`] — pure-Rust PDF reports |
//! | `html`        | [`html::HtmlExporter`] — print-friendly HTML       |
//! | `csv`         | [`csv_backend::CsvExporter`] — CSV rows            |
//! | `json`        | [`json_backend::JsonExporter`] — pretty JSON       |
//! | `axum-router` | [`routes::export_router`] — `POST /v1/export`      |
//!
//! ## Rust PDF vs. browser PDF — be honest about which is "amazing"
//!
//! The bundled [`pdf::PrintpdfExporter`] is great for tables, headers,
//! invoices and other structured reports that you can describe as
//! rows + text. It is *not* a web rendering engine: it does not lay
//! out CSS, web fonts, charts, flexbox or arbitrary HTML. If the
//! consumer needs a "looks like the website, but as a PDF" output,
//! generate the PDF on the **frontend** with `@nube/starter-ui-export`
//! (driven by the browser print pipeline or `html2canvas` + `jspdf`)
//! and either let the user save it locally or `POST` the bytes back
//! to the server. The [`Exporter`] trait does not care where the
//! bytes came from.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod exporter;
pub mod page;

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "html")]
pub mod html;

#[cfg(feature = "csv")]
pub mod csv_backend;

#[cfg(feature = "json")]
pub mod json_backend;

#[cfg(feature = "axum-router")]
pub mod routes;

pub use exporter::{ExportError, ExportFormat, ExportRequest, ExportResult, Exporter};
pub use page::{Margins, Orientation, PageOptions, PageSize};
