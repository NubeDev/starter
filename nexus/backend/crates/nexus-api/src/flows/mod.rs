//! Flow-builder domain logic: the bounded dry-run that powers the editor's
//! "Test" button. Saved-flow CRUD lives in the store; this module owns only the
//! test-run orchestration the transport layer calls into.

pub mod dry_run;
