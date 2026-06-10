//! Tenant-scoped agent and session persistence.
//!
//! Mirrors the flow store: every function opens a tenant-bound transaction so
//! RLS isolates the rows, and reads key on the immutable id. An *agent* is a
//! saved nexus-ai configuration; a *session* is one run against it. Config and
//! transcript are stored as jsonb and returned verbatim — the nexus-ai facade
//! interprets them at run time, the store only persists them.

mod delete;
mod fetch;
mod insert;
mod record;
mod update;

pub use delete::delete;
pub use fetch::{get, get_session, list, list_sessions};
pub use insert::{insert, insert_session};
pub use record::{AgentPatch, AgentRecord, NewAgent, NewSession, SessionRecord};
pub use update::{set_session_status, set_session_transcript, update};
