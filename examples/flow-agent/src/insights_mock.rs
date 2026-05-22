//! Insights mock-up REST surface (see `INSIGHTS-MOCKUP.md`).
//!
//! All routes read/write JSON fixtures under
//! `examples/flow-agent/fixtures/insights/`. No engine, no SQL — the
//! storage is `tokio::sync::RwLock<InsightsFixtures>`. When the real
//! `starter-insights` crate lands this module is deleted; the
//! frontend keeps working thanks to I2 (the fixture JSON shapes are
//! the wire contract preview).

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

/// In-memory mirror of the fixture files. Each value is the parsed
/// JSON tree; handlers serialize back to disk under the write lock.
#[derive(Debug, Default)]
pub struct InsightsFixtures {
    pub rules: Vec<Value>,
    pub verdicts: Vec<Value>,
    pub pipelines: Vec<Value>,
    pub coverage: Value,
    pub tags_index: Value,
    /// Root dir; handlers write back here.
    pub root: PathBuf,
}

impl InsightsFixtures {
    pub fn load(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        let read_array = |name: &str| -> std::io::Result<Vec<Value>> {
            let p = root.join(name);
            let bytes = std::fs::read(&p)?;
            let parsed: Value = serde_json::from_slice(&bytes).map_err(io_err)?;
            match parsed {
                Value::Array(a) => Ok(a),
                _ => Err(io_err(format!("{name} is not a JSON array"))),
            }
        };
        let read_value = |name: &str| -> std::io::Result<Value> {
            let p = root.join(name);
            let bytes = std::fs::read(&p)?;
            serde_json::from_slice(&bytes).map_err(io_err)
        };
        Ok(Self {
            rules: read_array("rules.json")?,
            verdicts: read_array("verdicts.json")?,
            pipelines: read_array("pipelines.json")?,
            coverage: read_value("coverage.json")?,
            tags_index: read_value("tags-index.json")?,
            root,
        })
    }

    pub(crate) fn persist_array(&self, name: &str, rows: &[Value]) -> std::io::Result<()> {
        write_pretty(&self.root.join(name), &Value::Array(rows.to_vec()))
    }
}

fn io_err<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

fn write_pretty(path: &FsPath, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec_pretty(value).map_err(io_err)?;
    std::fs::write(path, body)
}

/// Shared state. Cloneable Arc — handlers go through `RwLock`.
#[derive(Clone)]
pub struct InsightsState {
    pub data: Arc<RwLock<InsightsFixtures>>,
}

impl InsightsState {
    pub fn new(fixtures: InsightsFixtures) -> Self {
        Self {
            data: Arc::new(RwLock::new(fixtures)),
        }
    }
}

pub fn router<S>(state: InsightsState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        // Rules
        .route(
            "/api/insights/rules",
            get(list_rules).post(create_rule),
        )
        .route(
            "/api/insights/rules/{id}",
            get(get_rule).patch(update_rule),
        )
        .route("/api/insights/rules/{id}/dry-run", post(dry_run_rule))
        // Verdicts
        .route("/api/insights/verdicts", get(list_verdicts))
        .route("/api/insights/verdicts/{id}", get(get_verdict))
        // Pipelines
        .route(
            "/api/insights/pipelines",
            get(list_pipelines).post(upsert_pipeline),
        )
        .route("/api/insights/pipelines/{id}", get(get_pipeline))
        // Coverage / tags helpers (read-only, helpful for the UI).
        .route("/api/insights/coverage", get(get_coverage))
        .route("/api/insights/tags", get(get_tags_index))
        .with_state(state)
}

// ---------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------

async fn list_rules(State(s): State<InsightsState>) -> Json<Value> {
    let g = s.data.read().await;
    Json(Value::Array(g.rules.clone()))
}

async fn get_rule(
    State(s): State<InsightsState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let g = s.data.read().await;
    g.rules
        .iter()
        .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_rule(
    State(s): State<InsightsState>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let mut g = s.data.write().await;
    if g.rules
        .iter()
        .any(|r| r.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
    {
        return Err(StatusCode::CONFLICT);
    }
    g.rules.push(body.clone());
    g.persist_array("rules.json", &g.rules.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(body)))
}

async fn update_rule(
    State(s): State<InsightsState>,
    Path(id): Path<String>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let mut g = s.data.write().await;
    let row = g
        .rules
        .iter_mut()
        .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
        .ok_or(StatusCode::NOT_FOUND)?;
    merge_object(row, &patch);
    let snapshot = g.rules.clone();
    g.persist_array("rules.json", &snapshot)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let updated = snapshot
        .into_iter()
        .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(updated))
}

