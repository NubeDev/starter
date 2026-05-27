//! L1 → L2 cleaner tick.
//!
//! A "tick" is one pass that reads a fresh window of L1 rows from
//! `samples`, applies the [`RuleRegistry`] per `(tenant_id,
//! entity_id)`, and bulk-inserts the resulting rows into the
//! `samples_l2` hypertable. The tick is the seam between the
//! pure rule trait (sync, infallible) and the async I/O surface
//! (`sqlx::PgPool`) so it splits in two:
//!
//! - [`process_entity_window`] — pure: takes ordered readings,
//!   produces ordered `L2Row`s. Unit-tested without a database.
//! - [`run_tick`] — async I/O: SELECT a window from `samples`,
//!   group by `(tenant_id, entity_id)`, call `process_entity_window`
//!   per group, then bulk-insert into `samples_l2`.
//!
//! ## Window shape
//!
//! [`TickParams`] picks two intervals:
//!
//! - `[from_ts_ms, to_ts_ms)` is the **fresh** window — rows in this
//!   range are evaluated and written to L2.
//! - `[from_ts_ms - history_lookback_ms, from_ts_ms)` is the
//!   **history** window — rows in this range are loaded to give
//!   `SpikeRule` / `StuckRule` context, but they are not re-written.
//!
//! The history lookback exists because [`SpikeRule`] and
//! [`StuckRule`] need preceding readings to fire on the first row of
//! a fresh window. Without it, the first row of every tick would
//! always evaluate as `Ok` regardless of what came before.
//!
//! ## Dropped rows
//!
//! [`RuleOutcome::Drop`] is honoured by skipping the row in the
//! emitted `Vec<L2Row>`; the drop count surfaces on [`TickStats`].

use std::collections::HashMap;

use serde::Serialize;
use sqlx::Row as _;
use starter_store_warehouse::WarehouseClient;

use super::registry::RuleRegistry;
use super::rule::{QualityTag, Reading, RuleOutcome, WindowSlice};

/// Parameters for one cleaner tick.
///
/// Times are epoch milliseconds. `to_ts_ms` is exclusive; the
/// caller typically picks `to_ts_ms = now()` and `from_ts_ms =
/// last_tick_to_ts_ms` so successive ticks tile without overlap.
#[derive(Debug, Clone)]
pub struct TickParams {
    /// Inclusive lower bound of the fresh window.
    pub from_ts_ms: i64,
    /// Exclusive upper bound of the fresh window.
    pub to_ts_ms: i64,
    /// History lookback, in ms. Rows in `[from - lookback, from)`
    /// are loaded as context for the first-row case of rules that
    /// look at preceding readings, but are not re-emitted.
    pub history_lookback_ms: i64,
}

impl TickParams {
    /// Convenience: a `[from, to)` window with a default 30-minute
    /// history lookback (matches the synth producer's typical
    /// stuck-stretch length).
    pub fn new(from_ts_ms: i64, to_ts_ms: i64) -> Self {
        Self {
            from_ts_ms,
            to_ts_ms,
            history_lookback_ms: 30 * 60 * 1000,
        }
    }
}

/// One row destined for `samples_l2`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct L2Row {
    pub tenant_id: String,
    pub entity_id: String,
    pub ts_ms: i64,
    pub value: Option<f64>,
    pub quality: QualityTag,
    /// `Some(rule_id)` when a rule flagged the row; `None` when
    /// every rule returned `Ok` (quality = `ok`).
    pub rule_id: Option<&'static str>,
    /// JSONB payload. Currently carries `{ "<rule_id>": "<note>" }`
    /// when the flagging rule supplied a note; otherwise an empty
    /// object.
    pub tags: serde_json::Value,
}

