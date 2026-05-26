//! Chart / KPI source resolver — turns a [`ChartSource`] in the
//! authored tree into the server-emitted payload the client renders.
//!
//! Today this pass handles two source variants:
//!
//! - [`ChartSource::Static`] — the author-baked `(ts_ms, value)`
//!   points. For a [`Component::Chart`] they become one
//!   [`ChartSeries`]; for a [`Component::Kpi`] the last point's
//!   value becomes the KPI scalar.
//! - [`ChartSource::AnalyticsTemplate`] — the resolver invokes the
//!   configured [`AnalyticsBridge`] (host-supplied), then maps the
//!   rows into either a `ChartSeries` (chart) or a scalar (KPI)
//!   using the [`AnalyticsTemplateMap`] the source carries.
//!
//! Other variants ([`ChartSource::Series`], `SeriesByKind`, `Rows`,
//! `SeriesFromRsql`) are left untouched — the existing client-side
//! data paths (telemetry SSE, RSQL queries) already feed them.
//!
//! Errors fall through to an empty payload (`series = []`, `value =
//! null`) so a flaky analytics tool never freezes the page; the
//! warning lands on the `rubix.sdui` tracing target.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use starter_ui_ir::{ChartSeries, ChartSource, Component, ComponentTree};

/// Host-supplied analytics dispatcher. Implementations forward the
/// request to whatever in-process tool resolves named SQL templates
/// (e.g. `rubix.analytics.query`) and return the rows as a JSON
/// array — one object per row, columns keyed by name.
///
/// The trait is intentionally minimal so the SDUI crate stays free
/// of tool-registry deps; rubix wires an adapter that bridges a
/// concrete `AnalyticsQueryTool` to this surface.
#[async_trait]
pub trait AnalyticsBridge: Send + Sync {
    /// Invoke `name` with `params`, return one JSON object per row.
    async fn invoke(
        &self,
        name: &str,
        params: &BTreeMap<String, JsonValue>,
    ) -> Result<Vec<JsonValue>, String>;
}

/// Walk `tree` and fill every chart / KPI payload whose source we
/// can resolve. `bridge` is optional — when absent, only `Static`
/// sources are honoured; analytics templates collapse to empty.
pub async fn resolve_chart_sources(
    tree: &mut ComponentTree,
    bridge: Option<&dyn AnalyticsBridge>,
) {
    walk(&mut tree.root, bridge).await;
}

async fn walk(node: &mut Component, bridge: Option<&dyn AnalyticsBridge>) {
    // The walk is iterative-via-stack to keep the async future
    // size bounded (a recursive `async fn` over a deep tree would
    // generate an unboxable type). We collect every (chart, kpi)
    // reference up front, then resolve them sequentially. The tree
    // is small (R8 caps it) so the up-front walk is cheap.
    let mut leaves: Vec<&mut Component> = Vec::new();
    collect_leaves(node, &mut leaves);

    for leaf in leaves {
        match leaf {
            Component::Kpi { source, value, .. } => {
                *value = resolve_kpi(source, bridge).await;
            }
            Component::Chart { sources, series, .. } => {
                let mut out = Vec::new();
                for src in sources.iter() {
                    if let Some(s) = resolve_chart(src, bridge).await {
                        out.push(s);
                    }
                }
                if !out.is_empty() {
                    *series = out;
                }
            }
            _ => {}
        }
    }
}

/// Recursively gather mutable refs to every Chart / Kpi descendant.
/// Lives outside the async walker so the borrow set is contained.
fn collect_leaves<'a>(node: &'a mut Component, out: &mut Vec<&'a mut Component>) {
    match node {
        Component::Page { children, .. }
        | Component::Row { children, .. }
        | Component::Col { children, .. }
        | Component::Grid { children, .. }
        | Component::Card { children, .. }
        | Component::Section { children, .. }
        | Component::Dialog { children, .. }
        | Component::Drawer { children, .. } => {
            for c in children {
                collect_leaves(c, out);
            }
        }
        Component::Tabs { tabs, .. } => {
            for t in tabs {
                for c in &mut t.children {
                    collect_leaves(c, out);
                }
            }
        }
        Component::Kpi { .. } | Component::Chart { .. } => out.push(node),
        _ => {}
    }
}

