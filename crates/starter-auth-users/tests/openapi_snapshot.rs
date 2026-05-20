//! Lock the canonical OpenAPI document to a checked-in snapshot.
//!
//! `pnpm codegen` reads the snapshot at workspace-root `openapi.json` to
//! generate the TS wire types. Whenever a handler signature or DTO
//! changes, this test fails — run with `UPDATE_SNAPSHOTS=1` to refresh.
//!
//! The snapshot is the **merged** OpenAPI doc for every public crate
//! the frontend client speaks to. Right now that's `starter-auth-users`
//! (auth, sessions, MFA, users admin) and `starter-ui-theme`
//! (org-level theme). Add new crates here as they expose HTTP surfaces
//! the TS client needs to call.

use std::path::PathBuf;

#[test]
fn openapi_matches_snapshot() {
    let mut doc =
        starter_auth_users::openapi::openapi(&starter_auth_users::signup::SignupMode::Disabled);
    doc.merge(starter_ui_theme::openapi::openapi());
    let actual = serde_json::to_string_pretty(&doc).expect("serialize openapi");

    let path = workspace_root().join("openapi.json");

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, format!("{actual}\n")).expect("write snapshot");
        return;
    }

    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e} — run with UPDATE_SNAPSHOTS=1", path.display()));
    let expected = expected.trim_end_matches('\n');
    if expected != actual {
        panic!(
            "openapi.json drift — run `UPDATE_SNAPSHOTS=1 cargo test -p starter-auth-users --test openapi_snapshot` to refresh.\n\nfirst 200 chars actual:\n{}\n\nfirst 200 chars expected:\n{}",
            &actual.chars().take(200).collect::<String>(),
            &expected.chars().take(200).collect::<String>(),
        );
    }
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is .../crates/starter-auth-users
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}
