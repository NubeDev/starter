//! `MartSpec` — the user-facing shape behind `mart.define` /
//! `POST /api/marts`. Persists into `marts` via the
//! `starter_store_postgres::dimensions::marts` typed CRUD, and
//! feeds the `ddl::mart` DDL generator.
//!
//! `definition_hash` (W5) is SHA-256 over the canonical JSON
//! serialisation of `(filter, time_bucket_secs, group_by,
//! aggregations)`. The hash is the change-detector: identical
//! spec ⇒ idempotent no-op; different hash ⇒ hard error per the
//! W5 mart-redefine rule.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MartSpec {
    pub name: String,
    pub description: Option<String>,
    pub source_table: String,
    pub filter: serde_json::Value,
    /// Seconds; converted to `INTERVAL '<n> seconds'` for Postgres
    /// and `toStartOfInterval(<col>, INTERVAL <n> SECOND)` for CH.
    pub time_bucket_secs: i64,
    pub group_by: Vec<String>,
    pub aggregations: Vec<AggregationSpec>,
    /// Author identity per W12 (`user:…`, `agent:…`, `ext:<id>`).
    pub created_by: String,
    /// Manifest hash for `ext:` authors (W12). Ignored for
    /// `user:`/`agent:` rows.
    pub ext_manifest_hash: Option<String>,
}

/// One aggregation entry. `fn` is the CH aggregate function
/// (`sum`, `max`, `avg`, `count`, `min`, `quantile`); `col` is the
/// `samples` column the aggregate reads; `as` is the column name
/// in the mart target (and the corresponding `*_state`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregationSpec {
    #[serde(rename = "fn")]
    pub func: String,
    pub col: String,
    #[serde(rename = "as")]
    pub alias: String,
}

impl MartSpec {
    /// `(filter, time_bucket, group_by, aggregations)` — exactly
    /// what W5 covers.
    pub fn definition_hash(&self) -> String {
        let canonical = serde_json::json!({
            "filter": self.filter,
            "time_bucket_secs": self.time_bucket_secs,
            "group_by": self.group_by,
            "aggregations": self.aggregations,
        });
        let bytes = serde_json::to_vec(&canonical).expect("canonical JSON is always serialisable");
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    /// Promoted-column set for W14 validation: every entry in
    /// `group_by` plus every aggregation `alias`.
    pub fn promoted_columns(&self) -> Vec<String> {
        let mut out = self.group_by.clone();
        for a in &self.aggregations {
            out.push(a.alias.clone());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> MartSpec {
        MartSpec {
            name: "mart_energy_hourly".into(),
            description: None,
            source_table: "samples".into(),
            filter: serde_json::json!({"tags": {"kind": "energy"}}),
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
    fn definition_hash_is_stable() {
        let a = fixture().definition_hash();
        let b = fixture().definition_hash();
        assert_eq!(a, b);
    }

    #[test]
    fn promoted_columns_include_group_by_and_aliases() {
        let cols = fixture().promoted_columns();
        assert_eq!(cols, vec!["building", "tenant", "kwh"]);
    }
}