/// Dry-run: synthesise a plausible verdict by picking the most recent
/// verdict for this rule from fixtures (no engine). If none exists,
/// fall back to a Healthy stub. This matches spec §Backend "dry-run
/// synthesises a verdict from fixtures, no engine".
async fn dry_run_rule(
    State(s): State<InsightsState>,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> Result<Json<Value>, StatusCode> {
    let _ = body; // input ignored in the mock — fixture lookup is the source of truth
    let g = s.data.read().await;
    if !g
        .rules
        .iter()
        .any(|r| r.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
    {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Some(latest) = g
        .verdicts
        .iter()
        .filter(|v| v.get("rule_id").and_then(|x| x.as_str()) == Some(id.as_str()))
        .max_by_key(|v| {
            v.get("at")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()
        })
    {
        let mut clone = latest.clone();
        if let Some(obj) = clone.as_object_mut() {
            obj.insert("id".into(), Value::String(format!("dry-{id}")));
            obj.insert("dry_run".into(), Value::Bool(true));
        }
        return Ok(Json(clone));
    }
    Ok(Json(serde_json::json!({
        "id": format!("dry-{id}"),
        "rule_id": id,
        "dry_run": true,
        "severity": "Healthy",
        "summary": "Mock dry-run: no historical verdicts for this rule.",
    })))
}

// ---------------------------------------------------------------------
// Verdicts
// ---------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
pub struct VerdictFilter {
    pub rule_id: Option<String>,
    pub tag: Option<String>,
    pub severity: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

async fn list_verdicts(
    State(s): State<InsightsState>,
    Query(q): Query<VerdictFilter>,
) -> Json<Value> {
    let g = s.data.read().await;
    let out: Vec<Value> = g
        .verdicts
        .iter()
        .filter(|v| {
            if let Some(r) = &q.rule_id {
                if v.get("rule_id").and_then(|x| x.as_str()) != Some(r.as_str()) {
                    return false;
                }
            }
            if let Some(sev) = &q.severity {
                if v.get("severity").and_then(|x| x.as_str()) != Some(sev.as_str()) {
                    return false;
                }
            }
            if let Some(tag) = &q.tag {
                let tags = v.get("tags").and_then(|x| x.as_array());
                if !tags.is_some_and(|arr| arr.iter().any(|t| t.as_str() == Some(tag.as_str()))) {
                    return false;
                }
            }
            if let Some(since) = &q.since {
                if v.get("at").and_then(|x| x.as_str()).unwrap_or("") < since.as_str() {
                    return false;
                }
            }
            if let Some(until) = &q.until {
                if v.get("at").and_then(|x| x.as_str()).unwrap_or("") > until.as_str() {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();
    Json(Value::Array(out))
}

async fn get_verdict(
    State(s): State<InsightsState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let g = s.data.read().await;
    g.verdicts
        .iter()
        .find(|v| v.get("id").and_then(|x| x.as_str()) == Some(id.as_str()))
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

// ---------------------------------------------------------------------
// Pipelines
// ---------------------------------------------------------------------

async fn list_pipelines(State(s): State<InsightsState>) -> Json<Value> {
    let g = s.data.read().await;
    Json(Value::Array(g.pipelines.clone()))
}

async fn get_pipeline(
    State(s): State<InsightsState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let g = s.data.read().await;
    g.pipelines
        .iter()
        .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Create-or-update by `id`. The spec lists POST for both create and
/// "update graph"; folding them keeps the surface minimal.
async fn upsert_pipeline(
    State(s): State<InsightsState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let mut g = s.data.write().await;
    if let Some(existing) = g
        .pipelines
        .iter_mut()
        .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
    {
        *existing = body.clone();
    } else {
        g.pipelines.push(body.clone());
    }
    let snapshot = g.pipelines.clone();
    g.persist_array("pipelines.json", &snapshot)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(body))
}

// ---------------------------------------------------------------------
// Helpers — coverage + tags
// ---------------------------------------------------------------------

async fn get_coverage(State(s): State<InsightsState>) -> Json<Value> {
    Json(s.data.read().await.coverage.clone())
}

async fn get_tags_index(State(s): State<InsightsState>) -> Json<Value> {
    Json(s.data.read().await.tags_index.clone())
}

// ---------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------

/// Shallow object merge: copy top-level keys from `patch` into `into`
/// (overwriting). Non-object inputs are replaced wholesale.
fn merge_object(into: &mut Value, patch: &Value) {
    match (into.as_object_mut(), patch.as_object()) {
        (Some(target), Some(src)) => {
            for (k, v) in src {
                target.insert(k.clone(), v.clone());
            }
        }
        _ => *into = patch.clone(),
    }
}

/// Resolve the default fixtures dir relative to the workspace.
/// Falls back to `examples/flow-agent/fixtures/insights` from `CARGO_MANIFEST_DIR`
/// or the current working directory.
pub fn default_fixtures_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("INSIGHTS_FIXTURES_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        return PathBuf::from(manifest).join("fixtures/insights");
    }
    PathBuf::from("examples/flow-agent/fixtures/insights")
}

#[derive(Debug, Serialize, Deserialize)]
struct _Unused;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_seeded_fixtures() {
        let dir = default_fixtures_dir();
        let f = InsightsFixtures::load(&dir).expect("load fixtures");
        assert!(!f.rules.is_empty(), "rules.json should have seed data");
        assert!(!f.verdicts.is_empty(), "verdicts.json should have seed data");
        assert!(!f.pipelines.is_empty(), "pipelines.json should have seed data");
    }
}
