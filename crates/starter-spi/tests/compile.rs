//! Smoke test: the crate compiles and the headline re-exports are
//! reachable from outside. If this test won't build, downstream
//! consumers won't build either.

use starter_spi::{Cursor, Error, Id, Page, Result};

#[test]
fn types_are_reachable() {
    fn _accept_error(_: Error) {}
    fn _accept_result(_: Result<()>) {}
    fn _accept_page(_: Page<String>) {}
    fn _accept_cursor(_: Cursor) {}

    struct User;
    let _id: Id<User> = Id::new();
}
