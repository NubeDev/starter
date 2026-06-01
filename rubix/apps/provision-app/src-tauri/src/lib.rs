//! Rubix Provision — Tauri v2 app core. Wires managed state (shared
//! HTTP client, live session, migrated SQLite queue), the store plugin,
//! and the command surface. Kept thin: all behaviour lives in the
//! `agent`, `queue`, and `commands` modules.

mod agent;
mod commands;
mod error;
mod queue;
mod store;

use tauri::Manager;
use tokio::sync::Mutex;

use agent::client::{AgentClient, AgentClientState};
use agent::session::SessionState;
use queue::open::{open, QueueDb};

/// Build and run the app. Called by `main.rs` (desktop) and by the
/// generated mobile entrypoint below.
pub fn run() {
    // The shared client owns the cookie jar; fail loudly if reqwest
    // cannot build (e.g. no TLS backend) — there is no app without it.
    let client = AgentClient::new().expect("failed to build agent HTTP client");

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(AgentClientState(std::sync::Arc::new(client)))
        .manage(SessionState::default())
        .setup(|app| {
            // Open + migrate the offline queue under the app data dir.
            let dir = app.path().app_data_dir()?;
            let conn = open(&dir)?;
            app.manage(QueueDb(Mutex::new(conn)));

            // Pre-fill the session base_url from the last run so the
            // login screen defaults to the same agent (no auth yet).
            if let Some(base) = store::base_url::load(app.handle()) {
                let state = app.state::<SessionState>();
                state.0.blocking_lock().base_url = Some(base);
            }
            Ok(())
        })
        // Each path points at the verb's own module so the macro finds
        // both the `#[command]` fn and its generated `__cmd__*` helper.
        .invoke_handler(tauri::generate_handler![
            commands::auth_login::auth_login,
            commands::auth_me::auth_me,
            commands::auth_logout::auth_logout,
            commands::tool_dispatch::tool_dispatch,
            commands::queue_enqueue::queue_enqueue,
            commands::queue_list::queue_list,
            commands::queue_flush::queue_flush,
            commands::queue_drop::queue_drop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rubix Provision");
}

/// Mobile (iOS/Android) entrypoint. Tauri's mobile harness calls a
/// `#[mobile_entry_point]`-annotated function named `mobile_entry_point`
/// in the lib crate.
#[cfg(mobile)]
#[tauri::mobile_entry_point]
fn mobile_entry_point() {
    run();
}
