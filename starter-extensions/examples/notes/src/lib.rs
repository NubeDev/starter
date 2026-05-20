//! `starter-notes-ext` — single extension reaching every surface the
//! v0.1 substrate exposes.
//!
//! Layout follows the same shape as `hello-cli`:
//!
//! - The unit struct + `#[derive(Extension)]` + `requires!{}` define
//!   the trait the proc-macro generates (`NotesToolHandlers`). One
//!   handler per `contributes.tools[]` entry.
//! - `register_static_table!` exposes the builtin dispatch table that
//!   `starter-ext-mcp` (tools) and `starter-ext-server` (REST) look up
//!   contributions in.
//! - REST contributions live in the same `BuiltinTable` entry — the
//!   adapter calls them by their full contribute id, so they share one
//!   closure with the tools.
//! - CLI handlers are not part of the proc-macro dispatch path in
//!   v0.1; `main.rs` registers them into a `BuiltinCliRegistry`
//!   separately (the SDK pattern from `hello-cli`).
//!
//! State (the in-memory `NoteStore`) is a `Lazy` static rather than a
//! field on the unit struct: SCOPE R5 forbids `&mut self` and requires
//! the extension struct to be field-less, so per-host state lives next
//! to the handlers in a process-global. For a real product that store
//! would move behind a host-provided capability (a future `kv:`); for
//! the example, in-memory is enough to prove the wiring.
//!
//! Note on `register_static_table!`: the macro's emitted closure only
//! dispatches `contributes.tools[]` ids and rejects unknown ids with a
//! "tool not declared" error. Because this extension also contributes
//! REST routes (which the REST adapter dispatches against the *same*
//! `BuiltinTable` by contribute id), we hand-roll the table entry in
//! [`build_builtin_table`] so one closure handles every cross-surface
//! id. The proc-macro's compile-time check that every declared tool
//! has a `handle_*` method is still in force via the
//! `NotesToolHandlers` impl below.

use std::sync::{Mutex, OnceLock};

use starter_ext_sdk::serde_json::{self, Value};
use starter_ext_sdk::Extension;

// ---------------------------------------------------------------------------
// Extension struct + generated dispatch trait.
// ---------------------------------------------------------------------------

/// SCOPE R5: no fields. State lives in the process-global [`store`].
#[derive(Extension)]
#[extension(manifest = "block.yaml")]
pub struct Notes;

starter_ext_sdk::requires! {
    name = NotesCtx,
    capabilities = [],
}

impl NotesToolHandlers for Notes {
    type Ctx = NotesCtx;

    fn handle_com_nube_notes_add(
        &self,
        _ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        add_note(&params)
    }

    fn handle_com_nube_notes_list(
        &self,
        _ctx: &Self::Ctx,
        _params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        Ok(list_notes())
    }

    fn handle_com_nube_notes_search(
        &self,
        _ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let query = params
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| starter_ext_spi::Error::validation("`query` (string) is required"))?;
        Ok(search_notes(query))
    }
}

/// Build the [`BuiltinTable`] entry that backs *every* contribute id
/// in this extension — tools and REST routes — so the MCP and REST
/// adapters share one closure. Replaces `register_static_table!` (which
/// only handles tools).
pub fn build_builtin_table() -> starter_ext_sdk::builtin::BuiltinTable {
    use starter_ext_sdk::builtin::{BuiltinEntry, BuiltinTable};
    let id = <Notes as starter_ext_sdk::ExtensionMeta>::id().clone();
    let entry = BuiltinEntry::new(
        // The set of declared ids the adapter pre-validates against.
        // Includes both tool ids and REST ids; the closure below routes
        // on the same strings.
        &[
            "com.nube.notes.add",
            "com.nube.notes.list",
            "com.nube.notes.search",
            "com.nube.notes.rest_create",
            "com.nube.notes.rest_list",
        ],
        |contribute_id, _ctx, params| match contribute_id {
            // MCP tools.
            "com.nube.notes.add" => add_note(&params),
            "com.nube.notes.list" => Ok(list_notes()),
            "com.nube.notes.search" => {
                let q = params
                    .get("query")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        starter_ext_spi::Error::validation("`query` (string) is required")
                    })?;
                Ok(search_notes(q))
            }
            // REST routes — same backend, exposed at `POST /notes`,
            // `GET /notes`.
            "com.nube.notes.rest_create" => add_note(&params),
            "com.nube.notes.rest_list" => Ok(list_notes()),
            other => Err(starter_ext_spi::Error::validation(format!(
                "unknown contribute id: {other}"
            ))),
        },
    );
    let mut table = BuiltinTable::new();
    table.insert(id, entry);
    table
}

// ---------------------------------------------------------------------------
// Shared logic — also reached by the REST + CLI host adapters.
// ---------------------------------------------------------------------------

/// One persisted note. Plain Rust — no `starter-*` types leak into the
/// domain layer (mirrors the original `examples/notes` discipline).
#[derive(Debug, Clone)]
pub struct Note {
    pub id: u64,
    pub body: String,
}

impl Note {
    fn to_json(&self) -> Value {
        serde_json::json!({ "id": self.id, "body": &self.body })
    }
}

fn store() -> &'static Mutex<Vec<Note>> {
    static STORE: OnceLock<Mutex<Vec<Note>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Append a note from a `{ "body": "..." }` payload.
pub fn add_note(params: &Value) -> starter_ext_sdk::Result<Value> {
    let body = params
        .get("body")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| starter_ext_spi::Error::validation("`body` (non-empty string) is required"))?
        .to_owned();
    let mut guard = store().lock().expect("notes store poisoned");
    let id = guard.len() as u64 + 1;
    let note = Note { id, body };
    let snapshot = note.to_json();
    guard.push(note);
    Ok(snapshot)
}

/// Return every note as `{ "notes": [...] }`.
pub fn list_notes() -> Value {
    let guard = store().lock().expect("notes store poisoned");
    serde_json::json!({
        "notes": guard.iter().map(Note::to_json).collect::<Vec<_>>(),
    })
}

/// Return notes whose body contains `query` (case-insensitive).
pub fn search_notes(query: &str) -> Value {
    let needle = query.to_lowercase();
    let guard = store().lock().expect("notes store poisoned");
    serde_json::json!({
        "notes": guard
            .iter()
            .filter(|n| n.body.to_lowercase().contains(&needle))
            .map(Note::to_json)
            .collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------
// CLI handlers. Registered by `main.rs` into a BuiltinCliRegistry —
// CLI contributions are out of scope for the v0.1 proc-macro dispatch.
// ---------------------------------------------------------------------------

/// `notes-add --body "<text>"` — non-streaming.
pub fn cli_add(
    params: Value,
    _ctx: &starter_ext_sdk::ctx::CtxInner,
) -> starter_ext_sdk::Result<Value> {
    add_note(&params)
}

/// `notes-list` — non-streaming.
pub fn cli_list(
    _params: Value,
    _ctx: &starter_ext_sdk::ctx::CtxInner,
) -> starter_ext_sdk::Result<Value> {
    Ok(list_notes())
}
