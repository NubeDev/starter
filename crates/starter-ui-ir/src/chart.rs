//! Chart and KPI wire types — the source-of-truth contracts shared by
//! [`Component::Chart`](crate::Component::Chart) and
//! [`Component::Kpi`](crate::Component::Kpi).
//!
//! Phase 1+2 reshape (IR v3): [`ChartSource`] is now a discriminated
//! enum, not a flat struct. The pre-existing single-node `Series` source
//! is one variant alongside fan-outs (`SeriesByKind`), categorical RSQL
//! aggregates (`Rows`), bucket-stitched RSQL time-series
//! (`SeriesFromRsql`), and server-static authored points (`Static`).
//!
//! Wire grammar follows the IR convention used elsewhere in this crate:
//! discriminator field is `"type"`, casing is `snake_case`, every new
//! `serde`-tagged enum carries a `#[serde(other)] Unknown` fallback so
//! older clients deserialise newer trees instead of panicking.
//!
//! Two deliberate deviations from the design doc's Rust sketch:
//!
//! - Tuple variants like `Sum(String)` are encoded with **named fields**
//!   (`Sum { field: String }`) on the wire. Internally-tagged enums
//!   require named or unit variants — and named fields are forward-
//!   compatible (an optional new field is additive) where positional
//!   tuples aren't.
//! - [`ChartKind`] serialises as a bare string (e.g. `"line"`,
//!   `"stacked_bar"`) so it can be embedded next to other Chart fields
//!   without an envelope. `Custom(name)` round-trips as the literal
//!   renderer name; `Unknown` covers the empty / unrecognised case.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// -------------------------------------------------------------------
// ChartKind
// -------------------------------------------------------------------

/// What renderer to draw a [`Component::Chart`](crate::Component::Chart)
/// with. The renderer reads this; the resolver uses it to pick the
/// validation rules in [`ChartSource`] (see SDUI.md § "Chart kind ↔
/// source compatibility").
///
/// Wire shape: bare snake_case string (`"line"`, `"stacked_bar"`, …).
/// `Custom(name)` carries the renderer-registry id verbatim. `Unknown`
/// is the forward-compat fallback when a v4+ kind reaches a v3 client.
///
/// `Heatmap` is reserved so we don't bump the IR version again later;
/// the renderer and the source-compat rules for it are deferred.
#[derive(Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[schemars(with = "String")]
pub enum ChartKind {
    /// Time-series line chart (default).
    #[default]
    Line,
    /// Filled-area time-series chart.
    Area,
    /// Categorical bar chart.
    Bar,
    /// Stacked categorical bar chart.
    StackedBar,
    /// Pie chart over categorical buckets.
    Pie,
    /// Donut chart over categorical buckets.
    Donut,
    /// Single-value gauge.
    Gauge,
    /// Reserved — renderer + source rules deferred (see module doc).
    Heatmap,
    /// Block-registered custom renderer; the inner string is the
    /// renderer registry id (e.g. `"acme.flow_canvas"`).
    Custom(String),
    /// Forward-compat fallback for kinds this client doesn't recognise.
    Unknown,
}

impl ChartKind {
    /// Wire form for the built-in variants. Returns `None` for
    /// [`ChartKind::Custom`] (caller emits the embedded name) and for
    /// [`ChartKind::Unknown`] (caller decides whether to emit an empty
    /// string or skip the field).
    pub fn as_builtin_str(&self) -> Option<&str> {
        Some(match self {
            ChartKind::Line => "line",
            ChartKind::Area => "area",
            ChartKind::Bar => "bar",
            ChartKind::StackedBar => "stacked_bar",
            ChartKind::Pie => "pie",
            ChartKind::Donut => "donut",
            ChartKind::Gauge => "gauge",
            ChartKind::Heatmap => "heatmap",
            ChartKind::Custom(_) | ChartKind::Unknown => return None,
        })
    }

    /// True iff this kind renders correctly on V2 clients. The V3→V2
    /// downgrader keeps these inline; everything else degrades to
    /// [`Component::Dangling`](crate::Component::Dangling).
    ///
    /// V2 clients ship a renderer for Line / Area only — `bar` was
    /// declared in the V2 IR but never had a working renderer ahead of
    /// the V3 reshape, so it joins the V3-only set rather than render
    /// silently broken.
    pub fn v2_compatible(&self) -> bool {
        matches!(self, ChartKind::Line | ChartKind::Area)
    }
}

