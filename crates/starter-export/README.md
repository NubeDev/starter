# starter-export

Reusable, transport-agnostic export pipeline for the starter
ecosystem: PDF, HTML, CSV, JSON — behind a single `Exporter` trait so
REST, CLI and MCP all dispatch the same way (SCOPE.md R3).

Default features are empty (SCOPE.md R5). Turn on only the formats
you actually ship.

```toml
[dependencies]
starter-export = { version = "0.1", features = ["pdf", "csv", "axum-router"] }
```

## Choosing a backend — be honest about "amazing PDFs"

| Need                                                       | Use                                          |
|------------------------------------------------------------|----------------------------------------------|
| Tables, invoices, plain-text reports                       | `pdf` feature → `PrintpdfExporter`           |
| Pixel-perfect "looks like the web page" PDF                | **frontend** → `@nube/starter-ui-export`     |
| CSV downloads of tabular data                              | `csv` feature → `CsvExporter`                |
| Print-from-browser HTML                                    | `html` feature → `HtmlExporter`              |
| Raw JSON download                                          | `json` feature → `JsonExporter`              |
| Mount `POST /v1/export` in your axum app                   | `axum-router` feature → `export_router`      |

Pure-Rust PDF libraries (`printpdf`, `genpdf`, `weasyprint`-via-FFI,
…) are good at structured documents but are **not** browser engines.
If you need CSS, web fonts, charts, flexbox or fidelity-with-the-UI,
generate the PDF on the frontend (`@nube/starter-ui-export` wraps
`window.print()` and `html2canvas` + `jspdf`) and either save it
client-side or `POST` the bytes back through your own handler. The
`Exporter` trait does not care where the bytes were produced.

## Extending

Implement `Exporter` for your own backend (e.g. a Chromium-driven
"render this URL" service, a Typst pipeline, a hosted DocRaptor
client) and hand it to `ExportRoutesState::exporter`. Multi-format
fan-out is one match arm:

```rust
use std::sync::Arc;
use async_trait::async_trait;
use starter_export::{ExportError, ExportFormat, ExportRequest, ExportResult, Exporter};

pub struct Dispatcher {
    pub pdf: Arc<dyn Exporter>,
    pub csv: Arc<dyn Exporter>,
}

#[async_trait]
impl Exporter for Dispatcher {
    fn supports(&self, f: ExportFormat) -> bool {
        matches!(f, ExportFormat::Pdf | ExportFormat::Csv)
    }
    async fn export(&self, req: ExportRequest) -> Result<ExportResult, ExportError> {
        match req.format {
            ExportFormat::Pdf => self.pdf.export(req).await,
            ExportFormat::Csv => self.csv.export(req).await,
            other => Err(ExportError::UnsupportedFormat(other)),
        }
    }
}
```

## Hard rules satisfied

* **R1 — one job:** every file is < 200 lines; no `utils` module.
* **R2 — `spi`-only contract:** the crate depends on `starter-spi`
  for `Error`; everything else is local.
* **R3 — thin transport:** `routes::export` does extract → dispatch
  → shape; the format logic lives in the per-backend module.
* **R5 — opt-in everything:** all backends are feature-gated.
