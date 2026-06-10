//! AI assist DTOs — synchronous, task-typed AI assistance (vs. agent sessions).

pub mod assist;

pub use assist::{AssistRequest, AssistResponse, AssistTask};