/// Stats produced by one tick. Cheap to serialize — the tool
/// wrapper in B4 returns this verbatim.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct TickStats {
    /// Rows in the fresh window read from `samples`.
    pub rows_read: u64,
    /// Rows written to `samples_l2`.
    pub rows_written: u64,
    /// Rows dropped per [`RuleOutcome::Drop`].
    pub rows_dropped: u64,
    /// Per-quality breakdown of the written rows. Stringified for
    /// JSON friendliness (key matches [`QualityTag::as_str`]).
    pub by_quality: HashMap<String, u64>,
    /// Distinct `(tenant_id, entity_id)` pairs touched.
    pub entities_scanned: u64,
}

impl TickStats {
    fn record(&mut self, row: &L2Row) {
        self.rows_written += 1;
        *self
            .by_quality
            .entry(row.quality.as_str().to_owned())
            .or_insert(0) += 1;
    }
}

/// Pure window walker for one `(tenant_id, entity_id)`.
///
/// `history` and `fresh` are both expected to be chronologically
/// ascending. `history` rows are not emitted — they exist only to
/// seed the rule window. As the walker advances through `fresh`,
/// previously-processed fresh rows extend the window so later rules
/// (e.g. `StuckRule`) see the running tail.
pub fn process_entity_window(
    registry: &RuleRegistry,
    history: &[Reading],
    fresh: &[Reading],
) -> (Vec<L2Row>, u64) {
    let mut out = Vec::with_capacity(fresh.len());
    let mut dropped = 0u64;
    // Mutable rolling window: starts at the history slice and
    // grows with each fresh row processed.
    let mut window: Vec<Reading> = history.to_vec();
    for row in fresh {
        let (rule_id, outcome) = registry.apply_all(row, WindowSlice::new(&window));
        match outcome {
            RuleOutcome::Drop => {
                dropped += 1;
                // Dropped rows still extend the window: downstream
                // rules deserve to see them as historical context.
                window.push(row.clone());
                continue;
            }
            RuleOutcome::Ok => {
                out.push(L2Row {
                    tenant_id: row.tenant_id.clone(),
                    entity_id: row.entity_id.clone(),
                    ts_ms: row.ts_ms,
                    value: row.value,
                    quality: QualityTag::Ok,
                    rule_id: None,
                    tags: serde_json::json!({}),
                });
            }
            RuleOutcome::Flag { quality, note } => {
                let tags = match (&rule_id, &note) {
                    (Some(id), Some(n)) => serde_json::json!({ *id: n }),
                    _ => serde_json::json!({}),
                };
                out.push(L2Row {
                    tenant_id: row.tenant_id.clone(),
                    entity_id: row.entity_id.clone(),
                    ts_ms: row.ts_ms,
                    value: row.value,
                    quality,
                    rule_id,
                    tags,
                });
            }
        }
        window.push(row.clone());
    }
    (out, dropped)
}

