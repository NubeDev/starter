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
}
