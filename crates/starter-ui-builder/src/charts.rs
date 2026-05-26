//! Typed chart constructors with compile-time source/kind safety.
//!
//! The IR's [`starter_ui_ir::ChartSource`] is a single open enum that
//! any chart kind can in principle hold; the resolver's runtime
//! validation table rejects nonsensical combinations (e.g. `Pie +
//! Series`) at request time. That's the right safety net for
//! hand-authored / AI-emitted JSON, but it's a regrettable footgun
//! for Rust authors who would prefer the compiler to catch the
//! mismatch.
//!
//! This module fronts the IR with two tiny newtypes —
//! [`TimeSeriesSource`] and [`RowsSource`] — that partition the
//! variants by which kind they accept:
//!
//! | Newtype             | Wraps `ChartSource` variants                                                       |
//! |---------------------|-------------------------------------------------------------------------------------|
//! | [`TimeSeriesSource`]| [`Series`](starter_ui_ir::ChartSource::Series), [`SeriesByKind`](starter_ui_ir::ChartSource::SeriesByKind), [`SeriesFromRsql`](starter_ui_ir::ChartSource::SeriesFromRsql), [`Static`](starter_ui_ir::ChartSource::Static) |
//! | [`RowsSource`]      | [`Rows`](starter_ui_ir::ChartSource::Rows)                                          |
//!
//! Builder methods then accept exactly one — `line_chart` / `gauge`
//! / `area_chart` accept `TimeSeriesSource`; `bar_chart` /
//! `pie_chart` / `donut_chart` accept `RowsSource`. Wrong
//! combinations fail to compile rather than failing at resolve
//! time.
//!
//! # Example
//!
//! ```
//! use starter_ui_builder::prelude::*;
//!
//! let chart = bar_chart("by_gate")
//!     .source(rows(rsql().kind("com.acme.task"))
//!         .group_by("settings.gate")
//!         .count())
//!     .build();
//! let v = serde_json::to_value(&chart).unwrap();
//! assert_eq!(v["type"], "chart");
//! assert_eq!(v["kind"], "bar");
//! assert_eq!(v["sources"][0]["type"], "rows");
//! ```

use starter_ui_ir::{AggSpec, ChartKind, ChartSource, Component};

use crate::rsql::RsqlBuilder;

// =====================================================================
// Source newtypes
// =====================================================================

/// A [`ChartSource`] variant that yields time-series data — accepted
/// by [`line_chart`], [`gauge`], [`sparkline`], and [`kpi`].
#[derive(Debug, Clone)]
pub struct TimeSeriesSource(ChartSource);

impl TimeSeriesSource {
    /// Lift a typed [`ChartSource`] into a `TimeSeriesSource` without
    /// validation — escape hatch for callers building outside the
    /// canonical constructors below.
    pub fn from_chart_source(s: ChartSource) -> Self {
        Self(s)
    }

    /// Unwrap to the underlying [`ChartSource`].
    pub fn into_chart_source(self) -> ChartSource {
        self.0
    }
}

/// A [`ChartSource::Rows`] payload — accepted by [`bar_chart`] (and
/// pie / donut once their builders ship).
#[derive(Debug, Clone)]
pub struct RowsSource(ChartSource);

impl RowsSource {
    /// Unwrap to the underlying [`ChartSource`].
    pub fn into_chart_source(self) -> ChartSource {
        self.0
    }
}

// =====================================================================
// Source constructors
// =====================================================================

/// Build a [`TimeSeriesSource`] addressing a single node + slot.
pub fn series(node_id: impl Into<String>, slot: impl Into<String>) -> TimeSeriesSource {
    TimeSeriesSource(ChartSource::Series {
        node_id: node_id.into(),
        slot: slot.into(),
        field: None,
    })
}

/// Build a [`RowsSource`] from an RSQL filter. Apply
/// [`RowsSourceBuilder::group_by`] then one of the agg verbs
/// (`count` / `sum` / `avg` / `min` / `max`) — the agg is required
/// because [`ChartSource::Rows`] makes it non-optional on the wire.
pub fn rows(rsql: RsqlBuilder) -> RowsSourceBuilder {
    RowsSourceBuilder {
        rsql: rsql.build(),
        group_by: None,
        agg: None,
        limit: None,
    }
}