/// Run one cleaner tick end-to-end.
///
/// Reads the fresh + history window from `samples`, applies the
/// registry per entity, bulk-inserts the resulting rows into
/// `samples_l2`. Returns aggregate stats — no rows are returned
/// to the caller (the canonical record lives in the L2 hypertable).
pub async fn run_tick(
    client: &WarehouseClient,
    registry: &RuleRegistry,
    params: &TickParams,
) -> Result<TickStats, sqlx::Error> {
    let history_from_ms = params
        .from_ts_ms
        .saturating_sub(params.history_lookback_ms);

    // Single SELECT covers history + fresh; we split per row by
    // comparing ts_ms against `from_ts_ms`. Ordering by
    // (tenant_id, entity_id, ts) lets us group sequentially.
    let rows = sqlx::query(
        "SELECT tenant_id, entity_id, \
                (EXTRACT(EPOCH FROM ts) * 1000)::BIGINT AS ts_ms, \
                value_num, quality \
         FROM samples \
         WHERE ts >= to_timestamp($1::double precision / 1000.0) \
           AND ts <  to_timestamp($2::double precision / 1000.0) \
         ORDER BY tenant_id, entity_id, ts",
    )
    .bind(history_from_ms as f64)
    .bind(params.to_ts_ms as f64)
    .fetch_all(client.pool())
    .await?;

    let mut stats = TickStats::default();
    let mut l2_rows: Vec<L2Row> = Vec::new();

    let mut current_key: Option<(String, String)> = None;
    let mut history: Vec<Reading> = Vec::new();
    let mut fresh: Vec<Reading> = Vec::new();

    for row in rows {
        let tenant_id: String = row.get("tenant_id");
        let entity_id: String = row.get("entity_id");
        let ts_ms: i64 = row.get("ts_ms");
        let value: Option<f64> = row.try_get("value_num").ok();
        let source_quality: i16 = row.try_get("quality").unwrap_or(0);

        let key = (tenant_id.clone(), entity_id.clone());
        if Some(&key) != current_key.as_ref() {
            if current_key.is_some() {
                let (emitted, dropped) = process_entity_window(registry, &history, &fresh);
                stats.rows_read += fresh.len() as u64;
                stats.rows_dropped += dropped;
                for r in &emitted {
                    stats.record(r);
                }
                l2_rows.extend(emitted);
                stats.entities_scanned += 1;
            }
            current_key = Some(key);
            history.clear();
            fresh.clear();
        }

        let reading = Reading {
            tenant_id,
            entity_id,
            ts_ms,
            value,
            source_quality,
        };
        if ts_ms < params.from_ts_ms {
            history.push(reading);
        } else {
            fresh.push(reading);
        }
    }
    if current_key.is_some() {
        let (emitted, dropped) = process_entity_window(registry, &history, &fresh);
        stats.rows_read += fresh.len() as u64;
        stats.rows_dropped += dropped;
        for r in &emitted {
            stats.record(r);
        }
        l2_rows.extend(emitted);
        stats.entities_scanned += 1;
    }

    if !l2_rows.is_empty() {
        bulk_insert_l2(client, &l2_rows).await?;
    }

    Ok(stats)
}

