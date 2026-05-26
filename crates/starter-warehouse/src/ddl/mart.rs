//! Mart DDL generator (W5).
//!
//! From a [`crate::catalog::mart_spec::MartSpec`] this module emits
//! two DDL statements: a `mart_<name>_state` target table backed
//! by `AggregatingMergeTree`, and an incremental MV
//! `mart_<name>` that reads from the source table.
//!
//! ORDER BY rule (D7): `(<first group_by>, bucket, <rest>)` so the
//! primary dimension prunes granules before the time range scan.

use super::{validate_ident, IdentError};
use crate::catalog::mart_spec::{AggregationSpec, MartSpec};

#[derive(Debug, thiserror::Error)]
pub enum DdlError {
    #[error(transparent)]
    Ident(#[from] IdentError),
    #[error("aggregation {0:?} not supported by W5 DDL")]
    UnsupportedAggregation(String),
    #[error("mart spec missing group_by — at least one column is required (D7)")]
    EmptyGroupBy,
}

/// Generated DDL bundle.
pub struct MartDdl {
    pub target_name: String,
    pub view_name: String,
    pub create_target: String,
    pub create_view: String,
    pub drop_target: String,
    pub drop_view: String,
}

/// Build the DDL for a mart spec.
pub fn build(spec: &MartSpec) -> Result<MartDdl, DdlError> {
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
        agg_state_type(agg)?;
    }

    let target = format!("mart_{mart_name}_state");
    let view = format!("mart_{mart_name}");

    // ORDER BY: (<first group_by>, bucket, <rest>) per W5/D7.
    let mut order = vec![group_by[0].to_string(), "bucket".to_string()];
    for g in &group_by[1..] {
        order.push(g.to_string());
    }

    let mut state_cols = String::new();
    state_cols.push_str("  bucket     DateTime,\n");
    for g in &group_by {
        state_cols.push_str(&format!("  {g} String,\n"));
    }
    for (i, a) in spec.aggregations.iter().enumerate() {
        let comma = if i + 1 == spec.aggregations.len() {
            ""
        } else {
            ","
        };
        state_cols.push_str(&format!(
            "  {alias}_state {sty}{comma}\n",
            alias = a.alias,
            sty = agg_state_type(a)?,
        ));
    }
    let create_target = format!(
        "CREATE TABLE IF NOT EXISTS {target} (\n\
         {cols}\
         ) ENGINE = AggregatingMergeTree\n\
         PARTITION BY toYYYYMM(bucket)\n\
         ORDER BY ({order});",
        cols = state_cols,
        order = order.join(", "),
    );

    let mut select_cols = vec![format!(
        "toStartOfInterval(ts, INTERVAL {} SECOND) AS bucket",
        spec.time_bucket_secs
    )];
    for g in &group_by {
        select_cols.push(format!("tags['{g}'] AS {g}"));
    }
    for a in &spec.aggregations {
        select_cols.push(format!(
            "{fn_}State({col}) AS {alias}_state",
            fn_ = a.func,
            col = a.col,
            alias = a.alias
        ));
    }
    let mut group_cols = vec!["bucket".to_string()];
    for g in &group_by {
        group_cols.push((*g).to_string());
    }
    let create_view = format!(
        "CREATE MATERIALIZED VIEW IF NOT EXISTS {view}\n\
         TO {target} AS\n\
         SELECT {cols}\n\
         FROM {source}\n\
         GROUP BY {group_by};",
        cols = select_cols.join(", "),
        group_by = group_cols.join(", "),
    );

    Ok(MartDdl {
        drop_target: format!("DROP TABLE IF EXISTS {target}"),
        drop_view: format!("DROP VIEW IF EXISTS {view}"),
        target_name: target,
        view_name: view,
        create_target,
        create_view,
    })
}

/// Map a `MartSpec`'s aggregation `fn` onto the CH
/// `AggregateFunction(<fn>, <argtype>)` state type. Only the
/// W5-sanctioned aggregates are supported; anything else is a
/// typed error rather than a silently mangled DDL.
fn agg_state_type(a: &AggregationSpec) -> Result<String, DdlError> {
    let argty = match a.col.as_str() {
        "value_num" => "Float64",
        "value_bool" => "UInt8",
        _ => "Float64",
    };
    Ok(match a.func.as_str() {
        "sum" | "max" | "min" | "avg" => {
            format!("AggregateFunction({}, {})", a.func, argty)
        }
        "count" => "AggregateFunction(count, UInt64)".to_string(),
        _ => return Err(DdlError::UnsupportedAggregation(a.func.clone())),
    })
}

