//! Smoke-tests for the OpenAPI document fragments exposed by
//! `starter-audit`, `starter-agent-log`, `starter-undo`, and
//! `starter-clipboard`. We just make sure each `*Api::openapi()`
//! compiles and emits the expected path so the document can be
//! merged into a consumer's `OpenApi` per SCOPE R7.

use starter_agent_log::AgentLogApi;
use starter_audit::AuditApi;
use starter_clipboard::ClipboardApi;
use starter_undo::UndoApi;
use utoipa::OpenApi;

#[test]
fn audit_openapi_emits_audit_path() {
    let doc = AuditApi::openapi();
    assert!(doc.paths.paths.contains_key("/v1/audit"));
}

#[test]
fn agent_log_openapi_emits_agent_log_path() {
    let doc = AgentLogApi::openapi();
    assert!(doc.paths.paths.contains_key("/v1/agent-log"));
}

#[test]
fn undo_openapi_emits_undo_and_redo_paths() {
    let doc = UndoApi::openapi();
    assert!(doc.paths.paths.contains_key("/v1/undo"));
    assert!(doc.paths.paths.contains_key("/v1/redo"));
}

#[test]
fn clipboard_openapi_emits_copy_and_paste_paths() {
    let doc = ClipboardApi::openapi();
    assert!(doc.paths.paths.contains_key("/v1/clipboard/copy"));
    assert!(doc.paths.paths.contains_key("/v1/clipboard/paste"));
}

#[test]
fn change_schema_present_in_audit_doc() {
    let doc = AuditApi::openapi();
    let components = doc.components.expect("components present");
    assert!(components.schemas.contains_key("Change"));
    assert!(components.schemas.contains_key("ChangePage"));
    assert!(components.schemas.contains_key("Actor"));
    assert!(components.schemas.contains_key("Op"));
    assert!(components.schemas.contains_key("ResourceRef"));
}
