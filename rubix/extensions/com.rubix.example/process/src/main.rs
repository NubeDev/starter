//! Reference rubix extension — process flavour.
//!
//! Demonstrates the four extension-north-star surfaces:
//!
//!   1. **MCP-style tools** (`echo`, `csv_ingest`, `customer_quality`)
//!      via `contributes.tools[]` + the generated
//!      `*ToolHandlers` trait.
//!   2. **Warehouse-table contributions** — the `csv_ingest` handler
//!      goes through `ctx.warehouse_write()` and lands rows in the
//!      two tables declared in `contributes.warehouse_tables[]`. The
//!      host stamps `tenant_id` from `ctx.caller()` before issuing
//!      the INSERT so the extension cannot spoof cross-tenant writes.
//!   3. **Warehouse-read templates** — two named templates (top
//!      countries, low-stock products) declared via
//!      `contributes.warehouse_templates[]`. The host's
//!      `TemplateRegistry` loads them at boot; the SQL bodies live
//!      next to this file under `kinds/*.sql` and are captured
//!      verbatim into the registry for audit (R7: never templated
//!      at runtime).
//!   4. **Anomaly-rule contribution** — `contributes.anomaly_rules[]`
//!      points the cleaner at this extension's
//!      `customer_quality` tool; the host wraps the dispatch in a
//!      `ToolAnomalyRule` adapter and appends it after the three
//!      in-process builtins (NaN → Spike → Stuck).
//!
//! Per SCOPE R8 this binary depends ONLY on `starter-ext-sdk`. The
//! handler bodies are pure functions of `(ctx, params)` and would be
//! byte-identical in a builtin/wasm twin — only the entry-point macro
//! (`register_process_main!`) is flavour-specific.

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::{Extension, Row};

/// The extension's unit struct. SCOPE R5: no fields. All state lives
/// in the host-provided Ctx.
#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct Example;

starter_ext_sdk::requires! {
    name = ExampleCtx,
    capabilities = [warehouse_read, warehouse_write, tracing],
}

impl ExampleToolHandlers for Example {
    type Ctx = ExampleCtx;

    /// `com.rubix.example.echo` — return the input verbatim. Kept as
    /// the lowest-friction smoke probe.
    fn handle_com_rubix_example_echo(
        &self,
        _ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        Ok(params)
    }