/// Helper used by `mart.read` to assemble the SELECT for a mart
/// target. Emits `*Merge()` over each aggregation alias and joins
/// to `entities_dict` via `dictGetOrNull` per W13. The `range`
/// + caller-supplied `filter` are bound separately by the caller —
/// this returns the bare query text + the column list.
pub fn read_query(spec: &MartSpec, extra_where: &str, hide_unknown: bool) -> String {
    let mart_name = spec.name.strip_prefix("mart_").unwrap_or(&spec.name);
    let target = format!("mart_{mart_name}_state");
    let mut select = vec!["bucket".to_string()];
    for g in &spec.group_by {
        select.push(g.clone());
    }
    for a in &spec.aggregations {
        select.push(format!("{}Merge({}_state) AS {}", a.func, a.alias, a.alias));
    }
    // Promote the leading group_by into a `display` join via
    // `dictGetOrNull` for the W13 "[unknown entity]" rendering.
    let display_col = format!(
        "dictGetOrNull('entities_dict', 'display', {}) AS display_for_{}",
        spec.group_by[0], spec.group_by[0]
    );
    select.push(display_col);
    let mut group = vec!["bucket".to_string()];
    for g in &spec.group_by {
        group.push(g.clone());
    }
    let mut where_clause = String::from("bucket >= ? AND bucket < ?");
    if !extra_where.is_empty() {
        where_clause.push_str(" AND ");
        where_clause.push_str(extra_where);
    }
    if hide_unknown {
        where_clause.push_str(&format!(
            " AND isNotNull(dictGetOrNull('entities_dict','display',{}))",
            spec.group_by[0]
        ));
    }
    format!(
        "SELECT {select} FROM {target} WHERE {where_clause} GROUP BY {group} ORDER BY bucket",
        select = select.join(", "),
        group = group.join(", "),
    )
}

/// TimescaleDB twin of [`read_query`]. The cagg materialises the
/// aggregates directly (no `*Merge(*_state)` wrap is needed
/// because TimescaleDB stores the post-aggregation values, not
/// CH-style intermediate state). The leading `group_by` column is
/// joined against the `entities` dimension table in the same
/// database — the proposal's §"What is replaced" line on the
/// `entities_dict` dictionary going away.
///
/// The query uses Postgres `$1` / `$2` placeholders for the time
/// range; `extra_where` is appended verbatim (callers are
/// expected to have validated and parameterised any user input
/// upstream — same contract as the CH variant).
pub fn read_query_pg(spec: &MartSpec, extra_where: &str, hide_unknown: bool) -> String {
    let mart_name = spec.name.strip_prefix("mart_").unwrap_or(&spec.name);
    let view = format!("mart_{mart_name}");
    let lead = &spec.group_by[0];
    let mut select = vec![format!("m.bucket")];
    select.push("m.tenant_id".to_string());
    for g in &spec.group_by {
        select.push(format!("m.{g}"));
    }
    for a in &spec.aggregations {
        select.push(format!("m.{}", a.alias));
    }
    // Direct JOIN against the dimension table — replaces the old
    // `dictGetOrNull('entities_dict', …)` lookups. The join key
    // is the leading group_by column (the same role the CH
    // `display_for_<lead>` projection served).
    select.push(format!("d.display AS display_for_{lead}"));
    let join = format!(
        "LEFT JOIN entities AS d \
         ON d.id = m.{lead} AND d.tenant_id = m.tenant_id"
    );
    let mut where_clause = String::from("m.bucket >= $1 AND m.bucket < $2");
    if !extra_where.is_empty() {
        where_clause.push_str(" AND ");
        where_clause.push_str(extra_where);
    }
    if hide_unknown {
        where_clause.push_str(" AND d.display IS NOT NULL");
    }
    format!(
        "SELECT {select} FROM {view} AS m {join} WHERE {where_clause} ORDER BY m.bucket",
        select = select.join(", "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> MartSpec {
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
    fn order_by_promotes_first_group_by() {
        let ddl = build(&spec()).unwrap();
        assert!(ddl
            .create_target
            .contains("ORDER BY (building, bucket, tenant)"));
    }

    #[test]
    fn validates_identifiers() {
        let mut s = spec();
        s.group_by = vec!["valid".into(), "BAD-NAME".into()];
        assert!(build(&s).is_err());
    }

    #[test]
    fn read_query_pg_uses_direct_join_no_dict() {
        let q = read_query_pg(&spec(), "", false);
        assert!(!q.contains("entities_dict"));
        assert!(!q.contains("dictGet"));
        assert!(q.contains("LEFT JOIN entities AS d"));
        assert!(q.contains("d.display AS display_for_building"));
        assert!(q.contains("m.bucket >= $1 AND m.bucket < $2"));
    }

    #[test]
    fn read_query_pg_hide_unknown_filters_null_display() {
        let q = read_query_pg(&spec(), "", true);
        assert!(q.contains("d.display IS NOT NULL"));
    }

    #[test]
    fn rejects_empty_group_by() {
        let mut s = spec();
        s.group_by.clear();
        assert!(matches!(build(&s), Err(DdlError::EmptyGroupBy)));
    }
}