/// Bulk-insert `samples_l2` rows. Uses a single multi-row
/// `INSERT ... VALUES ($1,$2,...), (...)` per chunk to keep
/// round-trips down without pulling in `COPY` complexity.
async fn bulk_insert_l2(client: &WarehouseClient, rows: &[L2Row]) -> Result<(), sqlx::Error> {
    // Cap per-statement parameter count well under Postgres' 65535
    // limit. 7 columns × 4000 rows = 28000 params; comfortable.
    const CHUNK: usize = 4000;
    for chunk in rows.chunks(CHUNK) {
        let mut sql = String::from(
            "INSERT INTO samples_l2 \
             (tenant_id, entity_id, ts, value_num, quality, rule_id, tags) VALUES ",
        );
        let mut first = true;
        for i in 0..chunk.len() {
            if !first {
                sql.push_str(", ");
            }
            first = false;
            let base = i * 7;
            sql.push_str(&format!(
                "(${}, ${}, to_timestamp(${}::double precision / 1000.0), ${}, ${}, ${}, ${})",
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5,
                base + 6,
                base + 7,
            ));
        }
        let mut q = sqlx::query(&sql);
        for r in chunk {
            q = q
                .bind(&r.tenant_id)
                .bind(&r.entity_id)
                .bind(r.ts_ms as f64)
                .bind(r.value)
                .bind(r.quality.as_str())
                .bind(r.rule_id)
                .bind(&r.tags);
        }
        q.execute(client.pool()).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(entity: &str, ts_ms: i64, value: Option<f64>) -> Reading {
        Reading {
            tenant_id: "t".into(),
            entity_id: entity.into(),
            ts_ms,
            value,
            source_quality: 0,
        }
    }

    #[test]
    fn empty_fresh_emits_nothing() {
        let reg = RuleRegistry::builtin();
        let (rows, dropped) = process_entity_window(&reg, &[], &[]);
        assert!(rows.is_empty());
        assert_eq!(dropped, 0);
    }

    #[test]
    fn ok_rows_pass_through_with_quality_ok() {
        let reg = RuleRegistry::builtin();
        let fresh = vec![r("e", 1, Some(1.0)), r("e", 2, Some(1.1))];
        let (rows, _) = process_entity_window(&reg, &[], &fresh);
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert_eq!(row.quality, QualityTag::Ok);
            assert!(row.rule_id.is_none());
        }
    }

    #[test]
    fn nan_row_is_flagged() {
        let reg = RuleRegistry::builtin();
        let fresh = vec![r("e", 1, Some(f64::NAN))];
        let (rows, _) = process_entity_window(&reg, &[], &fresh);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].quality, QualityTag::Nan);
        assert_eq!(rows[0].rule_id, Some("builtin.nan"));
        // Tags carry the rule note.
        assert!(rows[0].tags.get("builtin.nan").is_some());
    }

    #[test]
    fn history_seeds_spike_on_first_fresh_row() {
        let reg = RuleRegistry::builtin();
        let history = vec![r("e", 1, Some(10.0))];
        let fresh = vec![r("e", 2, Some(500.0))]; // 50× — over default 10×
        let (rows, _) = process_entity_window(&reg, &history, &fresh);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].quality, QualityTag::Spike);
        assert_eq!(rows[0].rule_id, Some("builtin.spike"));
    }

    #[test]
    fn rolling_window_lets_stuck_fire_within_fresh() {
        let reg = RuleRegistry::builtin();
        // No history. Four equal fresh rows: first three pass as Ok,
        // the fourth triggers StuckRule via the running tail.
        let fresh = vec![
            r("e", 1, Some(5.0)),
            r("e", 2, Some(5.0)),
            r("e", 3, Some(5.0)),
            r("e", 4, Some(5.0)),
        ];
        let (rows, _) = process_entity_window(&reg, &[], &fresh);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].quality, QualityTag::Ok);
        assert_eq!(rows[1].quality, QualityTag::Ok);
        assert_eq!(rows[2].quality, QualityTag::Ok);
        assert_eq!(rows[3].quality, QualityTag::Stuck);
        assert_eq!(rows[3].rule_id, Some("builtin.stuck"));
    }

    #[test]
    fn drop_outcome_skips_emission_but_extends_window() {
        // Custom rule that drops any negative value. Place after
        // builtins so first-non-Ok ordering doesn't preempt it on
        // benign rows.
        #[derive(Debug)]
        struct DropNegatives;
        impl super::super::AnomalyRule for DropNegatives {
            fn id(&self) -> &'static str {
                "test.drop_neg"
            }
            fn apply(&self, row: &Reading, _w: WindowSlice<'_>) -> RuleOutcome {
                match row.value {
                    Some(v) if v < 0.0 => RuleOutcome::Drop,
                    _ => RuleOutcome::Ok,
                }
            }
        }
        let reg = RuleRegistry::new().add(std::sync::Arc::new(DropNegatives));
        let fresh = vec![
            r("e", 1, Some(1.0)),
            r("e", 2, Some(-1.0)),
            r("e", 3, Some(2.0)),
        ];
        let (rows, dropped) = process_entity_window(&reg, &[], &fresh);
        assert_eq!(dropped, 1);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ts_ms, 1);
        assert_eq!(rows[1].ts_ms, 3);
    }

    #[test]
    fn tick_params_default_lookback_is_thirty_minutes() {
        let p = TickParams::new(1000, 2000);
        assert_eq!(p.history_lookback_ms, 30 * 60 * 1000);
    }

    #[test]
    fn empty_registry_marks_every_row_ok() {
        let reg = RuleRegistry::new();
        let fresh = vec![r("e", 1, Some(f64::NAN)), r("e", 2, Some(1.0))];
        let (rows, _) = process_entity_window(&reg, &[], &fresh);
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert_eq!(row.quality, QualityTag::Ok);
            assert!(row.rule_id.is_none());
        }
    }
}
