//! Reference rubix extension — process flavour.
//!
//! Mirrors `starter-extensions/examples/hello-process/src/main.rs`.
//! Per SCOPE R8 the extension depends ONLY on `starter-ext-sdk`; per
//! SCOPE R1 / "One source, three flavours" the handler body would be
//! byte-identical to a builtin/wasm twin — only the entry-point macro
//! (`register_process_main!`) is flavour-specific.
//!
//! The host hands `block.yaml` to the SDK derive at compile time. Each
//! `contributes.tools[]` entry whose id is `com.rubix.example.foo`
//! produces a required handler method `handle_com_rubix_example_foo`
//! on the generated `*ToolHandlers` trait.

use starter_ext_sdk::Extension;

/// The extension's unit struct. SCOPE R5: no fields. All state lives
/// in the host-provided Ctx.
#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct Example;

starter_ext_sdk::requires! {
    name = ExampleCtx,
    capabilities = [],
}

impl ExampleToolHandlers for Example {
    type Ctx = ExampleCtx;

    /// `com.rubix.example.echo` — return the input verbatim.
    fn handle_com_rubix_example_echo(
        &self,
        _ctx: &Self::Ctx,
        params: starter_ext_sdk::serde_json::Value,
    ) -> starter_ext_sdk::Result<starter_ext_sdk::serde_json::Value> {
        Ok(params)
    }
}

// Emits `pub async fn run() -> starter_ext_sdk::Result<()>` driving
// the stdio JSON-RPC loop that the rubix-agent supervisor speaks to.
starter_ext_sdk::register_process_main! {
    extension: Example,
    ctx: ExampleCtx,
    instance: Example,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            // Supervisor's stderr-forwarder pushes this into the
            // per-extension event ring.
            eprintln!("rubix-example-extension exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
