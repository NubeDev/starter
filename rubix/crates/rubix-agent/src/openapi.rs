//! OpenAPI document assembly for the rubix-agent REST surface.
//!
//! Mirrors `crates/starter-server/src/routes/openapi_doc.rs` plus the
//! per-crate `*Api::openapi()` pattern in
//! `crates/starter-auth-users/src/openapi.rs`: one
//! `#[derive(OpenApi)]` struct that lists every Axum handler in
//! `rubix-agent::{health, routes}` carrying a `#[utoipa::path]`
//! attribute, plus a tag-per-goal block that surfaces the nine
//! rubix verb groups so the TS codegen pipeline (`pnpm --filter
//! @nube/rubix-client-ts run codegen`) emits clean per-tag
//! namespaces. See `rubix/docs/design/agent/README.md` for the
//! runtime wiring picture and `rubix/docs/design/client-ts/README.md`
//! for the codegen contract.
//!
//! The struct is intentionally route-thin today: every goal verb
//! currently dispatches through the single
//! `POST /api/v1/tools/{tool_id}` handler (per the goals-2-4-3
//! audit landed in stage 4), so the tag declarations carry the
//! per-goal narrative even though only the dispatcher + `/healthz`
//! carry concrete path attributes. As individual verbs grow their
//! own dedicated handlers, they slot into the `paths(...)` list
//! below.

use utoipa::OpenApi;

/// utoipa entry point. The `info`, `servers`, `paths`, `components`,
/// and `tags` blocks together form the document the agent serves at
/// `GET /openapi.json`.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "rubix-agent",
        description = "Rubix backend REST surface. See `rubix/docs/design/agent/README.md` for the runtime wiring picture.",
        version = env!("CARGO_PKG_VERSION"),
    ),
    servers(
        (url = "http://127.0.0.1:8088", description = "Default development bind (RUBIX_BIND)."),
    ),
    paths(
        crate::health::healthz,
        crate::routes::tools::dispatch,
    ),
    tags(
        (name = "auth", description = "Cookie-session + API-token authentication (delegated to starter-auth-users)."),
        (name = "system", description = "System probes: disk, db, flow errors (Goal 5 — system-check + alert)."),
        (name = "user-admin", description = "User-admin verbs (Goal 2): user/team/tenant create + disable + list."),
        (name = "clickhouse-ruler", description = "ClickHouse ruler verbs (Goal 4): rule write, mart create, retention set."),
        (name = "flow-programmer", description = "Flow programmer verbs (Goal 3): flow deploy, lint, list, duplicate."),
        (name = "mcp", description = "MCP JSON-RPC over HTTP (tools/list, tools/call)."),
        (name = "undo", description = "Undo dispatcher (`rubix.undo.last`) — reverses the most recent reversible change."),
        (name = "dashboard", description = "Goal 1 (dashboards): create, update, get, list, duplicate, delete, page_set."),
        (name = "weekly-report-stub", description = "Goal 6 (weekly-report) stub surface — `code = rubix.goal.not_wired`."),
    ),
)]
pub struct RubixApi;

/// Assemble the OpenAPI document the rubix-agent binary serves at
/// `GET /openapi.json`. Captured once at boot and served by value
/// per the starter-server precedent.
///
/// Merges in `starter-auth-users`' utoipa document so the
/// `/api/v1/auth/{login,logout,me,token}` routes the agent mounts
/// (via `crate::routes::auth::auth_router`) are described in the
/// served doc and picked up by downstream codegen
/// (`pnpm --filter @nube/rubix-client-ts run codegen`, `make
/// api-client` in `rubix/flutter/`). Signup is never mounted by
/// the agent, so we pass `SignupMode::Disabled`.
pub fn rubix_openapi() -> utoipa::openapi::OpenApi {
    let mut doc = RubixApi::openapi();
    let mut auth = starter_auth_users::openapi::openapi(
        &starter_auth_users::signup::SignupMode::Disabled,
    );
    prefix_paths(&mut auth, "/api/v1");
    doc.merge(auth);
    dedupe_tags(&mut doc);
    doc
}

/// Re-key every entry in `doc.paths.paths` so it starts with
/// `prefix`. utoipa's `OpenApi::merge` does NOT add path prefixes,
/// and the rubix binary nests the `starter-auth-users` router
/// under `/api/v1` in `main.rs` — without this step the served
/// document would advertise `/auth/me` while the live route is
/// `/api/v1/auth/me`.
fn prefix_paths(doc: &mut utoipa::openapi::OpenApi, prefix: &str) {
    let old = std::mem::take(&mut doc.paths.paths);
    for (path, item) in old {
        let new_path = format!("{prefix}{path}");
        doc.paths.paths.insert(new_path, item);
    }
}

/// Drop duplicate `tags[]` entries by `name`, keeping the first
/// occurrence. The rubix doc already declares an `auth` tag with
/// the goal-narrative description; the merged starter-auth-users
/// doc also declares `auth`. Without this dedupe the tag list
/// grows to 10 and the per-goal contract in
/// `tests/openapi_test.rs` (`tags.len() == 9`) breaks.
fn dedupe_tags(doc: &mut utoipa::openapi::OpenApi) {
    let Some(tags) = doc.tags.as_mut() else {
        return;
    };
    let mut seen = std::collections::HashSet::new();
    tags.retain(|t| seen.insert(t.name.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_assembles() {
        let doc = rubix_openapi();
        assert_eq!(doc.info.title, "rubix-agent");
        let tags = doc.tags.as_ref().expect("tags block declared");
        assert_eq!(tags.len(), 9, "one tag per goal area");
    }

    #[test]
    fn document_includes_canary_paths() {
        let doc = rubix_openapi();
        assert!(
            doc.paths.paths.contains_key("/healthz"),
            "healthz canary path present",
        );
        assert!(
            doc.paths.paths.contains_key("/api/v1/tools/{tool_id}"),
            "tools dispatcher canary path present",
        );
    }

    #[test]
    fn document_includes_auth_paths() {
        let doc = rubix_openapi();
        for path in [
            "/api/v1/auth/login",
            "/api/v1/auth/logout",
            "/api/v1/auth/me",
            "/api/v1/auth/token",
        ] {
            assert!(
                doc.paths.paths.contains_key(path),
                "auth path `{path}` merged from starter-auth-users; got keys {:?}",
                doc.paths.paths.keys().collect::<Vec<_>>(),
            );
        }
        // The bare unprefixed path must NOT appear — that would mean
        // `prefix_paths` skipped an entry.
        assert!(
            !doc.paths.paths.contains_key("/auth/me"),
            "unprefixed /auth/me leaked into served document",
        );
    }
}
