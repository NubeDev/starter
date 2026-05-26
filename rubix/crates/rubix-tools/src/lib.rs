//! Rubix tool implementations grouped by goal.
//!
//! One file per verb (FILE-LAYOUT §2): each `<goal>/<verb>.rs` ships
//! dispatch logic only — DTOs and descriptors live in `rubix-spi`.
//!
//! Several of these are upstream candidates (`starter-tool-sysdiag`,
//! `starter-tool-flow-ops`, `starter-tool-clickhouse`,
//! `starter-tool-sdui`, `starter-tool-tags`). When those land, the
//! matching rubix submodule becomes a thin re-export. See
//! [docs/design/tools/](../../docs/design/tools/README.md).

pub mod clipboard;
pub mod dashboard;
pub mod dataflow;
pub mod flow_ops;
pub mod insights;
pub mod system;
pub mod tags;
pub mod team;
pub mod tenant;
pub mod undo;
pub mod user;
