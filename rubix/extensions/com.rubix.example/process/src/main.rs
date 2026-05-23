//! Reference extension binary.
//!
//! Phase 0 placeholder — the planned `rubix-extensions-sdk` crate
//! does not yet exist (see docs/design/STARTER-CHANGES.md). Until
//! Phase 5 lands the SDK + the `starter-ext-flow` adapter, this
//! file documents the expected entry point shape.
//!
//! Expected real implementation (Phase 5):
//!
//! ```ignore
//! use rubix_extensions_sdk::{run_process_plugin, Tool, NodeCtx};
//!
//! struct EchoTool;
//!
//! #[async_trait::async_trait]
//! impl Tool for EchoTool {
//!     // ... per starter_spi::Tool ...
//! }
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     run_process_plugin(vec![Box::new(EchoTool)]).await
//! }
//! ```

fn main() {
    eprintln!(
        "rubix-example-extension is a Phase 5 placeholder; \
         rubix-extensions-sdk is not yet available."
    );
}
