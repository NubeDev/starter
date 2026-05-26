//! Warehouse-engine DDL dialect seam.
//!
//! Stage 1 of `rubix/docs/proposal/warehouse-engine-swap.md`
//! extracts every engine-specific SQL string the mart DDL
//! generator produces behind this trait so Stage 2 can add a
//! `TimescaleDbDialect` impl alongside `ClickHouseDialect` without
//! touching the verbs that consume the rendered strings.
//!
//! Stage 1 ships [`ClickHouseDialect`] only and the wired call
//! sites delegate to it; behaviour is byte-identical with the
//! pre-rename code so the existing test suite continues to pass.
//!
//! The trait deliberately returns owned `String`s (rather than
//! borrowing into the spec) because the renderer interpolates
//! the validated identifiers as text. The caller is responsible
//! for validating every identifier through
//! [`super::validate_ident`] before invoking the dialect — the
//! dialect itself trusts its inputs.

use crate::catalog::mart_spec::MartSpec;

use super::mart::{build, DdlError, MartDdl};

/// Engine-specific DDL bodies the warehouse verbs need.
///
/// Stage 1 only exposes the mart-create surface; Stage 2 will
/// extend this trait with retention-policy SQL and continuous-
/// aggregate refresh policies. The shape is left deliberately
/// open-ended so additional methods can land additively.
pub trait DdlDialect: Send + Sync {
    /// Render the full mart DDL bundle (target table + view) for
    /// `spec`. Returns the same [`MartDdl`] shape the verbs already
    /// consume.
    fn mart_create_ddl(&self, spec: &MartSpec) -> Result<MartDdl, DdlError>;
}

/// ClickHouse impl of [`DdlDialect`]. Produces the exact same
/// bytes the historical [`super::mart::build`] emitted.
#[derive(Debug, Default, Clone, Copy)]
pub struct ClickHouseDialect;

impl DdlDialect for ClickHouseDialect {
    fn mart_create_ddl(&self, spec: &MartSpec) -> Result<MartDdl, DdlError> {
        build(spec)
    }
}

/// TimescaleDB impl of [`DdlDialect`]. Renders a continuous
/// aggregate (`CREATE MATERIALIZED VIEW ... WITH
/// (timescaledb.continuous) ... WITH NO DATA`) plus the
/// `add_continuous_aggregate_policy` call that schedules its
/// refresh.
///
/// Stage 2 of `rubix/docs/proposal/warehouse-engine-swap.md`:
///
/// - `tenant_id` is forced into the GROUP BY (ADR-003), in
///   addition to the user-supplied `group_by` columns.
/// - `security_invoker = true` is set on the view so RLS on the
///   underlying hypertable applies to cagg reads.
/// - The refresh policy uses the proposal's standard shape:
///   `start_offset => INTERVAL '3 days'`, `end_offset => INTERVAL
///   '1 minute'`, `schedule_interval => INTERVAL '1 minute'`.
///
/// `MartDdl` is reused as the carrier:
///
/// - `target_name` and `view_name` collapse to the same string —
///   the continuous-aggregate name. There's no separate target
///   table on TimescaleDB; the materialization hypertable is an
///   internal artefact of `CREATE MATERIALIZED VIEW`.
/// - `create_target` carries the `add_continuous_aggregate_policy`
///   call; `create_view` carries the cagg `CREATE` itself. The
///   verb caller is expected to execute `create_view` before
///   `create_target` (a thin departure from the ClickHouse path,
///   where the target table came first).
/// - `drop_target` removes the refresh policy; `drop_view` drops
///   the cagg (which also drops the materialization hypertable —
///   see the proposal §"Constraints and caveats").
#[derive(Debug, Default, Clone, Copy)]
pub struct TimescaleDbDialect;

impl DdlDialect for TimescaleDbDialect {
    fn mart_create_ddl(&self, spec: &MartSpec) -> Result<MartDdl, DdlError> {
        timescale_build(spec)
    }
}