/// Builder for [`ChartSource::Rows`].
#[derive(Debug, Clone)]
pub struct RowsSourceBuilder {
    rsql: String,
    group_by: Option<String>,
    agg: Option<AggSpec>,
    limit: Option<u32>,
}

impl RowsSourceBuilder {
    /// Group rows by a slot dot-path. Required for bar / pie / donut
    /// (the resolver's validation table rejects ungrouped Rows there).
    pub fn group_by(mut self, field: impl Into<String>) -> Self {
        self.group_by = Some(field.into());
        self
    }

    /// Cap the number of result rows. The resolver enforces a server
    /// ceiling regardless of the authored value.
    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }

    /// Aggregate via `count(*)`. Materialises the [`RowsSource`].
    pub fn count(self) -> RowsSource {
        self.with_agg(AggSpec::Count)
    }

    /// Aggregate via `sum(field)`. Materialises the [`RowsSource`].
    pub fn sum(self, field: impl Into<String>) -> RowsSource {
        self.with_agg(AggSpec::Sum {
            field: field.into(),
        })
    }

    /// Aggregate via `avg(field)`. Materialises the [`RowsSource`].
    pub fn avg(self, field: impl Into<String>) -> RowsSource {
        self.with_agg(AggSpec::Avg {
            field: field.into(),
        })
    }

    /// Aggregate via `min(field)`. Materialises the [`RowsSource`].
    pub fn min(self, field: impl Into<String>) -> RowsSource {
        self.with_agg(AggSpec::Min {
            field: field.into(),
        })
    }

    /// Aggregate via `max(field)`. Materialises the [`RowsSource`].
    pub fn max(self, field: impl Into<String>) -> RowsSource {
        self.with_agg(AggSpec::Max {
            field: field.into(),
        })
    }

    fn with_agg(mut self, agg: AggSpec) -> RowsSource {
        self.agg = Some(agg);
        RowsSource(ChartSource::Rows {
            rsql: self.rsql,
            group_by: self.group_by,
            agg: self.agg.unwrap(),
            order: None,
            limit: self.limit,
        })
    }
}

// =====================================================================
// Chart builders — partitioned by which source they accept
// =====================================================================

/// Builder for a [`Component::Chart`] that accepts a
/// [`TimeSeriesSource`].
#[derive(Debug, Clone)]
pub struct TimeSeriesChartBuilder {
    id: String,
    kind: ChartKind,
    sources: Vec<ChartSource>,
}

impl TimeSeriesChartBuilder {
    /// Set the chart's primary source. Accepts only
    /// [`TimeSeriesSource`] — passing a [`RowsSource`] is a compile
    /// error.
    pub fn source(mut self, s: TimeSeriesSource) -> Self {
        self.sources = vec![s.into_chart_source()];
        self
    }

    /// Append another source for multi-series authoring.
    pub fn add_source(mut self, s: TimeSeriesSource) -> Self {
        self.sources.push(s.into_chart_source());
        self
    }

    /// Materialise to a [`Component::Chart`].
    pub fn build(self) -> Component {
        Component::Chart {
            id: Some(self.id),
            sources: self.sources,
            kind: self.kind,
            agg: None,
            series: Vec::new(),
            range: None,
            page_state_key: None,
            history: None,
        }
    }
}

/// Builder for a [`Component::Chart`] that accepts a [`RowsSource`].
#[derive(Debug, Clone)]
pub struct RowsChartBuilder {
    id: String,
    kind: ChartKind,
    sources: Vec<ChartSource>,
}

impl RowsChartBuilder {
    /// Set the chart's primary source. Accepts only [`RowsSource`] —
    /// passing a [`TimeSeriesSource`] is a compile error.
    pub fn source(mut self, s: RowsSource) -> Self {
        self.sources = vec![s.into_chart_source()];
        self
    }

    /// Append another source for multi-series authoring.
    pub fn add_source(mut self, s: RowsSource) -> Self {
        self.sources.push(s.into_chart_source());
        self
    }