    /// `com.rubix.example.csv_ingest` — drop a batch of
    /// datablist-shaped rows into one of the extension-owned
    /// warehouse tables.
    ///
    /// Wire shape:
    ///
    /// ```json
    /// { "dataset": "customers"|"products",
    ///   "rows": [ { "<column>": <value>, ... }, ... ] }
    /// ```
    ///
    /// The host:
    ///
    ///  - Refuses calls without a tenant-scoped caller
    ///    (`Error::Capability`).
    ///  - Resolves `dataset` against
    ///    `capabilities.warehouse_write.tables` — a dataset not in
    ///    the grant refuses at the supervisor before this handler
    ///    even runs.
    ///  - Stamps `tenant_id` onto every row, overriding any value
    ///    the caller tried to set.
    ///  - Validates each row's columns against the
    ///    `contributes.warehouse_tables[].columns[]` schema —
    ///    unknown columns refuse with `Error::Validation`.
    ///
    /// Returns `{ "dataset": ..., "inserted": <u64> }`.
    fn handle_com_rubix_example_csv_ingest(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let dataset = params
            .get("dataset")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                starter_ext_sdk::Error::Validation(
                    "csv_ingest: `dataset` (string) is required".into(),
                )
            })?
            .to_owned();

        // Allowlist mirrors `capabilities.warehouse_write.tables`.
        // The supervisor enforces the same allowlist; this is a
        // friendlier error surface for misconfigured callers.
        if dataset != "customers" && dataset != "products" {
            return Err(starter_ext_sdk::Error::Validation(format!(
                "csv_ingest: unknown dataset `{dataset}` \
                 (expected `customers` or `products`)"
            )));
        }

        let raw_rows = params
            .get("rows")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                starter_ext_sdk::Error::Validation(
                    "csv_ingest: `rows` (array of objects) is required".into(),
                )
            })?;

        if raw_rows.is_empty() {
            return Err(starter_ext_sdk::Error::Validation(
                "csv_ingest: `rows` must not be empty".into(),
            ));
        }

        let rows: Vec<Row> = raw_rows
            .iter()
            .map(|v| match v {
                Value::Object(m) => Ok(Row::from_map(m.clone())),
                _ => Err(starter_ext_sdk::Error::Validation(
                    "csv_ingest: every row must be a JSON object".into(),
                )),
            })
            .collect::<starter_ext_sdk::Result<_>>()?;

        let inserted = ctx.warehouse_write().insert(&dataset, rows)?;

        Ok(json!({ "dataset": dataset, "inserted": inserted }))
    }

    /// `com.rubix.example.customer_quality` — per-row data-quality
    /// detector wired into the cleaner via
    /// `contributes.anomaly_rules[]`.
    ///
    /// Wire shape (cleaner-side, see
    /// `rubix-tools/src/cleaner/adapter.rs`):
    ///
    /// ```json
    /// { "row": { ... }, "window_tail": [ ... ] }
    /// ```
    ///
    /// Response is decoded as a `RuleOutcome`:
    ///
    /// ```json
    /// { "outcome": "ok" }
    /// { "outcome": "flag", "quality": "MissingEmail", "note": "customer_id=..." }
    /// { "outcome": "drop" }
    /// ```
    ///
    /// A misbehaving rule (bad shape, panic, error) is downgraded
    /// to `ok` by the host adapter so an extension cannot silently
    /// flag rows.
    fn handle_com_rubix_example_customer_quality(
        &self,
        _ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let row = params
            .get("row")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                starter_ext_sdk::Error::Validation(
                    "customer_quality: `row` (object) is required".into(),
                )
            })?;

        Ok(evaluate_customer_quality(row))
    }

    /// `com.rubix.example.warehouse_query` — thin proxy over
    /// `ctx.warehouse_read().query(template, params)` for the
    /// bundled UI panel.
    ///
    /// The browser cannot reach `WarehouseReadHandle` directly, so
    /// the UI hits `/api/v1/tools/com.rubix.example.warehouse_query`
    /// instead and the host's tool dispatcher routes it here.
    ///
    /// This handler refuses anything outside this extension's own
    /// `com.rubix.example.*` template namespace — the host would
    /// also reject foreign templates via the grant gate, but the
    /// pre-check keeps the error surface friendly. Per R7 the SQL
    /// body is never templated here: the host's resolver matches
    /// by name and runs the corresponding `sqlx::query_as`.
    fn handle_com_rubix_example_warehouse_query(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let template = params
            .get("template")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                starter_ext_sdk::Error::Validation(
                    "warehouse_query: `template` (string) is required".into(),
                )
            })?
            .to_owned();

        if !template.starts_with("com.rubix.example.") {
            return Err(starter_ext_sdk::Error::Validation(format!(
                "warehouse_query: template `{template}` is outside this \
                 extension's namespace (`com.rubix.example.*`)"
            )));
        }

        let tpl_params = params
            .get("params")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let rows = ctx.warehouse_read().query(&template, tpl_params)?;
        let rows_json: Vec<Value> = rows
            .into_iter()
            .map(|r| Value::Object(r.0))
            .collect();
        let count = rows_json.len();

        Ok(json!({
            "template": template,
            "rows": rows_json,
            "count": count,
        }))
    }

    /// `com.rubix.example.products_create` — INSERT one product
    /// row. Wire shape: `{ "row": { "internal_id": ..., ... } }`.
    fn handle_com_rubix_example_products_create(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let row = take_row(&params, "products_create")?;
        let affected = ctx
            .warehouse_write()
            .insert("products", vec![Row::from_map(row)])?;
        Ok(json!({ "operation": "create", "affected": affected }))
    }

    /// `com.rubix.example.products_update` — UPDATE one product
    /// row matched by `internal_id`.
    fn handle_com_rubix_example_products_update(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let row = take_row(&params, "products_update")?;
        let has_key = row
            .get("internal_id")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty());
        if !has_key {
            return Err(starter_ext_sdk::Error::Validation(
                "products_update: `row.internal_id` (non-empty string) is required".into(),
            ));
        }
        let affected = ctx
            .warehouse_write()
            .update("products", "internal_id", vec![Row::from_map(row)])?;
        Ok(json!({ "operation": "update", "affected": affected }))
    }

    /// `com.rubix.example.products_delete` — DELETE one or more
    /// rows by `internal_id`.
    fn handle_com_rubix_example_products_delete(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let ids = params
            .get("internal_ids")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                starter_ext_sdk::Error::Validation(
                    "products_delete: `internal_ids` (array of strings) is required".into(),
                )
            })?;
        if ids.is_empty() {
            return Err(starter_ext_sdk::Error::Validation(
                "products_delete: `internal_ids` must not be empty".into(),
            ));
        }
        let keys: Vec<Value> = ids
            .iter()
            .map(|v| match v.as_str() {
                Some(s) if !s.is_empty() => Ok(Value::String(s.to_owned())),
                _ => Err(starter_ext_sdk::Error::Validation(
                    "products_delete: every entry must be a non-empty string".into(),
                )),
            })
            .collect::<starter_ext_sdk::Result<_>>()?;
        let affected = ctx
            .warehouse_write()
            .delete("products", "internal_id", keys)?;
        Ok(json!({ "operation": "delete", "affected": affected }))
    }
}

/// Extract `params["row"]` as an owned JSON object map for the
/// products CRUD handlers.
fn take_row(params: &Value, tool: &str) -> starter_ext_sdk::Result<Map<String, Value>> {
    params
        .get("row")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| {
            starter_ext_sdk::Error::Validation(format!("{tool}: `row` (object) is required"))
        })
}

