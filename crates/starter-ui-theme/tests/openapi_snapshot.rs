//! Lock the canonical OpenAPI document to a checked-in snapshot,
//! mirroring the pattern in `starter-auth-users::tests::openapi_snapshot`.

#![cfg(feature = "routes")]

use std::path::PathBuf;

#[test]
fn openapi_matches_snapshot() {
    let doc = starter_ui_theme::openapi::openapi();
    let actual = serde_json::to_string_pretty(&doc).expect("serialize openapi");

    let path = snapshot_path();

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, format!("{actual}\n")).expect("write snapshot");
        return;
    }

    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e} — run with UPDATE_SNAPSHOTS=1", path.display()));
    let expected = expected.trim_end_matches('\n');
    if expected != actual {
        panic!(
            "ui-theme OpenAPI drift — run `UPDATE_SNAPSHOTS=1 cargo test -p starter-ui-theme --test openapi_snapshot` to refresh.",
        );
    }
}

fn snapshot_path() -> PathBuf {
    // CARGO_MANIFEST_DIR is .../crates/starter-ui-theme
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("DOCS/backend/openapi/ui-theme.openapi.json");
    p
}