async fn resolve_kpi(
    source: &ChartSource,
    bridge: Option<&dyn AnalyticsBridge>,
) -> Option<JsonValue> {
    match source {
        ChartSource::Static { points } => points.last().map(|(_, v)| JsonValue::from(*v)),
        ChartSource::AnalyticsTemplate { name, params, map } => {
            let map = map.as_ref()?;
            let bridge = bridge?;
            match bridge.invoke(name, params).await {
                Ok(rows) => rows.first().and_then(|row| row.get(&map.value_field).cloned()),
                Err(err) => {
                    tracing::warn!(
                        target: "rubix.sdui",
                        template = name,
                        error = %err,
                        "analytics template invocation failed (KPI)",
                    );
                    None
                }
            }
        }
        _ => None,
    }
}

async fn resolve_chart(
    source: &ChartSource,
    bridge: Option<&dyn AnalyticsBridge>,
) -> Option<ChartSeries> {
    match source {
        ChartSource::Static { points } => Some(ChartSeries {
            label: String::new(),
            points: points.clone(),
        }),
        ChartSource::AnalyticsTemplate { name, params, map } => {
            let map = map.as_ref()?;
            let ts_field = map.ts_field.as_ref()?;
            let bridge = bridge?;
            let rows = match bridge.invoke(name, params).await {
                Ok(r) => r,
                Err(err) => {
                    tracing::warn!(
                        target: "rubix.sdui",
                        template = name,
                        error = %err,
                        "analytics template invocation failed (chart)",
                    );
                    return None;
                }
            };
            let mut points = Vec::with_capacity(rows.len());
            for row in &rows {
                let ts = row.get(ts_field).and_then(value_to_ts_ms);
                let v = row.get(&map.value_field).and_then(value_to_f64);
                if let (Some(t), Some(v)) = (ts, v) {
                    points.push((t, v));
                }
            }
            Some(ChartSeries {
                label: name.clone(),
                points,
            })
        }
        _ => None,
    }
}

/// ClickHouse `DateTime` columns come over `JSONEachRow` as either an
/// ISO-8601 string (`"2026-05-26 04:45:00"`) or seconds-since-epoch.
/// The resolver normalises both to ms-since-epoch.
fn value_to_ts_ms(v: &JsonValue) -> Option<i64> {
    if let Some(s) = v.as_str() {
        // Accept `YYYY-MM-DD HH:MM:SS[.fff][Z|±HH:MM]`. We replace
        // the space with `T` to satisfy RFC 3339 parsers.
        let mut normalised = if s.len() >= 19 && s.as_bytes().get(10) == Some(&b' ') {
            let (date, rest) = s.split_at(10);
            format!("{date}T{}", &rest[1..])
        } else {
            s.to_string()
        };
        // No tz → assume UTC.
        if normalised.len() >= 19
            && !normalised.contains('Z')
            && !normalised[11..].contains('+')
            && !normalised[11..].contains('-')
        {
            normalised.push('Z');
        }
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&normalised) {
            return Some(dt.timestamp_millis());
        }
        if let Ok(n) = s.parse::<i64>() {
            return Some(if n < 10_000_000_000 { n * 1000 } else { n });
        }
        return None;
    }
    if let Some(n) = v.as_i64() {
        return Some(if n < 10_000_000_000 { n * 1000 } else { n });
    }
    if let Some(n) = v.as_f64() {
        let n = n as i64;
        return Some(if n < 10_000_000_000 { n * 1000 } else { n });
    }
    None
}

fn value_to_f64(v: &JsonValue) -> Option<f64> {
    if let Some(n) = v.as_f64() {
        return Some(n);
    }
    if let Some(n) = v.as_i64() {
        return Some(n as f64);
    }
    if let Some(s) = v.as_str() {
        return s.parse().ok();
    }
    None
}

