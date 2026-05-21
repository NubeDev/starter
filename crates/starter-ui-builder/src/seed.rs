//! Idempotent SDUI page seeding.
//!
//! [`seed_page`] is the port of `rubix-sdui-builder::seed::seed_page`.
//! It ensures the folder chain exists, ensures the `ui.page` node
//! exists (returning the existing one if so), strict-validates the
//! layout against [`starter_ui_ir::ComponentTree`], then writes the
//! `title`, `layout`, and `visible_to` slots. Slot writes always
//! run — they act as an implicit upsert so the layout tracks the
//! code on every restart.
//!
//! # Idempotency contract
//!
//! Every node create on this path goes through
//! [`PageStore::find_or_create_node`], which the host must implement
//! atomically: a second `seed_page` call against an existing page
//! must reuse the existing node, not mint `<page>-2`. The Rubix
//! origin of this code had a long write-up of an `accreted-pages`
//! bug; the contract here exists to prevent the same.
//!
//! # Divergence from Rubix
//!
//! Rubix wires [`seed_page`] through `block_client::GraphClient`
//! directly; starter has no equivalent runtime crate, so this port
//! defers to a [`PageStore`] trait the host implements. Two methods:
//! [`PageStore::find_or_create_node`] and [`PageStore::write_slot`].
//! The function stays synchronous — consumers wrap an async store
//! at the call site (`block_on(...)`, dispatcher, etc.).
//!
//! Errors writing individual slots are logged via `eprintln!` but do
//! not fail the call — the host can still serve actions even if a
//! page didn't seed. Layout validation, by contrast, is a hard
//! error: a broken tree is a programming bug.

use serde_json::{Value as JsonValue, json};
use starter_ui_ir::ComponentTree;
use thiserror::Error;

/// Errors returned by [`seed_page`].
#[derive(Debug, Error)]
pub enum SeedError {
    /// The provided layout did not deserialise as a
    /// [`ComponentTree`]. Typo, wrong field name, missing
    /// `ir_version`, etc.
    #[error("layout is not a valid ComponentTree: {0}")]
    InvalidLayout(#[from] serde_json::Error),
}

/// Backing store the host wires into [`seed_page`]. Implementations
/// must guarantee that [`find_or_create_node`](Self::find_or_create_node)
/// is atomic under the host's write lock — a second call with the
/// same `(parent_path, kind, name)` triple must return the existing
/// node, not mint a sibling.
pub trait PageStore {
    /// Ensure a node exists at `parent_path/<name>` of the given
    /// `kind`. Idempotent: a second call with the same arguments
    /// must reuse the existing node.
    fn find_or_create_node(
        &mut self,
        parent_path: &str,
        kind: &str,
        name: &str,
    ) -> Result<(), String>;

