//! v3 — unified `WarehouseWriter` chokepoint.
//!
//! Every warehouse-write surface (the four `tsdb/store/*.rs` insert
//! paths and rubix-agent's `RubixWarehouseWriteBackend::insert`)
//! goes through one trait + impl. On `commit`, the chokepoint
//! deduplicates the per-row tag set it accumulated and fires a
//! **single** [`Invalidator::invalidate_tags`] call covering:
//!
//! - `table:<name>` for every distinct table touched in the batch,
//! - `bucket:<table>:<floor(t, granularity)>` for every distinct
//!   (table, bucket) the rows landed in,
//! - `table:<name>:<dim>=<value>` for every (table, dimension, value)
//!   the registered [`BucketTagSpec`] declared.
//!
//! Per §Layer 3 "Batched ingest coalesces tag emissions" a 500-row
//! batch spanning 12 buckets fires *one* `invalidate_tags` call with
//! the deduped tag set, not 500. The dedup is done at the
//! chokepoint, not the invalidator — so a write path that has no
//! cache wired (developer rig, single-tenant CLI) pays nothing.
//!
//! The chokepoint is the v3 precondition: it replaces the scattered
//! `// TODO(cache-invalidation):` markers from v0 with one trait the
//! type system enforces. Write paths take `&dyn WarehouseWriter`,
//! never poke the invalidator directly.

use crate::invalidator::Invalidator;
use crate::spec::BucketTagSpec;
use chrono::{DateTime, Duration, Utc};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Registry of `(table, granularity)` declarations supplied by the
/// host at boot time. The chokepoint walks the registry once per
/// commit to pre-derive the `bucket:<table>:<key>` tags from each
/// row's timestamp.
#[derive(Debug, Default, Clone)]
pub struct WriterTagRegistry {
    /// `table -> Vec<BucketTagSpec>`. One table may carry multiple
    /// declared granularities if more than one cache spec subscribes.
    by_table: BTreeMap<String, Vec<BucketTagSpec>>,
}

impl WriterTagRegistry {
    /// Build an empty registry.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build from an iterator of [`BucketTagSpec`]s — typically the
    /// output of walking every registered cache spec's
    /// `invalidate_on.buckets`.
    pub fn from_specs<I>(specs: I) -> Self
    where
        I: IntoIterator<Item = BucketTagSpec>,
    {
        let mut by_table: BTreeMap<String, Vec<BucketTagSpec>> = BTreeMap::new();
        for s in specs {
            by_table.entry(s.table.clone()).or_default().push(s);
        }
        Self { by_table }
    }