/// Convenience alias so `SduiState` can hold the bridge as a
/// trait object without leaking the `Arc` wrapper to call sites.
pub type AnalyticsBridgeRef = Arc<dyn AnalyticsBridge>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use starter_ui_ir::{AnalyticsTemplateMap, ChartKind, ChartSource};

    #[derive(Default)]
    struct StubBridge {
        rows: Vec<JsonValue>,
    }

    #[async_trait]
    impl AnalyticsBridge for StubBridge {
        async fn invoke(
            &self,
            _name: &str,
            _params: &BTreeMap<String, JsonValue>,
        ) -> Result<Vec<JsonValue>, String> {
            Ok(self.rows.clone())
        }
    }

    #[tokio::test]
    async fn static_kpi_takes_last_point_value() {
        let mut kpi = Component::Kpi {
            id: None,
            label: "k".into(),
            source: ChartSource::Static {
                points: vec![(1, 1.0), (2, 4.5)],
            },
            value: None,
            format: None,
            intent: None,
            delta: None,
            unit_symbol: None,
            style: None,
        };
        let mut leaves = Vec::new();
        collect_leaves(&mut kpi, &mut leaves);
        for leaf in leaves {
            if let Component::Kpi { source, value, .. } = leaf {
                *value = resolve_kpi(source, None).await;
            }
        }
        let v = if let Component::Kpi { value, .. } = &kpi { value.clone() } else { None };
        assert_eq!(v, Some(json!(4.5)));
    }

    #[tokio::test]
    async fn analytics_template_kpi_uses_value_field() {
        let bridge = StubBridge {
            rows: vec![json!({ "kwh": 12.5, "meter_id": "m" })],
        };
        let src = ChartSource::AnalyticsTemplate {
            name: "meter_kwh_last_24h".into(),
            params: BTreeMap::new(),
            map: Some(AnalyticsTemplateMap {
                value_field: "kwh".into(),
                ts_field: None,
            }),
        };
        let v = resolve_kpi(&src, Some(&bridge)).await;
        assert_eq!(v, Some(json!(12.5)));
    }

    #[tokio::test]
    async fn analytics_template_chart_parses_iso_timestamp() {
        let bridge = StubBridge {
            rows: vec![
                json!({ "bucket_start": "2026-05-26 04:45:00", "value_avg": 1.5 }),
                json!({ "bucket_start": "2026-05-26 05:00:00", "value_avg": 2.0 }),
            ],
        };
        let src = ChartSource::AnalyticsTemplate {
            name: "meter_value_30d_15m".into(),
            params: BTreeMap::new(),
            map: Some(AnalyticsTemplateMap {
                value_field: "value_avg".into(),
                ts_field: Some("bucket_start".into()),
            }),
        };
        let series = resolve_chart(&src, Some(&bridge)).await.unwrap();
        assert_eq!(series.points.len(), 2);
        assert!(series.points[1].0 > series.points[0].0);
        assert_eq!(series.points[0].1, 1.5);
    }

    #[tokio::test]
    async fn missing_bridge_collapses_to_no_data() {
        let src = ChartSource::AnalyticsTemplate {
            name: "x".into(),
            params: BTreeMap::new(),
            map: Some(AnalyticsTemplateMap {
                value_field: "v".into(),
                ts_field: Some("t".into()),
            }),
        };
        assert!(resolve_kpi(&src, None).await.is_none());
        assert!(resolve_chart(&src, None).await.is_none());
    }

    #[tokio::test]
    async fn walk_fills_static_chart_series() {
        let mut tree = ComponentTree {
            ir_version: starter_ui_ir::IR_VERSION,
            vars: Default::default(),
            root: Component::Page {
                id: "p".into(),
                title: None,
                header_actions: vec![],
                children: vec![Component::Chart {
                    id: None,
                    sources: vec![ChartSource::Static {
                        points: vec![(1_000, 1.0), (2_000, 2.0)],
                    }],
                    kind: ChartKind::Line,
                    agg: None,
                    series: vec![],
                    range: None,
                    page_state_key: None,
                    history: None,
                }],
                style: None,
                default_row_gap: None,
                default_column_gap: None,
                default_page_padding: None,
                default_max_width: None,
            },
        };
        resolve_chart_sources(&mut tree, None).await;
        if let Component::Page { children, .. } = &tree.root {
            if let Component::Chart { series, .. } = &children[0] {
                assert_eq!(series.len(), 1);
                assert_eq!(series[0].points.len(), 2);
                return;
            }
        }
        panic!("expected chart with one series");
    }
}
