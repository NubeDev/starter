//! Process-flavour mirror of `examples/hello-builtin`. SCOPE.md "One source,
//! three flavours" smoke test: the trait impl is byte-identical to the
//! builtin example; only the entry-point macro flips.
//!
//! See `examples/hello-builtin/src/lib.rs` for the line-by-line rationale.

use starter_ext_sdk::Extension;

/// The extension's unit struct. SCOPE R5: no fields. State lives in
/// the host-provided Ctx; the struct itself is `()`-sized.
#[derive(Extension)]
#[extension(manifest = "block.yaml")]
pub struct Hello;

starter_ext_sdk::requires! {
    name = HelloCtx,
    capabilities = [],
}

impl HelloToolHandlers for Hello {
    type Ctx = HelloCtx;

    fn handle_com_acme_hello_echo(
        &self,
        _ctx: &Self::Ctx,
        params: starter_ext_sdk::serde_json::Value,
    ) -> starter_ext_sdk::Result<starter_ext_sdk::serde_json::Value> {
        // Echo the input verbatim. Identical to the builtin handler —
        // SCOPE R1 demands that swapping flavours does not touch this body.
        Ok(params)
    }
}

// Emits `pub async fn run() -> starter_ext_sdk::Result<()>` driving the
// stdio JSON-RPC loop the supervisor speaks to. The single delta from
// `register_static_table!` is the entry-point — the trait impl above is
// identical (R1).
starter_ext_sdk::register_process_main! {
    extension: Hello,
    ctx: HelloCtx,
    instance: Hello,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            // Surface the cause on stderr; the supervisor's stderr-
            // forwarder pushes it into the per-extension event ring.
            eprintln!("hello-process exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