    /// Look up the declared specs for a table.
    pub fn for_table(&self, table: &str) -> &[BucketTagSpec] {
        self.by_table
            .get(table)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// One row's bookkeeping for tag emission. Write paths convert their
/// engine-specific row type to this on the way in.
#[derive(Debug, Clone)]
pub struct WriteRow {
    /// Fully-qualified table name (e.g.
    /// `"com_nubeio_rubixos__histories"`).
    pub table: String,
    /// Row timestamp, used for bucket-floor calculation. `None`
    /// signals "no time-series — only `table:<name>` is fired".
    pub ts: Option<DateTime<Utc>>,
    /// Dimensional values from the row, keyed by column name. The
    /// chokepoint only fires `table:<t>:<dim>=<value>` for columns
    /// the registry declared in [`BucketTagSpec::dimensions`].
    pub dimensions: BTreeMap<String, String>,
}

impl WriteRow {
    /// Convenience constructor — table-only (no time, no dims).
    pub fn table_only(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            ts: None,
            dimensions: BTreeMap::new(),
        }
    }
}

/// Trait every warehouse-write path goes through.
///
/// The contract: the path enqueues every row with [`Self::enqueue`]
/// inside the active transaction, then calls [`Self::commit`] on
/// success. On commit the chokepoint dedups the accumulated row
/// set into a tag list and fires one `invalidate_tags`. On rollback
/// callers drop the writer without committing; nothing fires.
#[async_trait::async_trait]
pub trait WarehouseWriter: Send + Sync {
    /// Add one row to the batch.
    async fn enqueue(&self, row: WriteRow);
    /// Add many rows. Default impl just loops; backends may
    /// override for efficiency.
    async fn enqueue_many(&self, rows: Vec<WriteRow>) {
        for r in rows {
            self.enqueue(r).await;
        }
    }
    /// Fire one deduped `invalidate_tags` call and drain the batch.
    /// Idempotent — calling `commit` on an empty writer fires nothing.
    async fn commit(&self);
    /// Drop the batch without firing. Used on transaction rollback.
    async fn discard(&self);
    /// Snapshot of the currently accumulated tag set (test/observability).
    fn pending_tags(&self) -> Vec<String>;
}

/// Default chokepoint impl. Holds the in-flight batch in a
/// `Mutex<BatchState>` keyed by the writer's lifetime; one writer per
/// transaction.
pub struct DefaultWarehouseWriter {
    invalidator: Arc<dyn Invalidator>,
    registry: WriterTagRegistry,
    state: tokio::sync::Mutex<BatchState>,
}

#[derive(Default)]
struct BatchState {
    rows: Vec<WriteRow>,
}

impl DefaultWarehouseWriter {
    /// Build a writer with the given invalidator + registry.
    pub fn new(invalidator: Arc<dyn Invalidator>, registry: WriterTagRegistry) -> Self {
        Self {
            invalidator,
            registry,
            state: tokio::sync::Mutex::new(BatchState::default()),
        }
    }

    /// Derive the deduped tag set from an accumulated row vec. Pulled
    /// out so tests can exercise the dedup logic without a real
    /// invalidator.
    pub fn derive_tags(registry: &WriterTagRegistry, rows: &[WriteRow]) -> Vec<String> {
        let mut out: BTreeSet<String> = BTreeSet::new();
        for r in rows {
            out.insert(format!("table:{}", r.table));
            let specs = registry.for_table(&r.table);
            for s in specs {
                // bucket:<table>:<floor(t, gran)>
                if let Some(ts) = r.ts {
                    if let Some(gran) = parse_granularity(&s.granularity) {
                        let floored = floor_to(ts, gran);
                        out.insert(format!(
                            "bucket:{}:{}",
                            r.table,
                            floored.format("%Y-%m-%dT%H:%M:%SZ")
                        ));
                    }
                }
                // table:<table>:<dim>=<value>
                for dim in &s.dimensions {
                    if let Some(v) = r.dimensions.get(dim) {
                        out.insert(format!("table:{}:{}={}", r.table, dim, v));
                    }
                }
            }
        }
        out.into_iter().collect()
    }
}

#[async_trait::async_trait]
impl WarehouseWriter for DefaultWarehouseWriter {
    async fn enqueue(&self, row: WriteRow) {
        self.state.lock().await.rows.push(row);
    }

    async fn enqueue_many(&self, rows: Vec<WriteRow>) {
        self.state.lock().await.rows.extend(rows);
    }

    async fn commit(&self) {
        let rows = std::mem::take(&mut self.state.lock().await.rows);
        if rows.is_empty() {
            return;
        }
        let tags = Self::derive_tags(&self.registry, &rows);
        if !tags.is_empty() {
            self.invalidator.invalidate_tags(&tags).await;
        }
    }

    async fn discard(&self) {
        self.state.lock().await.rows.clear();
    }

    fn pending_tags(&self) -> Vec<String> {
        // best-effort sync snapshot; use try_lock so this stays sync.
        match self.state.try_lock() {
            Ok(g) => Self::derive_tags(&self.registry, &g.rows),
            Err(_) => Vec::new(),
        }
    }
}

fn parse_granularity(s: &str) -> Option<Duration> {
    let s = s.trim();
    let (num, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit())?);
    let n: i64 = num.parse().ok()?;
    Some(match unit.trim() {
        "s" | "sec" | "secs" | "seconds" => Duration::seconds(n),
        "m" | "min" | "mins" | "minutes" => Duration::minutes(n),
        "h" | "hr" | "hrs" | "hours" => Duration::hours(n),
        "d" | "day" | "days" => Duration::days(n),
        _ => return None,
    })
}

