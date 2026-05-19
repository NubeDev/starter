//! Test harness for consumers — `cargo test` against a real bound
//! server without touching network ports the host cares about.
//! Compiled only with `feature = "testing"`.

mod test_app;

pub use test_app::TestApp;
