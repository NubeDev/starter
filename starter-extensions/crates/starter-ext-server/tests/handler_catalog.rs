//! v1 read-only handler declaration mechanism.
//!
//! Hard error at registration when a writing handler does not
//! declare `affects_tables`. Read-only handlers and properly-
//! declared writers register cleanly. Pinned per §"Read-only
//! handler declaration" of fe-cache-opt-in.md.

use starter_ext_server::rest::cache::{
    HandlerCatalog, HandlerCatalogBuilder, HandlerMeta, HandlerRegistrationError,
};
use starter_ext_spi::ExtensionId;

fn ext(s: &str) -> ExtensionId {
    ExtensionId::new(s).unwrap()
}

#[test]
fn read_only_handler_registers_cleanly() {
    let mut b = HandlerCatalogBuilder::new();
    b.register(ext("com.acme.ext"), "list_things", HandlerMeta::read_only())
        .expect("read-only handler registers");
    let cat = b.build();
    assert_eq!(cat.len(), 1);
    let meta = cat.get(&ext("com.acme.ext"), "list_things").unwrap();
    assert!(meta.read_only);
    assert!(meta.affects_tables.is_empty());
}

#[test]
fn writing_handler_with_tables_registers() {
    let cat = HandlerCatalog::from_entries([(
        ext("com.acme.ext"),
        "create_thing".to_string(),
        HandlerMeta::writing(["things", "thing_audits"]),
    )])
    .expect("writer with tables registers");
    let meta = cat.get(&ext("com.acme.ext"), "create_thing").unwrap();
    assert!(!meta.read_only);
    assert_eq!(meta.affects_tables, vec!["things", "thing_audits"]);
    assert_eq!(
        meta.invalidation_tags(),
        vec!["table:things".to_string(), "table:thing_audits".to_string()]
    );
}

#[test]
fn writing_handler_without_tables_is_hard_error() {
    let err = HandlerCatalog::from_entries([(
        ext("com.acme.ext"),
        "broken_writer".to_string(),
        HandlerMeta {
            read_only: false,
            affects_tables: Vec::new(),
        },
    )])
    .unwrap_err();
    match err {
        HandlerRegistrationError::WritingHandlerMissingTables {
            extension,
            contribute_id,
        } => {
            assert_eq!(extension, "com.acme.ext");
            assert_eq!(contribute_id, "broken_writer");
        }
    }
}