impl Serialize for ChartKind {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let s = match self {
            ChartKind::Custom(name) => name.as_str(),
            ChartKind::Unknown => "unknown",
            other => other.as_builtin_str().unwrap(),
        };
        ser.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for ChartKind {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        Ok(match s.as_str() {
            "line" => ChartKind::Line,
            "area" => ChartKind::Area,
            "bar" => ChartKind::Bar,
            "stacked_bar" => ChartKind::StackedBar,
            "pie" => ChartKind::Pie,
            "donut" => ChartKind::Donut,
            "gauge" => ChartKind::Gauge,
            "heatmap" => ChartKind::Heatmap,
            "" | "unknown" => ChartKind::Unknown,
            _ => ChartKind::Custom(s),
        })
    }
}

// -------------------------------------------------------------------
// AggSpec
// -------------------------------------------------------------------

/// Aggregation operation applied to a [`ChartSource::Rows`] or
/// [`ChartSource::SeriesFromRsql`] payload.
///
/// Phase 6 variants (`Percentile`, `Rate`, `Delta`, `Stddev`) are wire-
/// reserved here so v3 clients can deserialise future trees that use
/// them. The resolver returns `UnsupportedAgg` if a tree uses one
/// before Phase 6 lands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AggSpec {
    /// Count rows in the result set.
    Count,
    /// Sum a numeric field across rows.
    Sum {
        /// Slot dot-path (e.g. `"settings.duration_ms"`).
        field: String,
    },
    /// Arithmetic mean of a numeric field.
    Avg { field: String },
    /// Minimum of a numeric field.
    Min { field: String },
    /// Maximum of a numeric field.
    Max { field: String },
    /// p-th percentile of a numeric field. Phase 6 — resolver returns
    /// `UnsupportedAgg` until then.
    Percentile {
        field: String,
        /// 0.0–1.0 (e.g. 0.95 = p95). `f64` for clean JSON round-trip
        /// (`f32` loses precision on common percentile values like 0.95).
        p: f64,
    },
    /// Per-second rate (delta over time window). Phase 6.
    Rate { field: String },
    /// Last-minus-first delta over the window. Phase 6.
    Delta { field: String },
    /// Sample standard deviation. Phase 6.
    Stddev { field: String },
    /// Forward-compat fallback for aggregations this client doesn't
    /// know about.
    #[serde(other)]
    Unknown,
}

impl AggSpec {
    /// True iff this is a Phase 1+2 supported aggregation. Phase 6
    /// variants return false until resolver support lands.
    pub fn is_phase_1_2(&self) -> bool {
        matches!(
            self,
            AggSpec::Count
                | AggSpec::Sum { .. }
                | AggSpec::Avg { .. }
                | AggSpec::Min { .. }
                | AggSpec::Max { .. }
        )
    }
}

// -------------------------------------------------------------------
// RowsOrder, DataPoint
// -------------------------------------------------------------------

/// Ordering applied to a [`ChartSource::Rows`] result before `limit`
/// is enforced. Used for top-N bar charts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum RowsOrder {
    /// Order by the aggregation result. `Desc` for "top N", `Asc` for
    /// "bottom N".
    Value {
        /// `"asc"` or `"desc"`.
        direction: OrderDirection,
    },
    /// Order by the group-by key (alphabetical).
    Key { direction: OrderDirection },
}

/// Sort direction for [`RowsOrder`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderDirection {
    Asc,
    Desc,
}

/// One server-authored static point in [`ChartSource::Static`]. Kept as
/// a tuple on the wire for compactness — `[ts_ms, value]`.
pub type DataPoint = (i64, f64);

// -------------------------------------------------------------------
// ChartSource
// -------------------------------------------------------------------

