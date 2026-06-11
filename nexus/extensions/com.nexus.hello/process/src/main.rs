//! `com.nexus.hello` — process flavour.
//!
//! A real supervised child: the host spawns this binary, frames stdio
//! JSON-RPC against it, health-checks it, restarts it per the manifest's
//! `supervision:` policy, and reports live pid/uptime/RSS/CPU. This is the
//! genuine "start / stop / reload an extension" experience — unlike a builtin
//! bundle, there's an actual process to supervise.
//!
//! The trait impl is the same shape as the upstream `hello-process` example;
//! only the ids/names are nexus's. `register_process_main!` emits the
//! `run()` loop the supervisor speaks to.

use starter_ext_sdk::Extension;

/// The extension's unit struct (SCOPE R5: no fields — state lives in Ctx).
#[derive(Extension)]
// `block.yaml` lives in the bundle root, one level up from this crate.
#[extension(manifest = "../block.yaml")]
pub struct Hello;

starter_ext_sdk::requires! {
    name = HelloCtx,
    capabilities = [],
}

impl HelloToolHandlers for Hello {
    type Ctx = HelloCtx;

    /// Handler for the `com.nexus.hello.echo` tool. The macro derives the
    /// method name from the tool id (`.` → `_`). Echoes the validated input
    /// back verbatim — enough to prove the spawn + JSON-RPC round-trip works.
    fn handle_com_nexus_hello_echo_tool(
        &self,
        _ctx: &Self::Ctx,
        params: starter_ext_sdk::serde_json::Value,
    ) -> starter_ext_sdk::Result<starter_ext_sdk::serde_json::Value> {
        Ok(params)
    }
}

// Emits `pub async fn run() -> starter_ext_sdk::Result<()>` driving the stdio
// JSON-RPC loop the supervisor speaks to.
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
            // The supervisor's stderr-forwarder pushes this into the
            // per-extension event ring.
            eprintln!("nexus-hello-extension exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
