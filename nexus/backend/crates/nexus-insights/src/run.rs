//! `run_insight` — compile and run one tenant script over an input frame.
//!
//! The script sees two bound variables: `df`, the input [`Frame`], and `params`,
//! the caller's parameters as a Rhai object map. Its final expression must
//! evaluate to a `Frame`, which becomes the result. The whole evaluation runs
//! inside `spawn_blocking` because the engine primitives `block_on` DataFusion
//! and Rhai itself is synchronous; a current Tokio runtime must therefore exist
//! when this is called (it does — it runs inside an async handler).
//!
//! Errors are classified for the tenant: a parse failure is `Compile`, a tripped
//! sandbox bound is `LimitExceeded`, anything else the script raised is `Runtime`.

use std::time::Instant;

use rhai::{Dynamic, Engine, EvalAltResult, Scope};
use serde_json::Value;

use crate::engine::{batches_to_rows, rows_to_frame, Frame};
use crate::error::{InsightError, InsightResult};
use crate::limits::Limits;
use crate::{api, sandbox};

/// Run `script` over JSON `rows` with `params`, returning the transformed rows.
/// This is the host-facing entry the query stage and the CRUD preview call: it
/// converts rows → frame, runs the script under the default sandbox limits, and
/// converts the resulting frame → rows.
pub async fn run_insight_rows(
    script: String,
    rows: Vec<Value>,
    params: Value,
) -> InsightResult<Vec<Value>> {
    run_insight_rows_with_limits(script, rows, params, Limits::default()).await
}

/// As [`run_insight_rows`] but with caller-chosen sandbox `limits`. The query
/// stage uses the defaults; this lets a deployment (or a test) pick tighter or
/// looser bounds without re-implementing the rows↔frame conversion.
pub async fn run_insight_rows_with_limits(
    script: String,
    rows: Vec<Value>,
    params: Value,
    limits: Limits,
) -> InsightResult<Vec<Value>> {
    let frame = run_insight(script, rows_to_frame(&rows)?, params, limits).await?;
    batches_to_rows(frame.batches())
}

/// Run `script` over `input` with `params` under `limits`, returning the result
/// frame. Exposed for callers that already hold a [`Frame`] (tests, future
/// in-engine stages) and want to keep the Arrow result.
pub async fn run_insight(
    script: String,
    input: Frame,
    params: Value,
    limits: Limits,
) -> InsightResult<Frame> {
    let params = json_to_dynamic(&params);
    tokio::task::spawn_blocking(move || evaluate(&script, input, params, limits))
        .await
        .map_err(|e| InsightError::Engine(format!("insight task join: {e}")))?
}

/// The blocking body: build the sandbox, register the surface, compile, and run.
fn evaluate(script: &str, input: Frame, params: Dynamic, limits: Limits) -> InsightResult<Frame> {
    let started = Instant::now();
    let mut engine = sandbox::build(&limits, started);
    api::register(&mut engine);

    let ast = engine
        .compile(script)
        .map_err(|e| InsightError::Compile(e.to_string()))?;

    let mut scope = Scope::new();
    scope.push_constant("df", input);
    scope.push_constant("params", params);

    let frame: Frame = engine
        .eval_ast_with_scope(&mut scope, &ast)
        .map_err(|e| classify(*e))?;
    Ok(frame)
}

/// Re-classify a Rhai eval error into the tenant-facing variant: a sandbox bound
/// (operations, depth, data size, or the wall-clock termination) is a limit
/// breach; everything else is a runtime error.
fn classify(err: EvalAltResult) -> InsightError {
    if let EvalAltResult::ErrorTerminated(token, _) = &err {
        if token.clone().into_string().as_deref() == Ok(sandbox::WALL_CLOCK_TOKEN) {
            return InsightError::LimitExceeded("wall-clock deadline exceeded".into());
        }
    }
    if sandbox::is_limit_error(&err) {
        return InsightError::LimitExceeded(err.to_string());
    }
    InsightError::Runtime(err.to_string())
}

/// Convert a JSON params value into a Rhai `Dynamic` so a script reads
/// `params.threshold` naturally. Uses Rhai's own JSON parser for object inputs;
/// a non-object (or absent) params value becomes an empty map so `params.x` is a
/// clean unit rather than a type error.
fn json_to_dynamic(params: &Value) -> Dynamic {
    if params.is_object() {
        // A scratch engine just for parsing the params literal — it touches no
        // data and shares no state with the sandboxed run.
        let engine = Engine::new_raw();
        if let Ok(map) = engine.parse_json(&params.to_string(), true) {
            return map.into();
        }
    }
    rhai::Map::new().into()
}