    /// Materialise to a [`Component::Chart`].
    pub fn build(self) -> Component {
        Component::Chart {
            id: Some(self.id),
            sources: self.sources,
            kind: self.kind,
            agg: None,
            series: Vec::new(),
            range: None,
            page_state_key: None,
            history: None,
        }
    }
}

// =====================================================================
// Top-level chart constructors
// =====================================================================

/// Construct a [`ChartKind::Line`] chart. The id parameter is used as
/// the IR component's `id` (also the subscription-plan key).
pub fn line_chart(id: impl Into<String>) -> TimeSeriesChartBuilder {
    TimeSeriesChartBuilder {
        id: id.into(),
        kind: ChartKind::Line,
        sources: Vec::new(),
    }
}

/// Construct a [`ChartKind::Bar`] chart. Bars require a grouped
/// [`RowsSource`] — use `rows(rsql).group_by(...).count()`.
pub fn bar_chart(id: impl Into<String>) -> RowsChartBuilder {
    RowsChartBuilder {
        id: id.into(),
        kind: ChartKind::Bar,
        sources: Vec::new(),
    }
}

/// Construct a [`ChartKind::Gauge`] chart over a time-series
/// last-value.
pub fn gauge(id: impl Into<String>) -> TimeSeriesChartBuilder {
    TimeSeriesChartBuilder {
        id: id.into(),
        kind: ChartKind::Gauge,
        sources: Vec::new(),
    }
}

/// Construct a [`Component::Sparkline`]. Sparklines aren't a `Chart`
/// variant in the IR — they have their own component shape — but the
/// authoring API lives next to the chart constructors so callers find
/// it via prelude.
pub fn sparkline(id: impl Into<String>, subscribe: impl Into<String>) -> Component {
    Component::Sparkline {
        id: Some(id.into()),
        values: Vec::new(),
        subscribe: Some(subscribe.into()),
        intent: None,
        unit_symbol: None,
    }
}

/// Construct a [`Component::Kpi`] — a big-number tile fed by a
/// [`TimeSeriesSource`]. KPIs use the singular `source` (not
/// `sources`) per IR; the typed builder normalises that.
pub fn kpi(id: impl Into<String>, label: impl Into<String>, source: TimeSeriesSource) -> Component {
    Component::Kpi {
        id: Some(id.into()),
        label: label.into(),
        source: source.into_chart_source(),
        value: None,
        format: None,
        intent: None,
        delta: None,
        unit_symbol: None,
        style: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rsql::rsql;

    #[test]
    fn bar_chart_with_rows_source_builds() {
        let chart = bar_chart("by_gate")
            .source(
                rows(rsql().kind("com.acme.task"))
                    .group_by("settings.gate")
                    .count(),
            )
            .build();
        let v = serde_json::to_value(&chart).unwrap();
        assert_eq!(v["type"], "chart");
        assert_eq!(v["kind"], "bar");
        assert_eq!(v["sources"][0]["type"], "rows");
        assert_eq!(v["sources"][0]["group_by"], "settings.gate");
    }

    #[test]
    fn line_chart_with_series_source_builds() {
        let chart = line_chart("temp").source(series("node-1", "value")).build();
        let v = serde_json::to_value(&chart).unwrap();
        assert_eq!(v["kind"], "line");
        assert_eq!(v["sources"][0]["type"], "series");
        assert_eq!(v["sources"][0]["node_id"], "node-1");
    }

    #[test]
    fn kpi_with_series_source() {
        let k = kpi("active", "Active tasks", series("n1", "count"));
        let v = serde_json::to_value(&k).unwrap();
        assert_eq!(v["type"], "kpi");
        assert_eq!(v["label"], "Active tasks");
        assert_eq!(v["source"]["type"], "series");
    }

    #[test]
    fn sparkline_carries_subscribe() {
        let s = sparkline("spark", "node.x.slot.value");
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "sparkline");
        assert_eq!(v["subscribe"], "node.x.slot.value");
    }

    // Compile-time safety: the following lines should NOT compile.
    // Uncommenting them is the manual falsification — preserved here
    // as documentation of the invariant the newtype wrappers enforce.
    //
    //     let _ = bar_chart("x").source(series("n", "s"));      // RowsSource expected
    //     let _ = line_chart("x").source(rows(rsql()).count()); // TimeSeriesSource expected
}
