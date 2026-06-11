//! `GET /api/v1/insights/functions` — the curated insight function catalog.
//!
//! LAYER: transport (REST). This is a static catalog: no tenant, no DB. It lists
//! every function the Rhai insight sandbox exposes so the workbench can render a
//! cheatsheet and drive autocomplete that never drifts from the engine.
//!
//! The catalog is authored to match `nexus_insights`'s curated op registry
//! (`api::register` and friends). Each entry mirrors an actually-registered
//! function and its real argument types. When the registry gains or changes a
//! function, update this list in lockstep.

use axum::Json;
use nexus_spi::dto::insight::{InsightFunctionCatalog, InsightFunctionDoc};

/// One catalog entry, kept terse so the const table reads like the registry it
/// shadows.
fn doc(
    name: &str,
    signature: &str,
    summary: &str,
    category: &str,
    example: &str,
) -> InsightFunctionDoc {
    InsightFunctionDoc {
        name: name.to_string(),
        signature: signature.to_string(),
        summary: summary.to_string(),
        category: category.to_string(),
        example: example.to_string(),
    }
}

/// Build the curated catalog. Ordered by category so the cheatsheet groups
/// naturally: select → filter → window → shape → resample → anomaly.
fn catalog() -> Vec<InsightFunctionDoc> {
    vec![
        // --- select -------------------------------------------------------
        doc(
            "columns",
            "columns() -> array",
            "List the frame's column names.",
            "select",
            "columns()",
        ),
        doc(
            "row_count",
            "row_count() -> int",
            "The number of rows in the frame.",
            "select",
            "row_count()",
        ),
        doc(
            "select",
            "select(cols: array)",
            "Keep only the named columns, in the given order.",
            "select",
            r#"select(["time", "value"])"#,
        ),
        doc(
            "rename",
            "rename(from: string, to: string)",
            "Rename a single column.",
            "select",
            r#"rename("kw", "power")"#,
        ),
        // --- filter -------------------------------------------------------
        doc(
            "filter_gt",
            "filter_gt(col: string, value: number)",
            "Keep rows where the column is greater than the value.",
            "filter",
            r#"filter_gt("kw", 10.0)"#,
        ),
        doc(
            "filter_lt",
            "filter_lt(col: string, value: number)",
            "Keep rows where the column is less than the value.",
            "filter",
            r#"filter_lt("kw", 100.0)"#,
        ),
        doc(
            "filter_eq",
            "filter_eq(col: string, value: number | string)",
            "Keep rows where the column equals the value (numeric or string).",
            "filter",
            r#"filter_eq("site", "A")"#,
        ),
        // --- window -------------------------------------------------------
        doc(
            "rolling_mean",
            "rolling_mean(col: string, window: int)",
            "Add a rolling mean of the column over a window of rows.",
            "window",
            r#"rolling_mean("value", 5)"#,
        ),
        doc(
            "rolling_min",
            "rolling_min(col: string, window: int)",
            "Add a rolling minimum of the column over a window of rows.",
            "window",
            r#"rolling_min("value", 5)"#,
        ),
        doc(
            "rolling_max",
            "rolling_max(col: string, window: int)",
            "Add a rolling maximum of the column over a window of rows.",
            "window",
            r#"rolling_max("value", 5)"#,
        ),
        doc(
            "rolling_sum",
            "rolling_sum(col: string, window: int)",
            "Add a rolling sum of the column over a window of rows.",
            "window",
            r#"rolling_sum("value", 5)"#,
        ),
        doc(
            "lag",
            "lag(col: string, n: int)",
            "Add the column shifted down by n rows (a lag).",
            "window",
            r#"lag("value", 1)"#,
        ),
        doc(
            "diff",
            "diff(col: string)",
            "Add the row-over-row difference of the column.",
            "window",
            r#"diff("value")"#,
        ),
        doc(
            "pct_change",
            "pct_change(col: string)",
            "Add the row-over-row percentage change of the column.",
            "window",
            r#"pct_change("value")"#,
        ),
        doc(
            "zscore",
            "zscore(col: string)",
            "Add the standard score (z-score) of the column.",
            "window",
            r#"zscore("value")"#,
        ),
        // --- shape --------------------------------------------------------
        doc(
            "head",
            "head(n: int)",
            "Keep the first n rows.",
            "shape",
            "head(10)",
        ),
        doc(
            "tail",
            "tail(n: int)",
            "Keep the last n rows.",
            "shape",
            "tail(10)",
        ),
        doc(
            "sort",
            "sort(col: string, ascending: bool)",
            "Sort the frame by a column, ascending or descending.",
            "shape",
            r#"sort("value", false)"#,
        ),
        doc(
            "fill_null",
            "fill_null(col: string, strategy: string)",
            "Fill nulls in a column using a strategy (e.g. \"forward\", \"zero\").",
            "shape",
            r#"fill_null("value", "forward")"#,
        ),
        doc(
            "describe",
            "describe()",
            "Summary statistics (count, mean, min, max, …) per numeric column.",
            "shape",
            "describe()",
        ),
        // --- resample -----------------------------------------------------
        doc(
            "resample",
            "resample(time_col: string, every: string, aggs: array)",
            "Bucket rows by a time column into fixed intervals, aggregating each \
             bucket. `aggs` is an array of `#{ col, func }` maps.",
            "resample",
            r#"resample("time", "1 hour", [#{ col: "kw", func: "mean" }])"#,
        ),
        // --- anomaly ------------------------------------------------------
        doc(
            "anomalies",
            "anomalies(col: string, z: number)",
            "Flag rows whose column z-score exceeds the threshold as anomalies.",
            "anomaly",
            r#"anomalies("value", 3.0)"#,
        ),
    ]
}

#[utoipa::path(
    get,
    path = "/api/v1/insights/functions",
    tag = "insights",
    operation_id = "list_insight_functions",
    responses(
        (status = 200, description = "The curated insight function catalog", body = InsightFunctionCatalog),
    ),
)]
pub async fn list_functions() -> Json<InsightFunctionCatalog> {
    Json(InsightFunctionCatalog {
        functions: catalog(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_function_is_documented() {
        // Names registered in nexus_insights::api — keep in lockstep with the
        // engine's op registry so the catalog never drifts.
        let registered = [
            "columns",
            "row_count",
            "select",
            "rename",
            "filter_gt",
            "filter_lt",
            "filter_eq",
            "rolling_mean",
            "rolling_min",
            "rolling_max",
            "rolling_sum",
            "lag",
            "diff",
            "pct_change",
            "zscore",
            "head",
            "tail",
            "sort",
            "fill_null",
            "describe",
            "resample",
            "anomalies",
        ];
        let cat = catalog();
        for name in registered {
            assert!(
                cat.iter().any(|d| d.name == name),
                "function `{name}` is registered but missing from the catalog"
            );
        }
        // And no stray entries the engine does not expose.
        for d in &cat {
            assert!(
                registered.contains(&d.name.as_str()),
                "catalog lists `{}` which is not a registered function",
                d.name
            );
        }
    }
}