/// Where a chart or KPI gets its numbers.
///
/// The five non-fallback variants cover the resolver's data paths:
/// single-node telemetry ([`Series`](Self::Series)), kind-fanned
/// telemetry ([`SeriesByKind`](Self::SeriesByKind)), categorical RSQL
/// state ([`Rows`](Self::Rows)), bucket-stitched RSQL time-series
/// ([`SeriesFromRsql`](Self::SeriesFromRsql)), and author-emitted
/// static points ([`Static`](Self::Static)).
///
/// Wire grammar: internally-tagged on `"type"` with `snake_case`
/// variant names. `Series` keeps the V2 field set (`node_id`, `slot`,
/// `field`) plus the new `"type": "series"` discriminator — a V2 client
/// reading the V3 emission ignores the unknown field and renders as
/// before, which is what enables the surgical V3→V2 downgrade in the
/// resolver (only V3-only variants degrade to `Dangling`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChartSource {
    /// Time-series over a single node's slot — the V2 default. Live
    /// updates ride `node.<id>.slot.<slot>`.
    Series {
        /// Node id (UUID as string).
        node_id: String,
        /// Slot name on the node.
        slot: String,
        /// Optional dot-path into the slot value (`payload.count` for
        /// the conventional `Msg` envelope).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        field: Option<String>,
    },

    /// Time-series fanned out across every node of a given kind.
    /// Backed by `domain_history::grouped_telemetry`.
    SeriesByKind {
        /// Kind id, e.g. `"com.nube.plm.task"`.
        kind: String,
        /// Slot name shared across the kind's instances.
        slot: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        field: Option<String>,
        /// Cap on the number of series emitted (top-N by value).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        top_n: Option<u32>,
    },

    /// Relational query backed by RSQL. Counts rows, groups by a slot
    /// field, or aggregates a numeric field. Powers bar / pie / donut
    /// over current state and KPIs that count. Phase 3 ships the
    /// resolver; until then the resolver returns a `NotImplemented`
    /// payload and the client renders an empty placeholder.
    Rows {
        /// RSQL filter expression.
        rsql: String,
        /// Slot dot-path to group by. Required for bar/pie/donut.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group_by: Option<String>,
        /// Aggregation applied to each group.
        agg: AggSpec,
        /// Optional ordering for top-N.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        order: Option<RowsOrder>,
        /// Optional cap on result rows. The resolver enforces a server
        /// ceiling regardless of the authored value.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },

    /// Time-series of an RSQL aggregation. The resolver materialises
    /// this by issuing one [`Rows`](Self::Rows) query per bucket
    /// boundary in the requested range and stitching the results into
    /// a `(ts_ms, value)` series — eliminates the write-time rollup
    /// pattern. Phase 3 ships the resolver.
    SeriesFromRsql {
        rsql: String,
        agg: AggSpec,
        /// Bucket size in milliseconds.
        bucket_ms: i64,
        /// Group by a slot dot-path to emit multi-series. `None` =
        /// single series over the filtered rows.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        group_by: Option<String>,
    },

    /// Author-emitted static points. The resolver passes these through
    /// verbatim — useful for fixtures, doc charts, demos.
    Static {
        /// `[ts_ms, value]` tuples, oldest first.
        #[serde(default)]
        points: Vec<DataPoint>,
    },

    /// Named analytics SQL template, evaluated server-side via the
    /// host's analytics-query tool (e.g. `rubix.analytics.query`).
    /// `name` is the template stem; `params` are forwarded verbatim
    /// to the tool. `map` tells the resolver how to walk the row
    /// payload — for KPIs it picks the scalar column, for charts it
    /// picks the timestamp + value columns.
    AnalyticsTemplate {
        /// Template stem (filename without `.sql`).
        name: String,
        /// Parameters bound through the tool's named-parameter
        /// surface.
        #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
        params: std::collections::BTreeMap<String, serde_json::Value>,
        /// Row-shape mapping. Optional only because forward-compat
        /// payloads may omit it; the resolver requires it to produce
        /// any points.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        map: Option<AnalyticsTemplateMap>,
    },

    /// Forward-compat fallback for variants this client doesn't know.
    #[serde(other)]
    Unknown,
}

/// Row-shape mapping for [`ChartSource::AnalyticsTemplate`].
///
/// `value_field` is required — it names the column the resolver
/// reads to produce either the KPI scalar (row 0) or each chart
/// point's y-value. `ts_field` is required for charts and ignored
/// for KPIs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AnalyticsTemplateMap {
    /// Column name that carries the scalar / y-value.
    pub value_field: String,
    /// Column name that carries the bucket timestamp. May encode
    /// ms-since-epoch or an ISO-8601 string; the resolver parses
    /// both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts_field: Option<String>,
}

impl ChartSource {
    /// True iff this variant is renderable on V2 clients verbatim. The
    /// resolver's V3→V2 downgrader uses this — V3-only variants are
    /// replaced with [`Component::Dangling`](crate::Component::Dangling).
    pub fn v2_compatible(&self) -> bool {
        matches!(self, ChartSource::Series { .. })
    }
}

// -------------------------------------------------------------------
// ChartSeries / ChartRange / ChartHistory (unchanged from V2 — kept
// here so all chart types live in one module)
// -------------------------------------------------------------------

/// One series in a [`Component::Chart`](crate::Component::Chart) result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChartSeries {
    /// Display label.
    pub label: String,
    /// `[ts_ms, value]` pairs, oldest first.
    #[serde(default)]
    pub points: Vec<DataPoint>,
}

/// Inclusive time window (ms since epoch) for a chart.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct ChartRange {
    pub from: i64,
    pub to: i64,
}

/// Declarative history-backfill config — `range_ms` is rolling from
/// "now" so a `last_1h` dashboard stays current every mount.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChartHistory {
    /// Rolling window in ms from "now". `None` = "all time".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_ms: Option<i64>,
    /// Render a preset picker above the chart.
    #[serde(default, skip_serializing_if = "is_false")]
    pub user_selectable: bool,
    /// Preset options; empty → renderer default set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<ChartHistoryPreset>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// One row in [`ChartHistory::presets`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChartHistoryPreset {
    pub label: String,
    /// Rolling window in ms; `None` = "all time".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

