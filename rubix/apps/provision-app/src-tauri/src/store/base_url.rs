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
    match app.store(STORE_FILE) {
        Ok(store) => {
            store.set(BASE_URL_KEY, base_url.to_string());
            match store.save() {
                Ok(()) => eprintln!("[provision] base_url saved: {base_url}"),
                Err(e) => eprintln!("[provision] base_url save FAILED to flush: {e}"),
            }
        }
        Err(e) => eprintln!("[provision] base_url save FAILED to open store: {e}"),
    }
}

/// Read the last-used base_url, if any was persisted.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[provision] base_url load FAILED to open store: {e}");
            return None;
        }
    };
    let found = store
        .get(BASE_URL_KEY)
        .and_then(|v| v.as_str().map(str::to_owned));
    eprintln!("[provision] base_url load: {found:?}");
    found
}
