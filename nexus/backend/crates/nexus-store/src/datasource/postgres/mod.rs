//! The Postgres connector — how nexus connects to and queries a Postgres
//! datasource. Folder-per-connector: a new kind (e.g. `mqtt/`) is a sibling
//! folder, additive, with no edit to this one.

mod connect;

pub use connect::connect;
