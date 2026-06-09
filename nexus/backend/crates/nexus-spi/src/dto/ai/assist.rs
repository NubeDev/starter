//! `POST /api/v1/ai/assist` — synchronous, structured AI assistance.
//!
//! Distinct from agent *sessions* (which stream a transcript over SSE): an assist
//! request is one-shot and task-typed. The frontend's query editor and dashboard
//! builder call this to turn a plain-English intent plus context (a datasource's
//! schema, the current SQL) into a single concrete artifact — a SQL string or a
//! panel/dashboard suggestion — without spinning up a conversation.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// What the caller wants the assistant to produce. The task selects the system
/// instructions and the expected shape of [`AssistResponse::result`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssistTask {
    /// Generate or edit a SQL query. `result` is `{ "sql": "..." }`.
    Sql,
    /// Suggest one dashboard panel. `result` is a panel spec
    /// `{ "title", "viz", "sql", "x"?, "value" }`.
    Panel,
    /// Suggest a whole dashboard. `result` is `{ "name", "panels": [ <panel>, … ] }`.
    Dashboard,
}

/// An assist request: a task, the user's natural-language intent, and optional
/// grounding context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AssistRequest {
    pub task: AssistTask,
    /// The user's plain-English ask (e.g. "average temperature per site, last 24h").
    pub prompt: String,
    /// Optional datasource the query should target; when set, the server grounds
    /// the model with that datasource's table/column schema so generated SQL
    /// references real columns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datasource_id: Option<String>,
    /// Optional existing SQL to edit/improve rather than write from scratch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_sql: Option<String>,
    /// Optional model override (concrete id or `small`/`medium`/`large`).
    /// Defaults to the service's medium tier when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// The assistant's structured answer. `result` shape depends on the request task
/// (see [`AssistTask`]); it is opaque JSON on the wire so the contract stays
/// stable as task outputs evolve. `raw` carries the model's unparsed reply for
/// debugging / when structured parsing degrades.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AssistResponse {
    pub task: AssistTask,
    pub result: Value,
    /// The model's raw text reply, retained so the UI can fall back to showing it
    /// if the structured `result` is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}