fn floor_to(ts: DateTime<Utc>, gran: Duration) -> DateTime<Utc> {
    let secs = gran.num_seconds().max(1);
    let unix = ts.timestamp();
    let floored = unix - (unix.rem_euclid(secs));
    DateTime::from_timestamp(floored, 0).unwrap_or(ts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invalidator::InMemoryInvalidator;
    use chrono::TimeZone;

    fn row_at(table: &str, t: &str) -> WriteRow {
        WriteRow {
            table: table.into(),
            ts: Some(DateTime::parse_from_rfc3339(t).unwrap().with_timezone(&Utc)),
            dimensions: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn commit_fires_deduped_tags_once() {
        let inv = Arc::new(InMemoryInvalidator::new());
        let reg = WriterTagRegistry::from_specs([BucketTagSpec {
            table: "readings".into(),
            granularity: "1h".into(),
            dimensions: vec![],
        }]);
        let w = DefaultWarehouseWriter::new(inv.clone(), reg);
        // 500 rows across 12 buckets.
        for i in 0..500 {
            let hour = (i / 42) % 12; // 12 distinct buckets
            let ts = Utc
                .with_ymd_and_hms(2026, 5, 29, hour as u32, 7, 0)
                .unwrap();
            w.enqueue(WriteRow {
                table: "readings".into(),
                ts: Some(ts),
                dimensions: BTreeMap::new(),
            })
            .await;
        }
        w.commit().await;
        let fired = inv.fired_tags();
        // The invalidator records each tag in the single fire call.
        // 1 table tag + 12 bucket tags = 13 unique. fired_tags pushes
        // them all in one fire-call list.
        let unique: BTreeSet<&String> = fired.iter().collect();
        assert_eq!(unique.len(), 13, "expected 1 table + 12 buckets: {fired:?}");
    }

    #[tokio::test]
    async fn dimension_scoped_tags_emit_when_declared() {
        let inv = Arc::new(InMemoryInvalidator::new());
        let reg = WriterTagRegistry::from_specs([BucketTagSpec {
            table: "readings".into(),
            granularity: "1h".into(),
            dimensions: vec!["meter".into()],
        }]);
        let w = DefaultWarehouseWriter::new(inv.clone(), reg);
        let mut dims = BTreeMap::new();
        dims.insert("meter".into(), "42".into());
        w.enqueue(WriteRow {
            table: "readings".into(),
            ts: Some(Utc.with_ymd_and_hms(2026, 5, 29, 12, 0, 0).unwrap()),
            dimensions: dims,
        })
        .await;
        w.commit().await;
        let fired: BTreeSet<String> = inv.fired_tags().into_iter().collect();
        assert!(fired.contains("table:readings"));
        assert!(fired.contains("table:readings:meter=42"));
        assert!(
            fired.iter().any(|t| t.starts_with("bucket:readings:")),
            "{fired:?}"
        );
    }

    #[tokio::test]
    async fn discard_drops_pending_without_firing() {
        let inv = Arc::new(InMemoryInvalidator::new());
        let w = DefaultWarehouseWriter::new(inv.clone(), WriterTagRegistry::empty());
        w.enqueue(row_at("readings", "2026-05-29T12:00:00Z")).await;
        w.discard().await;
        w.commit().await;
        assert!(inv.fired_tags().is_empty());
    }

    #[test]
    fn floor_to_aligns_to_hour() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 29, 12, 47, 13).unwrap();
        let f = floor_to(ts, Duration::hours(1));
        assert_eq!(
            f.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "2026-05-29T12:00:00Z"
        );
    }
}
