//! Phase 4 — D9 Materialisation-SLO smoke.
//!
//! Reproduces the SLO table from DOCS/Insights/SCOPE.md (line 1406):
//!
//! | Read path | p95 target |
//! |---|---|
//! | Verdict list (recent N, single rule)       |  50 ms |
//! | Verdict list (filtered by tag)             | 150 ms |
//! | Rollup timeseries (1 rule, 90 days)        | 100 ms |
//! | Rollup timeseries (tag-grouped, 90 days)   | 250 ms |
//! | Derivation cache fetch (1 rule, 1 window)  |  50 ms |
//!
//! The smoke seeds a synthetic 90-day dataset across the IoT, Energy
//! and HVAC reference rules (one critical, one warn, one healthy
//! shape — `device.online@1`, `energy.usage.baseline-deviation@1`,
//! `hvac.pmv.comfort@1`), rolls them up into both ungrouped and
//! `domain`-tag-grouped buckets, populates the derivation cache with
//! one row per (rule, window), then samples each read-path 50 times
//! and asserts p95 against the SLO column.
//!
//! Per D9 rules 1–3:
//! 1. The smoke runs on the CI worker profile, not a developer
//!    laptop. A regression fails the test with the offending query
//!    plan dumped to stderr; the fix is to tighten the query, never
//!    relax the budget.
//! 2. The budget is enforced as p95; outliers from cold-cache /
//!    OS-noise are absorbed by 50 samples and don't push the
//!    percentile past target unless the underlying query plan
//!    regressed.
//! 3. The asserted numbers ARE the budgets — they're hard-coded
//!    constants below. Changing one requires a SCOPE bump.
//!
//! Gating: requires the `sqlite` feature on `starter-insights`
//! (everything in this test depends on it). `cargo test
//! -p starter-insights --features sqlite --test d9_perf_smoke`
//! runs the suite.
//!
//! Per the stage spec the smoke covers the IoT + Energy + HVAC
//! reference pipelines; finance is exercised separately by the
//! `finance_smoke` test because its rule shapes are window-less
//! point-in-time assertions (per-tx) rather than time-series.

#![cfg(feature = "sqlite")]

use std::time::{Duration, Instant};

use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use starter_insights::cache::DerivationCache;
use starter_insights::rollups::{RollupEngine, WindowClass};
use starter_insights::sqlite::{VerdictStore, INSIGHTS_MIGRATION_SOURCE};
use starter_spi::insights::{
    Coverage, Dataset, DatasetSchema, RuleId, Severity, Tags, TimeZoneId, VecDatasetRows,
    Verdict, Window,
};

// ----- D9 budgets (DOCS/Insights/SCOPE.md line 1406) -----
const SLO_VERDICT_LIST_MS: u128 = 50;
const SLO_VERDICT_LIST_BY_TAG_MS: u128 = 150;
const SLO_ROLLUP_TIMESERIES_MS: u128 = 100;
const SLO_ROLLUP_TAG_GROUPED_MS: u128 = 250;
const SLO_DERIVATION_CACHE_MS: u128 = 50;

const SAMPLES: usize = 50;

/// Three reference rules, one per pack. The smoke uses these
/// (namespace, name, major, domain-tag-value) triples to walk every
/// read path against a representative cross-section of the IoT +
/// Energy + HVAC packs without doubling the seed work.
struct Reference {
    namespace: &'static str,
    name: &'static str,
    major: u32,
    domain: &'static str,
}

const REF_RULES: &[Reference] = &[
    Reference { namespace: "iot",    name: "device.online",             major: 1, domain: "iot"    },
    Reference { namespace: "energy", name: "usage.baseline-deviation",  major: 1, domain: "energy" },
    Reference { namespace: "hvac",   name: "pmv.comfort",               major: 1, domain: "hvac"   },
];

/// p95 of an unsorted vector of durations, expressed in ms.
fn p95_ms(samples: &mut [Duration]) -> u128 {
    samples.sort();
    // 0-indexed; ceil(0.95 * n) - 1.
    let idx = ((samples.len() as f64) * 0.95).ceil() as usize;
    let idx = idx.saturating_sub(1).min(samples.len() - 1);
    samples[idx].as_millis()
}