/// Continuous-aggregate body builder. Public for the dialect
/// test-suite and any verb wiring that needs the cagg name shape.
pub fn timescale_build(spec: &MartSpec) -> Result<MartDdl, DdlError> {
    use crate::ddl::validate_ident;

    let mart_name = validate_ident(spec.name.strip_prefix("mart_").unwrap_or(&spec.name))?;
    let source = validate_ident(&spec.source_table)?;
    if spec.group_by.is_empty() {
        return Err(DdlError::EmptyGroupBy);
    }
    let group_by: Vec<&str> = spec
        .group_by
        .iter()
        .map(|s| validate_ident(s))
        .collect::<Result<_, _>>()?;
    for agg in &spec.aggregations {
        validate_ident(&agg.alias)?;
        validate_ident(&agg.col)?;
    }

    let view = format!("mart_{mart_name}");

    // SELECT list. `time_bucket('<n> seconds', ts)` is the
    // Timescale analogue of `toStartOfInterval(ts, INTERVAL <n>
    // SECOND)`. `tenant_id` and every user-supplied `group_by`
    // column are projected from the source hypertable directly
    // (no `tags['…']` lookup — the JSONB tags column is read with
    // standard `->>` if a group_by names a tag key, but the
    // common case is a promoted column).
    let mut select_cols = vec![format!(
        "time_bucket(INTERVAL '{} seconds', ts) AS bucket",
        spec.time_bucket_secs
    )];
    select_cols.push("tenant_id".to_string());
    for g in &group_by {
        select_cols.push((*g).to_string());
    }
    for a in &spec.aggregations {
        let projected = match a.func.as_str() {
            "sum" | "max" | "min" | "avg" | "count" => {
                format!(
                    "{fn_}({col}) AS {alias}",
                    fn_ = a.func,
                    col = a.col,
                    alias = a.alias
                )
            }
            "quantile" => {
                // proposal §"Risks": quantile maps to
                // `percentile_cont`; default to the 0.95
                // percentile so the rendered SQL is valid against
                // a fresh TimescaleDB without `timescaledb_toolkit`
                // loaded. Tuning lives on `MartSpec` in a later
                // patch; this default keeps the dialect total.
                format!(
                    "percentile_cont(0.95) WITHIN GROUP (ORDER BY {col}) AS {alias}",
                    col = a.col,
                    alias = a.alias,
                )
            }
            other => return Err(DdlError::UnsupportedAggregation(other.to_string())),
        };
        select_cols.push(projected);
    }

    // GROUP BY: bucket, tenant_id, <user group_by>. ADR-003
    // requires `tenant_id` in every cagg's GROUP BY.
    let mut group_cols = vec!["bucket".to_string(), "tenant_id".to_string()];
    for g in &group_by {
        group_cols.push((*g).to_string());
    }

    let create_view = format!(
        "CREATE MATERIALIZED VIEW IF NOT EXISTS {view}\n\
         WITH (timescaledb.continuous, timescaledb.materialized_only = false, \
         timescaledb.security_invoker = true) AS\n\
         SELECT {cols}\n\
         FROM {source}\n\
         GROUP BY {group_by}\n\
         WITH NO DATA;",
        cols = select_cols.join(", "),
        group_by = group_cols.join(", "),
    );

    let create_target = format!(
        "SELECT add_continuous_aggregate_policy('{view}', \
         start_offset => INTERVAL '3 days', \
         end_offset => INTERVAL '1 minute', \
         schedule_interval => INTERVAL '1 minute', \
         if_not_exists => TRUE);",
    );

    let drop_view = format!("DROP MATERIALIZED VIEW IF EXISTS {view} CASCADE");
    let drop_target =
        format!("SELECT remove_continuous_aggregate_policy('{view}', if_exists => TRUE)");

    Ok(MartDdl {
        // The cagg fills both roles — there's no separate target
        // table the way ClickHouse's `AggregatingMergeTree`
        // requires. Keep both names equal so any caller iterating
        // both fields produces consistent identifiers.
        target_name: view.clone(),
        view_name: view,
        create_target,
        create_view,
        drop_target,
        drop_view,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::mart_spec::{AggregationSpec, MartSpec};

    fn sample_spec() -> MartSpec {
        MartSpec {
            name: "mart_energy_hourly".into(),
            description: None,
            source_table: "samples".into(),
            filter: serde_json::json!({}),
            time_bucket_secs: 3600,
            group_by: vec!["building".into(), "tenant".into()],
            aggregations: vec![AggregationSpec {
                func: "sum".into(),
                col: "value_num".into(),
                alias: "kwh".into(),
            }],
            created_by: "user:alice".into(),
            ext_manifest_hash: None,
        }
    }

    #[test]
    fn clickhouse_dialect_matches_legacy_build_byte_for_byte() {
        let spec = sample_spec();
        let legacy = build(&spec).unwrap();
        let via_trait = ClickHouseDialect.mart_create_ddl(&spec).unwrap();
        assert_eq!(legacy.create_target, via_trait.create_target);
        assert_eq!(legacy.create_view, via_trait.create_view);
        assert_eq!(legacy.drop_target, via_trait.drop_target);
        assert_eq!(legacy.drop_view, via_trait.drop_view);
    }

    #[test]
    fn timescale_dialect_emits_continuous_aggregate() {
        let ddl = TimescaleDbDialect.mart_create_ddl(&sample_spec()).unwrap();
        assert!(ddl
            .create_view
            .contains("CREATE MATERIALIZED VIEW IF NOT EXISTS mart_energy_hourly"));
        assert!(ddl.create_view.contains("timescaledb.continuous"));
        assert!(ddl.create_view.contains("security_invoker = true"));
        assert!(ddl.create_view.contains("WITH NO DATA"));
        assert!(ddl
            .create_view
            .contains("time_bucket(INTERVAL '3600 seconds', ts) AS bucket"));
        assert!(ddl.create_view.contains("sum(value_num) AS kwh"));
        // ADR-003: tenant_id is in every cagg's GROUP BY.
        assert!(ddl
            .create_view
            .contains("GROUP BY bucket, tenant_id, building, tenant"));
    }

    #[test]
    fn timescale_dialect_renders_refresh_policy() {
        let ddl = TimescaleDbDialect.mart_create_ddl(&sample_spec()).unwrap();
        assert!(ddl
            .create_target
            .contains("add_continuous_aggregate_policy"));
        assert!(ddl
            .create_target
            .contains("start_offset => INTERVAL '3 days'"));
        assert!(ddl
            .create_target
            .contains("end_offset => INTERVAL '1 minute'"));
        assert!(ddl
            .create_target
            .contains("schedule_interval => INTERVAL '1 minute'"));
    }

    #[test]
    fn timescale_dialect_drops_view_and_policy() {
        let ddl = TimescaleDbDialect.mart_create_ddl(&sample_spec()).unwrap();
        assert!(ddl.drop_view.contains("DROP MATERIALIZED VIEW"));
        assert!(ddl
            .drop_target
            .contains("remove_continuous_aggregate_policy"));
    }
}