/// Pure rule body. Split out so it can be unit-tested without an
/// SDK Ctx.
fn evaluate_customer_quality(row: &Map<String, Value>) -> Value {
    let str_field = |k: &str| -> Option<&str> { row.get(k).and_then(Value::as_str) };

    // 1. Missing country.
    match str_field("country") {
        None | Some("") => {
            return flag(
                "MissingCountry",
                &format!(
                    "customer_id={}",
                    str_field("customer_id").unwrap_or("<unknown>")
                ),
            );
        }
        _ => {}
    }

    // 2. Email rules.
    match str_field("email") {
        None | Some("") => {
            return flag(
                "MissingEmail",
                &format!(
                    "customer_id={}",
                    str_field("customer_id").unwrap_or("<unknown>")
                ),
            );
        }
        Some(e) if !e.contains('@') => {
            return flag("InvalidEmail", &format!("email={e}"));
        }
        _ => {}
    }

    // 3. Subscription date sanity. Permits ISO `YYYY-MM-DD` strings
    //    in the [2000-01-01, today] window. Bad/unknown values are
    //    treated as `BadDate` flags rather than silently ignored —
    //    extensions catching schema drift is the whole point of a
    //    per-row rule.
    if let Some(d) = str_field("subscription_date") {
        if !is_plausible_iso_date(d) {
            return flag("BadDate", &format!("subscription_date={d}"));
        }
    }

    json!({ "outcome": "ok" })
}

fn flag(quality: &str, note: &str) -> Value {
    json!({ "outcome": "flag", "quality": quality, "note": note })
}

/// Crude plausibility check for `YYYY-MM-DD`. We deliberately avoid
/// pulling `chrono`/`time` into a reference extension — schema drift
/// detection only needs to refuse obviously-wrong strings.
fn is_plausible_iso_date(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    let bytes = s.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let all_digits = |range: std::ops::Range<usize>| -> bool {
        s[range].bytes().all(|b| b.is_ascii_digit())
    };
    if !(all_digits(0..4) && all_digits(5..7) && all_digits(8..10)) {
        return false;
    }
    let year: u32 = s[0..4].parse().unwrap_or(0);
    let month: u32 = s[5..7].parse().unwrap_or(0);
    let day: u32 = s[8..10].parse().unwrap_or(0);
    // Loose bounds — schema drift, not actuarial precision.
    (2000..=2100).contains(&year)
        && (1..=12).contains(&month)
        && (1..=31).contains(&day)
}

// Emits `pub async fn run() -> starter_ext_sdk::Result<()>` driving
// the stdio JSON-RPC loop that the rubix-agent supervisor speaks to.
starter_ext_sdk::register_process_main! {
    extension: Example,
    ctx: ExampleCtx,
    instance: Example,
}

// SDK's process loop calls `tokio::task::block_in_place` inside
// the dispatcher (see starter-ext-sdk/src/process.rs), which only
// works on the multi-threaded runtime. Use `multi_thread` with a
// minimal worker pool — the extension's actual work is still
// single-tool-at-a-time.
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            // Supervisor's stderr-forwarder pushes this into the
            // per-extension event ring.
            eprintln!("rubix-example-extension exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_sdk::serde_json::json;

    fn row(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn customer_quality_ok_when_all_fields_present() {
        let r = row(json!({
            "customer_id": "DD37Cf93aecA6Dc",
            "country": "Chile",
            "email": "zunigavanessa@smith.info",
            "subscription_date": "2020-08-24",
        }));
        assert_eq!(evaluate_customer_quality(&r), json!({ "outcome": "ok" }));
    }

    #[test]
    fn customer_quality_flags_missing_country_first() {
        let r = row(json!({
            "customer_id": "X1",
            "country": "",
            "email": "noemail",
        }));
        let out = evaluate_customer_quality(&r);
        assert_eq!(out["outcome"], "flag");
        assert_eq!(out["quality"], "MissingCountry");
    }

    #[test]
    fn customer_quality_flags_missing_email() {
        let r = row(json!({
            "customer_id": "X2",
            "country": "Chile",
        }));
        let out = evaluate_customer_quality(&r);
        assert_eq!(out["quality"], "MissingEmail");
    }

    #[test]
    fn customer_quality_flags_invalid_email() {
        let r = row(json!({
            "customer_id": "X3",
            "country": "Chile",
            "email": "not-an-email",
        }));
        let out = evaluate_customer_quality(&r);
        assert_eq!(out["quality"], "InvalidEmail");
        assert_eq!(out["note"], "email=not-an-email");
    }

    #[test]
    fn customer_quality_flags_bad_date() {
        let r = row(json!({
            "customer_id": "X4",
            "country": "Chile",
            "email": "a@b.co",
            "subscription_date": "1899-99-99",
        }));
        let out = evaluate_customer_quality(&r);
        assert_eq!(out["quality"], "BadDate");
    }

    #[test]
    fn iso_date_parser_accepts_plausible_strings() {
        assert!(is_plausible_iso_date("2020-08-24"));
        assert!(is_plausible_iso_date("2099-12-31"));
        assert!(!is_plausible_iso_date("1899-01-01"));
        assert!(!is_plausible_iso_date("2020-13-01"));
        assert!(!is_plausible_iso_date("2020-08-32"));
        assert!(!is_plausible_iso_date("not-a-date"));
        assert!(!is_plausible_iso_date("2020/08/24"));
    }
}
