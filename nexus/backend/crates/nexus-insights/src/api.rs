//! The curated `Frame` surface registered into the Rhai engine.
//!
//! This is the *only* thing a script can do to data: call a method on a frame
//! handle, getting a new frame back. Scripts chain
//! `df.resample(time, "1 hour", [...]).zscore("kw").anomalies("kw", 3.0)`.
//! Each method is a thin shim onto an engine primitive; the engine guarantees no
//! primitive increases the row count, so the surface is explosion-proof. Errors
//! from a primitive (unknown column, bad argument) become Rhai runtime errors the
//! run layer maps to a tenant-safe [`crate::InsightError::Runtime`].

use rhai::{Array, Engine, EvalAltResult, Map};

use crate::engine::{Agg, FilterValue, Frame};
use crate::error::InsightError;

/// Register the `Frame` type and every primitive onto `engine`. Called once per
/// execution after the sandbox limits are set.
pub fn register(engine: &mut Engine) {
    engine.register_type_with_name::<Frame>("Frame");

    engine.register_fn("columns", |df: &mut Frame| -> Array {
        df.columns().into_iter().map(Into::into).collect()
    });
    engine.register_fn("row_count", |df: &mut Frame| df.row_count() as i64);

    engine.register_fn("select", |df: &mut Frame, cols: Array| {
        let cols = string_array(cols)?;
        ok(df.select(&cols))
    });
    engine.register_fn("rename", |df: &mut Frame, from: &str, to: &str| {
        ok(df.rename(from, to))
    });

    register_filters(engine);
    register_windows(engine);
    register_shape(engine);

    engine.register_fn("resample", |df: &mut Frame, time_col: &str, every: &str, aggs: Array| {
        let aggs = parse_aggs(aggs)?;
        ok(df.resample(time_col, every, &aggs))
    });
    engine.register_fn("anomalies", |df: &mut Frame, col: &str, z: f64| {
        ok(df.anomalies(col, z))
    });
}

/// Filters take either a number or a string bound; Rhai dispatches on the arg
/// type so a script writes `filter_gt("kw", 10.0)` or `filter_eq("site", "A")`.
fn register_filters(engine: &mut Engine) {
    engine.register_fn("filter_gt", |df: &mut Frame, c: &str, v: f64| ok(df.filter_gt(c, FilterValue::Num(v))));
    engine.register_fn("filter_lt", |df: &mut Frame, c: &str, v: f64| ok(df.filter_lt(c, FilterValue::Num(v))));
    engine.register_fn("filter_eq", |df: &mut Frame, c: &str, v: f64| ok(df.filter_eq(c, FilterValue::Num(v))));
    engine.register_fn("filter_eq", |df: &mut Frame, c: &str, v: &str| {
        ok(df.filter_eq(c, FilterValue::Str(v.to_string())))
    });
    // Integer literals are common in scripts; accept them without forcing a cast.
    engine.register_fn("filter_gt", |df: &mut Frame, c: &str, v: i64| ok(df.filter_gt(c, FilterValue::Num(v as f64))));
    engine.register_fn("filter_lt", |df: &mut Frame, c: &str, v: i64| ok(df.filter_lt(c, FilterValue::Num(v as f64))));
    engine.register_fn("filter_eq", |df: &mut Frame, c: &str, v: i64| ok(df.filter_eq(c, FilterValue::Num(v as f64))));
}

fn register_windows(engine: &mut Engine) {
    engine.register_fn("rolling_mean", |df: &mut Frame, c: &str, w: i64| ok(df.rolling_mean(c, w)));
    engine.register_fn("rolling_min", |df: &mut Frame, c: &str, w: i64| ok(df.rolling_min(c, w)));
    engine.register_fn("rolling_max", |df: &mut Frame, c: &str, w: i64| ok(df.rolling_max(c, w)));
    engine.register_fn("rolling_sum", |df: &mut Frame, c: &str, w: i64| ok(df.rolling_sum(c, w)));
    engine.register_fn("lag", |df: &mut Frame, c: &str, n: i64| ok(df.lag(c, n)));
    engine.register_fn("diff", |df: &mut Frame, c: &str| ok(df.diff(c)));
    engine.register_fn("pct_change", |df: &mut Frame, c: &str| ok(df.pct_change(c)));
    engine.register_fn("zscore", |df: &mut Frame, c: &str| ok(df.zscore(c)));
}

fn register_shape(engine: &mut Engine) {
    engine.register_fn("head", |df: &mut Frame, n: i64| ok(df.head(n)));
    engine.register_fn("tail", |df: &mut Frame, n: i64| ok(df.tail(n)));
    engine.register_fn("sort", |df: &mut Frame, c: &str, asc: bool| ok(df.sort(c, asc)));
    engine.register_fn("fill_null", |df: &mut Frame, c: &str, s: &str| ok(df.fill_null(c, s)));
    engine.register_fn("describe", |df: &mut Frame| ok(df.describe()));
}

/// Map an insight result into a Rhai return, turning an engine/runtime error into
/// a Rhai runtime error string the run layer re-classifies.
fn ok(result: Result<Frame, InsightError>) -> Result<Frame, Box<EvalAltResult>> {
    result.map_err(|e| e.to_string().into())
}

/// Coerce a Rhai array of strings into a `Vec<String>`, rejecting a non-string
/// element so the failure names the cause.
fn string_array(arr: Array) -> Result<Vec<String>, Box<EvalAltResult>> {
    arr.into_iter()
        .map(|v| {
            v.into_string()
                .map_err(|_| "expected an array of column-name strings".into())
        })
        .collect()
}

/// Parse the resample aggregate spec: an array of `#{ col: "...", func: "..." }`
/// maps. A malformed entry is a script error.
fn parse_aggs(arr: Array) -> Result<Vec<Agg>, Box<EvalAltResult>> {
    arr.into_iter()
        .map(|v| {
            let map: Map = v
                .try_cast()
                .ok_or("each resample aggregate must be a map")?;
            let col = map_str(&map, "col")?;
            let func = map_str(&map, "func")?;
            Ok(Agg { col, func })
        })
        .collect()
}

/// Read a required string field from a Rhai map.
fn map_str(map: &Map, key: &str) -> Result<String, Box<EvalAltResult>> {
    map.get(key)
        .and_then(|v| v.clone().into_string().ok())
        .ok_or_else(|| format!("aggregate map missing string field {key:?}").into())
}
