//! Persist the agent `base_url` across launches via tauri-plugin-store.
//!
//! Only the base_url is persisted — NOT the session cookie or CSRF
//! token. The session is short-lived and CSRF is a secret; on relaunch
//! the user logs in again, but the base_url defaults to the last agent
//! they used (SCOPE allows persisting base_url + optionally the cookie;
//! we keep it to base_url to avoid writing a credential to plain JSON).

use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

/// Store file under the app config dir.
const STORE_FILE: &str = "provision-app.json";
/// Key inside the store JSON.
const BASE_URL_KEY: &str = "base_url";

/// Write the chosen base_url to disk. Best-effort: a store failure is
/// logged by the plugin and does not fail the login (the in-memory
/// session is already set by the caller).
pub fn save<R: Runtime>(app: &AppHandle<R>, base_url: &str) {
    if let Ok(store) = app.store(STORE_FILE) {
        store.set(BASE_URL_KEY, base_url.to_string());
        let _ = store.save();
    }
}

/// Read the last-used base_url, if any was persisted.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    let store = app.store(STORE_FILE).ok()?;
    store
        .get(BASE_URL_KEY)
        .and_then(|v| v.as_str().map(str::to_owned))
}