    /// Write the named slot on the node at `path`. Overwrites any
    /// previous value (slot writes are upserts).
    fn write_slot(
        &mut self,
        path: &str,
        slot: &str,
        value: &JsonValue,
    ) -> Result<(), String>;
}

/// Idempotently seed a `ui.page` SDUI page.
///
/// `parent_path` is the folder chain to ensure (e.g.
/// `"/sys/ui/pages"`); every segment is created if missing via
/// `find_or_create_node("sys.core.folder", …)`. `page_name` is the
/// leaf `ui.page` node. `layout` must round-trip through
/// [`ComponentTree`] — a structural mismatch fails fast at startup
/// rather than at resolve time.
///
/// Errors writing individual slots are logged but do not fail the
/// call. Layout validation is a hard error.
///
/// Idempotency: this function is safe to call on every block /
/// service restart. If the page already exists the existing node is
/// reused; slot writes upsert the body so layout edits in code still
/// ship.
pub fn seed_page<S: PageStore + ?Sized>(
    store: &mut S,
    parent_path: &str,
    page_name: &str,
    title: &str,
    layout: JsonValue,
    visible_to: &[&str],
) -> Result<(), SeedError> {
    // Strict-validate before any writes — fail fast on author error.
    let _: ComponentTree = serde_json::from_value(layout.clone())?;

    ensure_folder_chain(store, parent_path);

    // Ensure the ui.page node exists. Errors here (e.g. kind-mismatch
    // because some other extension previously created a different
    // kind at this path) are non-fatal — the slot writes below will
    // fail visibly with a clear error, which is the right place for
    // the operator to see the conflict.
    if let Err(e) = store.find_or_create_node(parent_path, "ui.page", page_name) {
        eprintln!(
            "[starter-ui-builder] failed to ensure page node {parent_path}/{page_name}: {e}"
        );
    }

    let page_path = format!("{}/{}", parent_path.trim_end_matches('/'), page_name);

    if let Err(e) = store.write_slot(&page_path, "title", &json!(title)) {
        eprintln!(
            "[starter-ui-builder] failed to write title slot at {page_path}: {e}"
        );
    }
    if let Err(e) = store.write_slot(&page_path, "layout", &layout) {
        eprintln!(
            "[starter-ui-builder] failed to write layout slot at {page_path}: {e}"
        );
    }
    let visible_to_json = json!(visible_to);
    if let Err(e) = store.write_slot(&page_path, "visible_to", &visible_to_json) {
        eprintln!(
            "[starter-ui-builder] failed to write visible_to slot at {page_path}: {e}"
        );
    }

    Ok(())
}

/// Ensure every segment of `parent_path` exists as a
/// `sys.core.folder`. Each segment goes through
/// [`PageStore::find_or_create_node`], so reloads reuse the existing
/// folders rather than minting `sys-2`, `ui-3`, … under the root.
/// Errors are logged but don't fail the call — a missing folder
/// shows up downstream as a slot-write error against a non-existent
/// path, which is more diagnostic than a fail-fast here.
fn ensure_folder_chain<S: PageStore + ?Sized>(store: &mut S, parent_path: &str) {
    let segments: Vec<&str> = parent_path.split('/').filter(|s| !s.is_empty()).collect();
    let mut cursor = String::from("/");
    for seg in segments {
        if let Err(e) = store.find_or_create_node(&cursor, "sys.core.folder", seg) {
            eprintln!("[starter-ui-builder] failed to ensure folder {cursor}/{seg}: {e}");
        }
        if cursor != "/" {
            cursor.push('/');
        }
        cursor.push_str(seg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    use crate::dashboard::dashboard;
    use crate::display::heading;

    /// In-memory store recording every call. `find_or_create_node`
    /// is atomic in the sense that repeated calls are no-ops — the
    /// idempotency contract.
    #[derive(Default)]
    struct InMemoryStore {
        nodes: HashSet<(String, String, String)>,
        slots: Vec<(String, String, JsonValue)>,
    }

    impl PageStore for InMemoryStore {
        fn find_or_create_node(
            &mut self,
            parent_path: &str,
            kind: &str,
            name: &str,
        ) -> Result<(), String> {
            self.nodes
                .insert((parent_path.to_owned(), kind.to_owned(), name.to_owned()));
            Ok(())
        }

        fn write_slot(
            &mut self,
            path: &str,
            slot: &str,
            value: &JsonValue,
        ) -> Result<(), String> {
            self.slots
                .push((path.to_owned(), slot.to_owned(), value.clone()));
            Ok(())
        }
    }

    #[test]
    fn seed_page_writes_three_slots_and_is_idempotent() {
        let tree = dashboard("p", "Title", [heading("Hello").build()]);
        let layout = serde_json::to_value(&tree).unwrap();

        let mut store = InMemoryStore::default();
        seed_page(
            &mut store,
            "/sys/ui/pages",
            "overview",
            "Overview",
            layout.clone(),
            &["admin", "operator"],
        )
        .unwrap();

        // Two folder segments (`sys`, `ui/pages` etc — see ensure_folder_chain)
        // plus the ui.page entry must be present.
        assert!(store.nodes.contains(&(
            "/sys/ui".to_owned(),
            "sys.core.folder".to_owned(),
            "pages".to_owned(),
        )));
        assert!(store.nodes.contains(&(
            "/sys/ui/pages".to_owned(),
            "ui.page".to_owned(),
            "overview".to_owned(),
        )));

        // Three slot writes — title, layout, visible_to.
        let slot_names: Vec<&str> = store.slots.iter().map(|(_, s, _)| s.as_str()).collect();
        assert_eq!(slot_names, vec!["title", "layout", "visible_to"]);

        // Idempotency — a second call must not duplicate folder
        // nodes or page nodes.
        let nodes_before = store.nodes.len();
        seed_page(
            &mut store,
            "/sys/ui/pages",
            "overview",
            "Overview",
            layout,
            &["admin", "operator"],
        )
        .unwrap();
        assert_eq!(
            store.nodes.len(),
            nodes_before,
            "second seed_page must not create new nodes"
        );
    }

    #[test]
    fn seed_page_rejects_garbage_layout_before_any_writes() {
        let mut store = InMemoryStore::default();
        let err = seed_page(
            &mut store,
            "/sys/ui/pages",
            "p",
            "T",
            json!({ "not_a_tree": true }),
            &[],
        )
        .unwrap_err();
        assert!(matches!(err, SeedError::InvalidLayout(_)));
        assert!(store.nodes.is_empty(), "no writes on validation failure");
        assert!(store.slots.is_empty());
    }
}