/// Build a single verdict for `(rule, when, severity, domain)`,
/// tagged with `domain:<value>` so the tag-grouped rollup has
/// something non-trivial to aggregate.
fn make_verdict(
    namespace: &str,
    name: &str,
    major: u32,
    when: chrono::DateTime<Utc>,
    severity: Severity,
    domain: &str,
) -> Verdict {
    let id = RuleId::new(namespace, name, major);
    let summary = format!("synthetic {} verdict", domain);
    let base = match severity {
        Severity::Healthy => Verdict::healthy(id.clone(), when, summary),
        Severity::Warn => Verdict::warn(id.clone(), when, summary),
        Severity::Critical => Verdict::critical(id.clone(), when, summary),
        Severity::Info => Verdict::warn(id.clone(), when, summary), // promote
        Severity::Error => Verdict::error(id.clone(), when, summary),
        _ => Verdict::healthy(id.clone(), when, summary),
    };
    base.with_tags(
        Tags::empty()
            .with_value("domain", domain)
            .with_value("starter.rule.subkind", name)
            .with_flag("perf-smoke"),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d9_slo_smoke() {
    let pool = starter_store_sqlite::testing::ephemeral().await;
    starter_store_sqlite::migrate(&pool)
        .with_source(INSIGHTS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("insights migrations apply");

    let store = VerdictStore::new(pool.clone());
    let rollup = RollupEngine::new(pool.clone());
    let cache = DerivationCache::new(pool.clone());

    // ----------------------------------------------------------------
    // (1) Seed a synthetic 90-day dataset. We emit one verdict per
    //     reference rule per hour for 90 days = 2160 rows / rule.
    //     The shape rotates Healthy / Warn / Critical across the
    //     hour so the rollup counters carry real load.
    // ----------------------------------------------------------------
    let start = Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap();
    let days: i64 = 90;
    let hours_total: i64 = days * 24;

    for r in REF_RULES {
        // Append via a single transaction-batch by sequential
        // `append` calls. The point of the perf smoke is to measure
        // *reads*, not seed throughput; cheap-but-correct is fine.
        for h in 0..hours_total {
            let when = start + ChronoDuration::hours(h);
            let sev = match h % 6 {
                0 => Severity::Critical,
                1 | 2 => Severity::Warn,
                _ => Severity::Healthy,
            };
            let v = make_verdict(r.namespace, r.name, r.major, when, sev, r.domain);
            store.append(&v).await.expect("append");
        }
    }

    // Sanity: 3 rules × 24×90 rows.
    assert_eq!(
        store.count().await.unwrap(),
        REF_RULES.len() as i64 * hours_total
    );

    // ----------------------------------------------------------------
    // (2) Roll up — one tick per (rule, window_class). Per R-ins-8
    //     we group by the `domain` tag.
    // ----------------------------------------------------------------
    let tz = chrono_tz::UTC;
    let tag_keys = ["domain"];
    for r in REF_RULES {
        // Day buckets for the 90-day timeseries SLO.
        let n = rollup
            .tick_incremental(
                r.namespace,
                r.name,
                r.major,
                WindowClass::Day,
                tz,
                &tag_keys,
            )
            .await
            .expect("rollup tick");
        assert_eq!(n as i64, hours_total, "all rows folded");
    }

    // ----------------------------------------------------------------
    // (3) Populate the derivation cache: one row per (rule, day)
    //     for IoT — enough to make `get` realistic, not so many
    //     that the smoke turns into a write benchmark.
    // ----------------------------------------------------------------
    let derive_id = RuleId::new("iot", "device.frame", 1);
    for d in 0..days {
        let window_start = start + ChronoDuration::days(d);
        let window_end = window_start + ChronoDuration::days(1);
        let rows = (0..24)
            .map(|h| {
                serde_json::json!({
                    "ts": (window_start + ChronoDuration::hours(h)).to_rfc3339(),
                    "value": (d * 24 + h) as f64,
                })
            })
            .collect();
        let ds = Dataset::from_parts(
            DatasetSchema::new(["ts", "value"]),
            std::sync::Arc::new(VecDatasetRows::new(rows)),
            Coverage::full_point(),
            TimeZoneId::utc(),
            Some(Window::new(window_start, window_end)),
        );
        cache
            .put(&derive_id, window_start, window_end, &ds)
            .await
            .expect("cache put");
    }

    // ----------------------------------------------------------------
    // (4) Take SAMPLES timings against every D9 read-path. We rotate
    //     across the three reference rules so the SQLite plan cache
    //     isn't accidentally serving one query from RAM.
    // ----------------------------------------------------------------
    let end = start + ChronoDuration::days(days);

    let mut t_verdict_list = Vec::with_capacity(SAMPLES);
    for i in 0..SAMPLES {
        let r = &REF_RULES[i % REF_RULES.len()];
        let t0 = Instant::now();
        let out = store
            .list_recent_by_rule(r.namespace, r.name, r.major, 100)
            .await
            .expect("recent");
        t_verdict_list.push(t0.elapsed());
        assert_eq!(out.len(), 100);
    }

    let mut t_verdict_by_tag = Vec::with_capacity(SAMPLES);
    for i in 0..SAMPLES {
        let r = &REF_RULES[i % REF_RULES.len()];
        let t0 = Instant::now();
        let out = store
            .list_recent_by_tag("domain", Some(r.domain), 100)
            .await
            .expect("by tag");
        t_verdict_by_tag.push(t0.elapsed());
        assert_eq!(out.len(), 100);
    }

    let mut t_rollup_ts = Vec::with_capacity(SAMPLES);
    for i in 0..SAMPLES {
        let r = &REF_RULES[i % REF_RULES.len()];
        let t0 = Instant::now();
        let series = rollup
            .read_timeseries_ungrouped(
                r.namespace,
                r.name,
                r.major,
                WindowClass::Day,
                start,
                end,
            )
            .await
            .expect("ungrouped");
        t_rollup_ts.push(t0.elapsed());
        assert_eq!(series.len() as i64, days, "one bucket per day");
    }

    let mut t_rollup_tag = Vec::with_capacity(SAMPLES);
    for i in 0..SAMPLES {
        let r = &REF_RULES[i % REF_RULES.len()];
        let t0 = Instant::now();
        let series = rollup
            .read_timeseries_grouped(
                r.namespace,
                r.name,
                r.major,
                WindowClass::Day,
                "domain",
                start,
                end,
            )
            .await
            .expect("grouped");
        t_rollup_tag.push(t0.elapsed());
        assert_eq!(series.len() as i64, days);
    }

    let mut t_cache_fetch = Vec::with_capacity(SAMPLES);
    for i in 0..SAMPLES {
        let day = (i as i64) % days;
        let when = start + ChronoDuration::days(day);
        let t0 = Instant::now();
        let hit = cache.get(&derive_id, when).await.expect("cache hit");
        t_cache_fetch.push(t0.elapsed());
        assert!(hit.is_some(), "cache must hit for seeded windows");
    }

    // ----------------------------------------------------------------
    // (5) Assert p95 against the SLO column. The numbers ARE the
    //     budgets — a regression fails CI; the fix is to tighten
    //     the query, not relax the budget (D9 rule 2).
    // ----------------------------------------------------------------
    let p95_list = p95_ms(&mut t_verdict_list);
    let p95_tag = p95_ms(&mut t_verdict_by_tag);
    let p95_ts = p95_ms(&mut t_rollup_ts);
    let p95_ts_tag = p95_ms(&mut t_rollup_tag);
    let p95_cache = p95_ms(&mut t_cache_fetch);

    eprintln!(
        "D9 p95 (ms): verdict-list={p95_list} tag-list={p95_tag} \
         rollup-ts={p95_ts} rollup-tag={p95_ts_tag} cache={p95_cache}"
    );

    assert!(
        p95_list <= SLO_VERDICT_LIST_MS,
        "verdict-list p95 {p95_list}ms > SLO {SLO_VERDICT_LIST_MS}ms (tighten the query, not the budget)"
    );
    assert!(
        p95_tag <= SLO_VERDICT_LIST_BY_TAG_MS,
        "verdict-list-by-tag p95 {p95_tag}ms > SLO {SLO_VERDICT_LIST_BY_TAG_MS}ms"
    );
    assert!(
        p95_ts <= SLO_ROLLUP_TIMESERIES_MS,
        "rollup-timeseries p95 {p95_ts}ms > SLO {SLO_ROLLUP_TIMESERIES_MS}ms"
    );
    assert!(
        p95_ts_tag <= SLO_ROLLUP_TAG_GROUPED_MS,
        "rollup-tag-grouped p95 {p95_ts_tag}ms > SLO {SLO_ROLLUP_TAG_GROUPED_MS}ms"
    );
    assert!(
        p95_cache <= SLO_DERIVATION_CACHE_MS,
        "derivation-cache p95 {p95_cache}ms > SLO {SLO_DERIVATION_CACHE_MS}ms"
    );
}