// -------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chart_kind_serialises_as_bare_string() {
        assert_eq!(serde_json::to_value(ChartKind::Line).unwrap(), "line");
        assert_eq!(
            serde_json::to_value(ChartKind::StackedBar).unwrap(),
            "stacked_bar"
        );
        assert_eq!(
            serde_json::to_value(ChartKind::Custom("acme.flow".into())).unwrap(),
            "acme.flow"
        );
        assert_eq!(serde_json::to_value(ChartKind::Unknown).unwrap(), "unknown");
    }

    #[test]
    fn chart_kind_round_trip_unknown_string_becomes_custom() {
        let v = json!("acme.flow_canvas");
        let k: ChartKind = serde_json::from_value(v).unwrap();
        assert_eq!(k, ChartKind::Custom("acme.flow_canvas".into()));
    }

    #[test]
    fn chart_kind_default_is_line() {
        assert_eq!(ChartKind::default(), ChartKind::Line);
    }

    #[test]
    fn agg_spec_count_no_field_on_wire() {
        let v = serde_json::to_value(AggSpec::Count).unwrap();
        assert_eq!(v, json!({"type": "count"}));
    }

    #[test]
    fn agg_spec_sum_carries_field() {
        let v = serde_json::to_value(AggSpec::Sum {
            field: "duration_ms".into(),
        })
        .unwrap();
        assert_eq!(v, json!({"type": "sum", "field": "duration_ms"}));
    }

    #[test]
    fn agg_spec_percentile_named_p() {
        let v = serde_json::to_value(AggSpec::Percentile {
            field: "duration_ms".into(),
            p: 0.95,
        })
        .unwrap();
        assert_eq!(
            v,
            json!({"type": "percentile", "field": "duration_ms", "p": 0.95})
        );
    }

    #[test]
    fn agg_spec_unknown_is_forward_compat() {
        let v = json!({"type": "tdigest_q", "field": "x", "q": 0.5});
        let a: AggSpec = serde_json::from_value(v).unwrap();
        assert_eq!(a, AggSpec::Unknown);
    }

    #[test]
    fn chart_source_series_round_trip() {
        let s = ChartSource::Series {
            node_id: "abc".into(),
            slot: "out".into(),
            field: Some("payload.count".into()),
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "series");
        assert_eq!(v["node_id"], "abc");
        assert_eq!(v["slot"], "out");
        assert_eq!(v["field"], "payload.count");
        let back: ChartSource = serde_json::from_value(v).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn chart_source_rows_with_count_agg() {
        let s = ChartSource::Rows {
            rsql: "kind==com.nube.plm.task".into(),
            group_by: Some("settings.gate".into()),
            agg: AggSpec::Count,
            order: None,
            limit: Some(20),
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "rows");
        assert_eq!(v["agg"], json!({"type": "count"}));
        assert_eq!(v["limit"], 20);
    }

    #[test]
    fn chart_source_unknown_variant_is_forward_compat() {
        let v = json!({"type": "future_tile_grid", "shards": 4});
        let s: ChartSource = serde_json::from_value(v).unwrap();
        assert_eq!(s, ChartSource::Unknown);
    }

    #[test]
    fn v2_compatibility_flags() {
        assert!(ChartSource::Series {
            node_id: "x".into(),
            slot: "y".into(),
            field: None
        }
        .v2_compatible());
        assert!(!ChartSource::Rows {
            rsql: "kind==x".into(),
            group_by: None,
            agg: AggSpec::Count,
            order: None,
            limit: None,
        }
        .v2_compatible());
        assert!(ChartKind::Line.v2_compatible());
        assert!(ChartKind::Area.v2_compatible());
        assert!(!ChartKind::Bar.v2_compatible());
        assert!(!ChartKind::StackedBar.v2_compatible());
        assert!(!ChartKind::Custom("x".into()).v2_compatible());
    }

    #[test]
    fn rows_order_round_trip() {
        let o = RowsOrder::Value {
            direction: OrderDirection::Desc,
        };
        let v = serde_json::to_value(&o).unwrap();
        assert_eq!(v, json!({"by": "value", "direction": "desc"}));
        let back: RowsOrder = serde_json::from_value(v).unwrap();
        assert_eq!(back, o);
    }

    #[test]
    fn static_points_round_trip() {
        let s = ChartSource::Static {
            points: vec![(1_000, 1.0), (2_000, 1.5)],
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "static");
        assert_eq!(v["points"][0], json!([1_000, 1.0]));
    }
}
