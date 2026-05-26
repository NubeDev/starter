//! Component enum — the heart of the IR.
//!
//! Six categories plus placeholder stubs for ACL-redacted and
//! dangling widgets. Every variant carries `#[serde(tag = "type")]`
//! so the wire discriminator is the stable `"type"` field.
//!
//! S1 variants (~15): page, row, col, grid, tabs, text, heading,
//! badge, button, form, table, diff, rich_text, forbidden, dangling.
//! S1 (write-path): toggle, slider, BindingSpec, Concurrency.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::chart::{AggSpec, ChartHistory, ChartKind, ChartRange, ChartSeries, ChartSource};

// -------------------------------------------------------------------
// Binding types (write-path, S1)
// -------------------------------------------------------------------

/// Slot binding for a two-way bound control (toggle, slider, …).
///
/// Short form is sugar: `"$target.enabled"` is equivalent to
/// `{ slot: "$target.enabled", concurrency: "lww" }`. The binding
/// expression grammar is the same as read bindings (DASHBOARD.md §
/// Bindings) — `$target.*`, `$self.*`, `$stack.*`, child-walk `/`.
/// Must resolve to a concrete slot at resolve time.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BindingSpec {
    /// Sugar: just the binding expression string.
    Short(String),
    /// Full form — allows overriding concurrency and debounce per
    /// control.
    Full {
        slot: String,
        #[serde(default)]
        concurrency: Concurrency,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        debounce_ms: Option<u32>,
    },
}

impl BindingSpec {
    /// Extract the slot binding expression regardless of form.
    pub fn slot_expr(&self) -> &str {
        match self {
            BindingSpec::Short(s) => s.as_str(),
            BindingSpec::Full { slot, .. } => slot.as_str(),
        }
    }

    /// Concurrency mode — `Lww` when not specified.
    pub fn concurrency(&self) -> Concurrency {
        match self {
            BindingSpec::Short(_) => Concurrency::default(),
            BindingSpec::Full { concurrency, .. } => *concurrency,
        }
    }

    /// Debounce in ms — `None` defers to the component's default.
    pub fn debounce_ms(&self) -> Option<u32> {
        match self {
            BindingSpec::Short(_) => None,
            BindingSpec::Full { debounce_ms, .. } => *debounce_ms,
        }
    }
}

/// Newtype over `Vec<BindingSpec>` — the field type carried by every
/// bound variant of `Component`. The wire form accepts three shapes,
/// all of which deserialise into the same flat list:
///
/// 1. A single string (`"$target.enabled"`) — the common case;
///    sugar for one `BindingSpec::Short`.
/// 2. A single object (`{ slot: "...", concurrency: "occ" }`) — one
///    `BindingSpec::Full` for the structured form.
/// 3. An array of either — the multi-write fan-out case. Each entry
///    becomes one `WritePlan` entry; the first entry is the read
///    source by convention (see SDUI-VALUES.md §3.1).
///
/// Why a newtype rather than `Vec<BindingSpec>` directly: the
/// `#[derive(Bindable)]` macro keys off this type to identify
/// bindable variants. A bare `Vec<BindingSpec>` could appear on a
/// non-bound variant for unrelated reasons; `Bindings` is an
/// unambiguous structural marker that "this variant carries the
/// bound-control contract." See SDUI-VALUES.md §3.4.
#[derive(Debug, Clone, Default, JsonSchema)]
#[serde(transparent)]
pub struct Bindings(pub Vec<BindingSpec>);

impl Serialize for Bindings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Mirror the wire form accepted by Deserialize: a single
        // entry serialises as the bare spec (string or object), a
        // multi-entry list serialises as an array. Empty serialises
        // as an empty array — a Bindings with no entries is itself
        // a contract violation on bound variants (caught by the
        // derive's witnesses) but we still need a consistent shape
        // for the no-witness case.
        match self.0.as_slice() {
            [single] => single.serialize(serializer),
            many => many.serialize(serializer),
        }
    }
}

impl Bindings {
    /// Wrap a single `BindingSpec`. Convenience for hand-built
    /// fixtures and tests; the wire form's most common shape.
    pub fn one(spec: BindingSpec) -> Self {
        Self(vec![spec])
    }

    /// Iterate the contained specs in declaration order. Renderer
    /// and WritePlan both consume in this order.
    pub fn iter(&self) -> std::slice::Iter<'_, BindingSpec> {
        self.0.iter()
    }

    /// First spec, by convention the read source. `None` only when
    /// the list is empty — which the derive's compile-time witnesses
    /// disallow on bound variants but is structurally permitted.
    pub fn first(&self) -> Option<&BindingSpec> {
        self.0.first()
    }

    /// Borrow the underlying slice for `write_bindings()`-style
    /// iteration. Returns an empty slice when no specs are present.
    pub fn as_slice(&self) -> &[BindingSpec] {
        self.0.as_slice()
    }

    /// `true` when no specs are declared. The compile-time witnesses
    /// in `#[derive(Bindable)]` reject this on bound variants, but
    /// runtime code that constructs a `Bindings` programmatically
    /// (tests, dynamic authoring) can observe it.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of bound slots — one for the common case, more for
    /// fan-out. Layout walkers that report stats use this.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Slot expression of the first (read) entry. Convenience for
    /// tests and read-side helpers; `None` for an empty `Bindings`.
    /// Per SDUI-VALUES.md §3.1 the first entry is the read source.
    pub fn slot_expr(&self) -> Option<&str> {
        self.0.first().map(BindingSpec::slot_expr)
    }

    /// Concurrency mode of the first (read) entry. `None` for an
    /// empty `Bindings`. Each fan-out entry carries its own
    /// concurrency mode; this helper exposes only the first.
    pub fn concurrency(&self) -> Option<Concurrency> {
        self.0.first().map(BindingSpec::concurrency)
    }
}

impl<'de> Deserialize<'de> for Bindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // The wire accepts a string, a structured object, or an array
        // of either. We deserialise into `serde_json::Value` first and
        // then dispatch on shape — using untagged enums or a custom
        // visitor here would each lose one of the three forms or
        // produce noisy "no matching variant" diagnostics. The
        // intermediate `Value` is fine: bindings are tiny and parsing
        // happens once per layout.
        use serde::de::Error as _;

        let v = serde_json::Value::deserialize(deserializer)?;
        match v {
            serde_json::Value::String(_) | serde_json::Value::Object(_) => {
                let spec: BindingSpec = serde_json::from_value(v).map_err(D::Error::custom)?;
                Ok(Bindings(vec![spec]))
            }
            serde_json::Value::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    let spec: BindingSpec =
                        serde_json::from_value(item).map_err(D::Error::custom)?;
                    out.push(spec);
                }
                Ok(Bindings(out))
            }
            other => Err(D::Error::custom(format!(
                "bind: expected string, object, or array; got {}",
                match &other {
                    serde_json::Value::Null => "null",
                    serde_json::Value::Bool(_) => "bool",
                    serde_json::Value::Number(_) => "number",
                    _ => unreachable!(),
                }
            ))),
        }
    }
}

/// Concurrency mode for two-way bound controls.
///
/// - `Lww` (last-write-wins, default): no `expected_generation` sent.
///   Appropriate for continuous controls like sliders and most toggles.
/// - `Occ` (optimistic concurrency): sends `expected_generation`;
///   the server 409s on mismatch — shows a conflict banner + re-resolve.
///   Appropriate when two simultaneous editors overwriting the same
///   slot silently is unacceptable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Concurrency {
    /// Last-write-wins. No generation guard. Default.
    #[default]
    Lww,
    /// Optimistic concurrency check. Server 409s on mismatch.
    Occ,
}

// -------------------------------------------------------------------
// Component
// -------------------------------------------------------------------

/// A single component in the IR tree.
///
/// Discriminated by the `"type"` field on the wire (`#[serde(tag =
/// "type")]`). Variant names are `snake_case` on the wire (`page`,
/// `row`, `col`, …).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Component {
    // ---- layout ---------------------------------------------------
    /// Root component for a resolved page.
    Page {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Buttons rendered in the page header beside the title.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        header_actions: Vec<ToolbarAction>,
        #[serde(default)]
        children: Vec<Component>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
        /// Default horizontal gap between widgets in a row. CSS length
        /// (e.g. `"1rem"`, `"16px"`). Renderer emits this as the
        /// `--rb-row-gap` CSS custom property on the page root; rows
        /// without their own `gap` fall back to it. Absent = renderer
        /// default. Captures intent at the page level so widgets added
        /// later inherit consistent spacing without the authoring layer
        /// needing to know what it is.
        #[serde(skip_serializing_if = "Option::is_none")]
        default_row_gap: Option<String>,
        /// Default vertical gap between rows. CSS length. Emitted as
        /// `--rb-column-gap`; the page wrapper consumes it to space
        /// stacked rows.
        #[serde(skip_serializing_if = "Option::is_none")]
        default_column_gap: Option<String>,
        /// Inner padding of the page wrapper. CSS length applied
        /// symmetrically. Emitted as `--rb-page-padding`.
        #[serde(skip_serializing_if = "Option::is_none")]
        default_page_padding: Option<String>,
        /// Cap on the page wrapper's content width. CSS length
        /// (`"80rem"`) or `"100%"` for no cap. Emitted as
        /// `--rb-page-max-width`.
        #[serde(skip_serializing_if = "Option::is_none")]
        default_max_width: Option<String>,
    },

    /// Horizontal layout container — either a 12-track CSS grid (the
    /// default; cols use `span` to claim cells) or an auto flex row
    /// (cols size to content, packed by `justify`). The `layout` field
    /// picks the mode.
    Row {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        children: Vec<Component>,
        #[serde(skip_serializing_if = "Option::is_none")]
        gap: Option<String>,
        /// Layout primitive. `"grid"` (default) renders a 12-track CSS
        /// grid where children with `type = "col"` claim cells via
        /// their `span`. `"auto"` renders a flex row where children
        /// shrink-wrap to their content width — use this when you
        /// want widgets to sit next to each other without committing
        /// to a column system. Cols inside an `auto` row keep their
        /// `span` value (so toggling back to `grid` is non-lossy) but
        /// the renderer ignores it.
        #[serde(skip_serializing_if = "Option::is_none")]
        layout: Option<RowLayout>,
        /// V1.9 responsive behaviour. When `breakpoints.stack_below`
        /// is set, the row collapses to a column on viewports narrower
        /// than the named breakpoint (`"sm"` or `"md"`).
        #[serde(skip_serializing_if = "Option::is_none")]
        breakpoints: Option<RowBreakpoints>,
        /// Cross-axis alignment of children. For a row this is the
        /// *vertical* axis. Defaults to `stretch`.
        #[serde(skip_serializing_if = "Option::is_none")]
        align: Option<FlexAlign>,
        /// Main-axis distribution of children. For a row this is the
        /// *horizontal* axis. Defaults to `start`. In `grid` layout
        /// this only has visible effect when the cols' spans don't
        /// sum to the full 12 tracks.
        #[serde(skip_serializing_if = "Option::is_none")]
        justify: Option<FlexJustify>,
        /// When `false`, children stay on a single line and overflow.
        /// Default `true` (`flex-wrap`). Only meaningful in `auto`
        /// layout — grid layout never wraps.
        #[serde(skip_serializing_if = "Option::is_none")]
        wrap: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Vertical flex column.
    Col {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        children: Vec<Component>,
        #[serde(skip_serializing_if = "Option::is_none")]
        gap: Option<String>,
        /// 12-grid width when this column is a child of a `row`.
        /// `1..=12`. Absent = auto sizing.
        #[serde(skip_serializing_if = "Option::is_none")]
        span: Option<u8>,
        /// Cross-axis alignment of children. For a col this is the
        /// *horizontal* axis. Defaults to `stretch`.
        #[serde(skip_serializing_if = "Option::is_none")]
        align: Option<FlexAlign>,
        /// Main-axis distribution of children. For a col this is the
        /// *vertical* axis. Defaults to `start`.
        #[serde(skip_serializing_if = "Option::is_none")]
        justify: Option<FlexJustify>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// CSS grid layout.
    Grid {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        children: Vec<Component>,
        /// CSS `grid-template-columns` value, e.g. `"1fr 1fr"`.
        #[serde(skip_serializing_if = "Option::is_none")]
        columns: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Tab container — each tab has a label + child tree.
    Tabs {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        tabs: Vec<Tab>,
        /// When `true`, a tab's `children` only mount on first activation.
        /// Once mounted the tab stays mounted (so SSE subscriptions persist
        /// and switching back is instant). Default `false` for backward
        /// compatibility — all tabs render eagerly.
        #[serde(default)]
        lazy: bool,
        /// When set, the active tab id is mirrored into a query-string param
        /// of this name (e.g. `?tab=permissions`). Reading the param on mount
        /// selects the tab; switching replaces (not pushes) history.
        #[serde(skip_serializing_if = "Option::is_none")]
        url_param: Option<String>,
        /// Tab `id` to select when the URL param is absent or unrecognised.
        /// Falls back to the first tab when absent (existing behaviour).
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
    },

    // ---- iteration ------------------------------------------------
    /// Server-side template expansion. The resolver evaluates `source`
    /// (a binding expression yielding an array), then for each item
    /// instantiates `template` with `$item.*` and `$index` bound to
    /// that item; the result replaces the `Repeat` node in its parent's
    /// `children`. The renderer never sees a `Repeat` — by the time the
    /// tree leaves the agent, every Repeat has been expanded.
    ///
    /// Use cases:
    /// - One tab per entry in a slot array: `tabs.children([repeat(...)])`
    ///   pattern — Tab itself isn't a Component, so wrapper helpers like
    ///   the builder's `tabs_for_each` produce a `Tabs` whose `tabs`
    ///   list is filled at expand time.
    /// - One card per child node, one row per series, etc. Anywhere a
    ///   parent's `children` is currently a static `Vec<Component>`.
    ///
    /// Empty source → zero expansions → the Repeat node disappears.
    /// Unbound or non-array source → resolver issues a binding error
    /// and the Repeat stays in place (the dry-run validator surfaces it).
    Repeat {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Binding expression resolving to a JSON array. Same grammar
        /// as any other binding: `{{$self.gates}}`, `{{$page.items}}`,
        /// `{{$stack.target.children}}`. The surrounding `{{ }}` is
        /// optional — bare `$self.gates` is accepted too.
        source: String,
        /// Optional second binding scope name. When omitted, items are
        /// reachable via `{{$item.*}}`. When set (e.g. `"gate"`), items
        /// are reachable via `{{$gate.*}}` *as well as* `{{$item.*}}` —
        /// the alias makes nested repeats readable.
        #[serde(skip_serializing_if = "Option::is_none")]
        alias: Option<String>,
        /// The component to instantiate per item. Bindings inside it
        /// are evaluated with `$item` and (if set) `$<alias>` populated.
        template: Box<Component>,
    },

    // ---- display --------------------------------------------------
    /// Plain text span.
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        content: String,
        /// Semantic intent: `"info"`, `"success"`, `"warning"`,
        /// `"danger"`, or `null`.
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Section heading (h1–h6).
    Heading {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        content: String,
        /// Optional secondary line rendered below the heading text.
        #[serde(skip_serializing_if = "Option::is_none")]
        subtitle: Option<String>,
        /// 1–6, maps to `<h1>`–`<h6>`. Defaults to 2.
        #[serde(skip_serializing_if = "Option::is_none")]
        level: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Small status pill / tag.
    Badge {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        label: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Unified diff display with optional per-line annotations.
    Diff {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        old_text: String,
        new_text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        annotations: Vec<DiffAnnotation>,
        /// Optional per-line action (e.g. inline comment). `$line`
        /// placeholder is substituted from the click context.
        #[serde(skip_serializing_if = "Option::is_none")]
        line_action: Option<Action>,
    },

    /// Horizontal or vertical separator. Replaces the `text("---")`
    /// hack that block authors used to fake section breaks. Carries
    /// only design tokens — never pixel values — so the renderer
    /// owns the visual mapping.
    ///
    /// All fields are optional; the renderer applies defaults
    /// (`orientation = "horizontal"`, `intent = "muted"`,
    /// `spacing = "md"`).
    ///
    /// Tokens (V1.6 source of truth, V1.1 consumer):
    /// - `orientation`: `"horizontal"` | `"vertical"`
    /// - `intent`: `"default" | "muted" | "primary" | "danger"
    ///   | "warn" | "ok" | "info"`
    /// - `spacing`: `"xs" | "sm" | "md" | "lg" | "xl"`
    Divider {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// `"horizontal"` (default) or `"vertical"`.
        #[serde(skip_serializing_if = "Option::is_none")]
        orientation: Option<String>,
        /// Semantic intent token. Defaults to `"muted"` at render time.
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<String>,
        /// Spacing token (margin around the rule). Defaults to
        /// `"md"` at render time.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<String>,
    },

    /// Atomic form unit (V1.3 of SDUI-VISUAL.md): label + required
    /// `control` + optional helper text + optional structured error.
    /// Decoupled from `form` so authors can interleave fields,
    /// headings, dividers, and standalone widgets in a `col`.
    ///
    /// `control` is non-optional — a field group without a control is
    /// just a `text`. The wire shape mirrors the diagnostics item
    /// (`{ severity, message, code? }`) so action responses can map a
    /// returned diagnostic into a field's error slot without
    /// reshaping.
    FieldGroup {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Visible field label rendered above the control.
        label: String,
        /// The bound input — typically a `text_field` / `select` /
        /// `toggle` / `slider` (V1.4 nodes) or any existing control.
        control: Box<Component>,
        /// Helper / caption text rendered below the control in muted
        /// foreground (e.g. `"Enter your 16-digit number"`).
        #[serde(skip_serializing_if = "Option::is_none")]
        helper: Option<String>,
        /// Structured field error consumed from a diagnostic item.
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<FieldError>,
        /// When true, the renderer marks the label with a required
        /// indicator and exposes `aria-required` on the control.
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Semantic sub-group — a labelled container with its own heading,
    /// optional subtitle, and a child stack with built-in vertical
    /// rhythm. Gives the IR a name for "this is a sub-section inside a
    /// card / page" so the renderer styles consistently and assistive
    /// tech can landmark.
    ///
    /// V1.2 of SDUI-VISUAL.md. Pure layout sugar with no data
    /// bindings — for content with bindings, compose this around a
    /// `card` / `col` / `form` subtree.
    Section {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Section heading text. Required — a section without a title
        /// is just a `col`.
        title: String,
        /// Optional secondary line rendered below the title in muted
        /// foreground.
        #[serde(skip_serializing_if = "Option::is_none")]
        subtitle: Option<String>,
        /// Heading level for the title — 1–6, maps to `<h2>`–`<h6>`.
        /// Defaults to 3 at render time (sections sit inside pages
        /// whose own heading is typically h1/h2).
        #[serde(skip_serializing_if = "Option::is_none")]
        level: Option<u8>,
        /// Optional ARIA landmark role: `"region"`, `"form"`, `"nav"`,
        /// or `"complementary"`. When set, the renderer emits the
        /// matching landmark element so screen readers can navigate.
        #[serde(skip_serializing_if = "Option::is_none")]
        landmark: Option<String>,
        /// Section body — any IR subtree.
        #[serde(default)]
        children: Vec<Component>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    // ---- data -----------------------------------------------------
    /// Time-series chart. Data points are fetched server-side at
    /// resolve time against the node+slot addressed by `source`. Zoom
    /// / pan gestures write `{from, to}` into `$page[page_state_key]`
    /// and re-issue `/ui/resolve`, so the server can return denser
    /// data for the focused window. Live ticks come in via the
    /// subscription plan — the subject is `node.<id>.slot.<slot>`.
    Chart {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// One or more typed sources — V3 supports multi-series authoring.
        /// V2's single `source` becomes a one-element `sources` vector;
        /// the V3→V2 downgrader degrades multi-source charts to
        /// [`Component::Dangling`].
        #[serde(default)]
        sources: Vec<ChartSource>,
        /// Renderer kind. Now actually consumed by the renderer (V2
        /// silently ignored unrecognised strings).
        #[serde(default)]
        kind: ChartKind,
        /// Page-level default aggregation. Per-source aggregations on
        /// [`ChartSource::Rows`] / [`ChartSource::SeriesFromRsql`] win
        /// when set; this is only consulted as a fallback.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agg: Option<AggSpec>,
        /// Server-emitted series payload (one per line / area / bar).
        #[serde(default)]
        series: Vec<ChartSeries>,
        /// Current visible window (inclusive ms since epoch). The
        /// server fills this from `$page.<page_state_key>` or its own
        /// default when the client hasn't zoomed yet.
        #[serde(skip_serializing_if = "Option::is_none")]
        range: Option<ChartRange>,
        /// Client writes zoom / pan state here on `$page`. Defaults to
        /// `"chart_range"` when absent.
        #[serde(skip_serializing_if = "Option::is_none")]
        page_state_key: Option<String>,
        /// Declarative backfill config: on mount the client fetches the
        /// past window then SSE extends the series forward. Absent =
        /// today's behaviour (ad-hoc 1h default inside the client). See
        /// docs/sessions/DASHBOARD-BUILDER.md § "Chart history backfill".
        #[serde(skip_serializing_if = "Option::is_none")]
        history: Option<ChartHistory>,
    },

    /// Compact sparkline — a single line of recent points, no axes, no
    /// interaction. Intended for KPI tiles.
    Sparkline {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Inline values, newest last. Server fills these at resolve.
        #[serde(default)]
        values: Vec<f64>,
        /// Subscription subject for live append (`node.<id>.slot.<s>`).
        #[serde(skip_serializing_if = "Option::is_none")]
        subscribe: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<String>,
        /// Unit symbol appended to aria-label and tooltip (e.g. `"°C"`).
        #[serde(skip_serializing_if = "Option::is_none")]
        unit_symbol: Option<String>,
    },

    /// Server-paginated, sortable table. Rows fetched via
    /// `GET /api/v1/ui/table` (S3); S1 emits the schema only.
    ///
    /// `row_actions` add per-row buttons in a synthetic last column.
    /// `toolbar_actions` add page-level buttons rendered above the
    /// table. Both action lists' `args` may contain `{{$row.<path>}}`
    /// tokens (row_actions only) which the renderer substitutes from
    /// the clicked row before dispatching — handlers receive concrete
    /// values, not the binding string.
    Table {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        source: TableSource,
        columns: Vec<TableColumn>,
        /// Whole-row click — preserved for back-compat (e.g. drilldown
        /// to a detail page). Mutually compatible with `row_actions`.
        #[serde(skip_serializing_if = "Option::is_none")]
        row_action: Option<Action>,
        /// Per-row buttons rendered in a synthetic trailing column.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        row_actions: Vec<RowAction>,
        /// Page-level buttons rendered above the table (e.g. "Add").
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        toolbar_actions: Vec<ToolbarAction>,
        /// When true the renderer shows a search input above the table.
        /// The typed query is appended as a wildcard RSQL clause to the
        /// source query via the `filter` param.
        #[serde(default, skip_serializing_if = "crate::is_false")]
        searchable: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        page_size: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Client-side table rendered from a JSON array binding.
    ///
    /// Unlike [`Component::Table`] (which fetches rows from
    /// `GET /api/v1/ui/table` via RSQL), `ArrayTable` receives its
    /// data from a binding expression that resolves to a JSON array
    /// already present in the page context (e.g. a slot value).
    ///
    /// The renderer parses the array, maps each element through the
    /// column definitions, and renders an in-memory table with
    /// optional row actions. No server pagination — the full array
    /// is rendered client-side.
    ///
    /// Typical source: `"{{$self.slots.services.value}}"` — a slot
    /// containing a JSON-serialised `Vec<ServiceDescriptor>`.
    ArrayTable {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Binding expression resolving to a JSON array. Same grammar
        /// as any read binding (`$self.*`, `$target.*`, `$page.*`).
        source: String,
        columns: Vec<TableColumn>,
        /// Per-row buttons. Args may contain `{{$row.<path>}}` tokens
        /// substituted from the current array element.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        row_actions: Vec<RowAction>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Client-side searchable table over an inline JSON array.
    ///
    /// Distinct from [`Component::Table`] (server-fetched via
    /// `GET /api/v1/ui/table`) and [`Component::ArrayTable`]
    /// (binding-driven over a slot's JSON value). `JsonTable` carries
    /// the rows directly in the IR — used for diagnostic payloads,
    /// preview panels, and any data the backend already has in hand
    /// when it builds the page.
    ///
    /// Columns are derived from row shape when omitted (one level of
    /// dot-flattening for nested objects); supply `columns` to pin
    /// order, rename headers, or override the format of a single
    /// column.
    ///
    /// Timestamp columns auto-format: any column whose key matches
    /// `*_ms` or `*_at` and whose values look like epoch milliseconds
    /// is rendered through the caller's resolved preferences
    /// (timezone, locale, date_format, time_format) by the
    /// renderer. Override per-column with `format: "raw"` to opt out
    /// or `format: "datetime"` to opt in explicitly.
    JsonTable {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// The rows. Empty array is legal — the renderer shows a
        /// "No rows" hint.
        data: Vec<JsonValue>,
        /// Optional explicit column list. Omit for auto-derive.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        columns: Vec<JsonTableColumn>,
        /// When true (default), the renderer shows a search input
        /// that filters across the formatted form of every cell.
        #[serde(
            default = "crate::default_true",
            skip_serializing_if = "crate::is_true"
        )]
        searchable: bool,
        /// Tailwind class capping the table's vertical scroll. The
        /// default is the renderer's choice; provide e.g. `"max-h-64"`
        /// to override.
        #[serde(skip_serializing_if = "Option::is_none")]
        max_height_class: Option<String>,
        /// Placeholder text for the search input.
        #[serde(skip_serializing_if = "Option::is_none")]
        search_placeholder: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Queryable, SSE-aware linear list with a per-row component template.
    ///
    /// Rows are fetched via `GET /api/v1/ui/table` (same plumbing as
    /// [`Component::Table`] — the source DSL is identical). For each row the
    /// renderer substitutes `{{$row.*}}` tokens in the `item` template and
    /// renders the result. SSE `slot_changed` events whose node id matches a
    /// row's `id` replace that row's rendered subtree without a full re-fetch.
    ///
    /// **Token syntax is uniform:** every binding uses `{{ ... }}`. Bare
    /// `$row.id` is not valid; use `"{{$row.id}}"`.
    ///
    /// **Scope:** `$row` is in scope for the entire `item` subtree, including
    /// arbitrarily nested components. It resolves to the row's `UiTableRow`
    /// object — `{{$row.id}}`, `{{$row.path}}`, `{{$row.slots.settings.title}}`.
    List {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// RSQL data source. Same shape as `TableSource`.
        source: TableSource,
        /// Component template rendered once per row. String fields may
        /// contain `{{$row.*}}` substitution tokens.
        item: Box<Component>,
        /// Optional click / tap action fired when the user clicks
        /// anywhere on a row (not on an item-level button). `$row`
        /// is injected into the action args.
        #[serde(skip_serializing_if = "Option::is_none")]
        row_action: Option<Action>,
        /// Number of rows per page (default 20).
        #[serde(skip_serializing_if = "Option::is_none")]
        page_size: Option<u32>,
        /// When `true`, the renderer invalidates the query on every
        /// `node_created` / `node_removed` / `slot_changed` SSE event
        /// whose path matches the source query's `kind==` filter.
        /// Default `false` (polling / manual refresh only).
        #[serde(default)]
        subscribe: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Modal dialog container. Rendered as a shadcn `Dialog`.
    ///
    /// A `dialog` component can be:
    /// - **Static** — authored in the page tree, opened by an `Action` that
    ///   writes `true` into `$page[page_state_key]`.
    /// - **Dynamic** — returned as an `Action` response
    ///   (`{ "type": "dialog", "tree": ... }`) from a handler. The renderer
    ///   pushes a new dialog onto the LIFO dialog stack; responding with a
    ///   non-dialog result pops it.
    ///
    /// `page_state_key` is only used for the static form; dynamic dialogs
    /// are managed entirely by the dialog stack.
    Dialog {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default)]
        children: Vec<Component>,
        /// Buttons rendered in the dialog footer, ordered left-to-right.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        actions: Vec<DialogAction>,
        /// `$page` key that controls open/closed state for **static** dialogs.
        /// Omit for dynamic (action-response) dialogs.
        #[serde(skip_serializing_if = "Option::is_none")]
        page_state_key: Option<String>,
    },

    /// Dropdown / context menu with optional trigger component.
    ///
    /// When `trigger` is set the menu renders as a dropdown button — the
    /// trigger component is rendered inline and clicking it opens the popover.
    /// When `trigger` is absent the component is invisible and must be opened
    /// programmatically (e.g. by a right-click handler on a `list` or `table`
    /// row — see docs/design/extensions/RIGHT-CLICK.md).
    ///
    /// Items are rendered top-to-bottom. `{ type: "separator" }` items render
    /// as a thin `<hr>` divider. All other items carry a `label` and `action`.
    /// A disabled item has no `action` (or the renderer may add an explicit
    /// `disabled: true` field in future — currently absent = enabled).
    Menu {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Optional inline component (typically a [`Component::Button`]) that
        /// opens the menu on click. When absent the host must call a future
        /// `open_menu` action to open it.
        #[serde(skip_serializing_if = "Option::is_none")]
        trigger: Option<Box<Component>>,
        /// Ordered list of menu items.
        items: Vec<MenuItem>,
    },

    /// Hierarchical tree — the sidebar/file-browser shape. Children
    /// arrive pre-expanded; lazy-expand is an S7 refinement.
    Tree {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        nodes: Vec<TreeItem>,
        /// Optional click action — `$node.id` is substituted at
        /// dispatch time.
        #[serde(skip_serializing_if = "Option::is_none")]
        node_action: Option<Action>,
    },

    /// Chronological event list. When `subscribe` is set and `mode` is
    /// `"append"`, incoming NATS messages on the subject are appended
    /// to `events` client-side without a tree re-resolve — the
    /// streaming-text story from SDUI.md § "Streaming content".
    Timeline {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default)]
        events: Vec<TimelineEvent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subscribe: Option<String>,
        /// `"append"` (default — new messages are added to the list)
        /// or `"replace"` (each message replaces the list).
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<String>,
    },

    /// Markdown block. With `subscribe` set, new messages on the
    /// subject append (or replace) the content depending on `mode` —
    /// the UC2 AI-streaming-output primitive.
    Markdown {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subscribe: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<String>,
    },

    // ---- input ----------------------------------------------------
    /// Markdown-aware rich-text editor.
    RichText {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
    },

    /// Two-way bound markdown editor — view/edit toggle backed by a
    /// slot write.  In view mode renders the content as prose; in edit
    /// mode shows a textarea.  Write semantics follow
    /// [`Component::Toggle`] / [`Component::Slider`]: `bind` maps to a
    /// `WritePlanEntry` baked at resolve time; the client sends the
    /// slot write on save.
    MarkdownEditor {
        id: String,
        bind: Bindings,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        /// Initial content baked at resolve time.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// Minimum visible rows in edit mode (default 6).
        #[serde(skip_serializing_if = "Option::is_none")]
        rows: Option<u32>,
    },

    /// Node-graph reference picker — user searches/filters nodes in
    /// the graph, picks one; the form stores its id. UC1 alarm rules,
    /// UC2 settings forms, UC3 scope target pickers.
    RefPicker {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// RSQL filter restricting which nodes the picker offers.
        #[serde(skip_serializing_if = "Option::is_none")]
        query: Option<String>,
        /// Current value — a node id.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
    },

    /// Single-choice dropdown. Writes `value` into
    /// `$page[page_state_key]` on selection. Values can be any JSON
    /// scalar — a `select` over `severity: [low, medium, high]` writes
    /// strings; a severity-as-int select writes numbers. Downstream
    /// components (table `source.query`, chart source, etc.) reference
    /// the same `$page` key via `{{$page.<key>}}` binding substitution.
    Select {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        page_state_key: String,
        options: Vec<SelectOption>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        /// Initial option value applied on mount when the key is
        /// unset. Must be one of `options[].value` if set.
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<JsonValue>,
    },

    /// Big-number stat tile. Reads the current slot value from the
    /// graph at resolve time and lives-updates over the same
    /// subscription plan that powers charts. `format` controls
    /// display: `"number"` (default) | `"percent"` | `"bytes"`.
    Kpi {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        label: String,
        source: ChartSource,
        /// Server-resolved scalar — filled by the chart-source
        /// resolver from `source` at /resolve time. The client
        /// renders this directly. Authors leave it unset; the
        /// resolver overwrites whatever was authored.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<JsonValue>,
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<String>,
        /// Optional comparison delta rendered as "↑ +12% vs prior".
        /// Structured so the renderer can format and localise; extensions
        /// must not pre-format this into a string.
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<KpiDelta>,
        /// Unit symbol to display after the value (e.g. `"°C"`, `"kPa"`,
        /// `"%"`). Set by the resolver from the slot's quantity +
        /// caller unit prefs. Block authors may also set this statically
        /// in a view template; the resolver overwrites it when the slot
        /// has a known quantity.
        #[serde(skip_serializing_if = "Option::is_none")]
        unit_symbol: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Responsive grid of static KPI tiles. `columns` is the number of
    /// columns at "desktop" breakpoint; the renderer collapses to fewer
    /// on smaller viewports. Use `kpi_grid` instead of a hand-rolled
    /// `grid` of `kpi` items to get consistent responsive behaviour.
    KpiGrid {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Number of columns at the desktop breakpoint (default 4).
        #[serde(skip_serializing_if = "Option::is_none")]
        columns: Option<u8>,
        /// Inline KPI tiles — pre-resolved at server resolve time.
        items: Vec<KpiGridItem>,
        /// Optional action fired when the user clicks any tile.
        /// The action context receives `tile_id` (the tile's `id` field)
        /// and `tile_label` so the handler can branch on which tile was
        /// clicked. Use `set_state` with `value_from: "tile_id"` to drive
        /// a page-level filter without custom JS.
        #[serde(skip_serializing_if = "Option::is_none")]
        on_tile_click: Option<Action>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Property-sheet / label-value detail display. Renders as a `dl`
    /// (or shadcn `Card` rows on web). Each row can carry an optional
    /// inline action (e.g. "Edit", "Reassign…").
    Detail {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        items: Vec<DetailItem>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Titled container — closed slot set per V1.5 of SDUI-VISUAL.md:
    /// `title` / `lead` / `content` / `trailing`. `title` and
    /// `subtitle` are strings; `lead` and `trailing` each carry a
    /// single component (typically an icon, avatar, badge, or status
    /// pill); `content` is the existing `children` body.
    ///
    /// Backwards-compatible: pre-V1.5 cards with bare `children` and
    /// `actions` continue to work unchanged. `actions` renders next to
    /// `trailing` in the card header.
    Card {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        subtitle: Option<String>,
        /// Semantic intent: `"info"`, `"success"`, `"warning"`,
        /// `"danger"`, or `null`. Controls the card's accent colour.
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<String>,
        /// Single component rendered to the left of the title (icon,
        /// avatar, leading visual). V1.5: requires `title` to be set
        /// — enforced by the builder; on the wire, a `lead` without a
        /// `title` falls back to rendering as a top-row prefix.
        #[serde(skip_serializing_if = "Option::is_none")]
        lead: Option<Box<Component>>,
        /// Single component rendered at the top-right of the header
        /// (status badge, metadata pill). Sits alongside `actions`.
        #[serde(skip_serializing_if = "Option::is_none")]
        trailing: Option<Box<Component>>,
        /// Action buttons rendered in the card header.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        actions: Vec<CardAction>,
        /// Body content — the `content` slot. Any IR subtree.
        #[serde(default)]
        children: Vec<Component>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Time-range picker with preset buttons. Writes
    /// `{from, to}` (Unix ms) into `$page[page_state_key]` on every
    /// click. `to` is "now" at click time for presets; `null/null`
    /// means "all time". Any component reading the same
    /// `page_state_key` (typically a `chart`) automatically retunes.
    ///
    /// A preset with `duration_ms: null` is "all" / unbounded — the
    /// component writes `null` for `from` (and `to`) so the consumer
    /// understands "no window clamp."
    DateRange {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// `$page` key to write `{from, to}` into. Consumers read the
        /// same key. No default — authors must name it explicitly to
        /// avoid accidental cross-widget coupling on shared pages.
        page_state_key: String,
        /// Ordered preset buttons; the first one is applied on mount
        /// when `$page[page_state_key]` is unset.
        presets: Vec<DateRangePreset>,
    },

    /// Multi-step form. Each step has a nested child tree rendered
    /// one at a time; `submit` fires when the last step is confirmed.
    Wizard {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        steps: Vec<WizardStep>,
        #[serde(skip_serializing_if = "Option::is_none")]
        submit: Option<Action>,
    },

    /// Off-canvas slide-over panel. `open` is bound from `$page`; the
    /// close gesture writes `false` back.
    Drawer {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default)]
        open: bool,
        /// `$page` key that owns the open state. Defaults to
        /// `drawer_<id>`.
        #[serde(skip_serializing_if = "Option::is_none")]
        page_state_key: Option<String>,
        #[serde(default)]
        children: Vec<Component>,
    },

    // ---- interactive ----------------------------------------------
    /// A button that fires an action on click.
    Button {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        label: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        disabled: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Two-way bound boolean toggle (on/off switch). Reads its initial
    /// value from the slot addressed by `bind`; writes back on click
    /// via `POST /api/v1/slots`; reconciles via SSE echo. No
    /// `HandlerRegistry` entry required.
    ///
    /// Renders disabled when no matching `WritePlan` entry exists for
    /// the caller (ACL-denied write, binding error, etc.); the current
    /// slot value remains visible.
    ///
    /// Default concurrency: `lww`. Default debounce: none (fires
    /// immediately on click).
    Toggle {
        id: String,
        /// Binding expression resolving to a boolean slot. Sugar form
        /// or full `BindingSpec`.
        bind: Bindings,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        /// Resolved slot value baked in at resolve time. `null` when
        /// the slot is unset.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Two-way bound numeric slider. Reads its initial value from the
    /// slot addressed by `bind`; debounces writes on drag and fires
    /// once on pointer release. Same write + SSE-echo path as
    /// [`Component::Toggle`].
    ///
    /// `min`, `max`, `step` are **rendering hints only** — they shape
    /// the widget; the slot schema is the authoritative constraint.
    ///
    /// Default concurrency: `lww`. Default debounce: 150 ms trailing +
    /// always fire on pointer release.
    Slider {
        id: String,
        /// Binding expression resolving to a numeric slot.
        bind: Bindings,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        /// Resolved slot value baked in at resolve time.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<f64>,
        /// Rendering hint — minimum value. Default: 0.
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        /// Rendering hint — maximum value. Default: 100.
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        /// Rendering hint — step size. Default: 1.
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    // ---- standalone field nodes (V1.4 of SDUI-VISUAL.md) ----------
    /// Two-way bound text input. Same write plumbing as
    /// [`Component::Toggle`] / [`Component::Slider`] — value is
    /// resolved server-side; client writes via `POST /api/v1/slots`
    /// and reconciles via SSE.
    ///
    /// `format` declares masking the renderer should apply
    /// (`credit_card | phone | postcode | iban | none`). `validate`
    /// is the local validation contract; async / server-side
    /// validation is V2.
    ///
    /// `form_id` ties this field to a `Component::Form` coordinator
    /// (V1.4b). Optional in V1; planned to become required-by-
    /// typestate in V3.3.
    TextField {
        id: String,
        bind: Bindings,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        /// Resolved slot value baked at resolve time.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// Masking hint — closed enum: `"credit_card" | "phone" |
        /// "postcode" | "iban" | "none"`. Defaults to `"none"`.
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        validate: Option<TextValidate>,
        #[serde(skip_serializing_if = "Option::is_none")]
        form_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Two-way bound numeric input. Same plumbing as `text_field`;
    /// the renderer enforces numeric IME / step at the input level.
    NumberField {
        id: String,
        bind: Bindings,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<f64>,
        /// Masking hint — same closed enum as `text_field`.
        #[serde(skip_serializing_if = "Option::is_none")]
        format: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        validate: Option<NumberValidate>,
        #[serde(skip_serializing_if = "Option::is_none")]
        form_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Multi-line text area. Same plumbing as `text_field` minus
    /// `format` (no masking on multi-line).
    Textarea {
        id: String,
        bind: Bindings,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// Visible row count hint. Defaults to 3.
        #[serde(skip_serializing_if = "Option::is_none")]
        rows: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        validate: Option<TextValidate>,
        #[serde(skip_serializing_if = "Option::is_none")]
        form_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Two-way bound single-choice select. Same plumbing as
    /// [`Component::TextField`] but the bound slot is the chosen
    /// option's `value`. Distinct from [`Component::Select`] which
    /// writes to page-state — `SelectField` writes to a slot via the
    /// standard binding pipeline, so it composes with `field_group`,
    /// validation, and the future form coordinator.
    SelectField {
        id: String,
        bind: Bindings,
        options: Vec<SelectOption>,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        /// Resolved slot value baked at resolve time.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<JsonValue>,
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        form_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Two-way bound radio group. Same shape as `select_field` but
    /// renders as a vertical list of radio inputs — useful when the
    /// option count is small (≤5) and the choices benefit from
    /// always-visible labels.
    RadioGroup {
        id: String,
        bind: Bindings,
        options: Vec<SelectOption>,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<JsonValue>,
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        form_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Segmented control — a horizontally-arranged single-choice
    /// picker (think iOS UISegmentedControl). Same wire shape as
    /// `radio_group`; the renderer styles them differently.
    Segmented {
        id: String,
        bind: Bindings,
        options: Vec<SelectOption>,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<JsonValue>,
        #[serde(skip_serializing_if = "Option::is_none")]
        form_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Two-way bound date field. The wire value is an ISO `YYYY-MM-DD`
    /// string regardless of how the renderer displays it (locale /
    /// `prefs.date_format` is render-time concern only). For datetime
    /// values use a string slot with `format` set on the slot schema —
    /// `date_field` is calendar-only.
    DateField {
        id: String,
        bind: Bindings,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        /// Resolved slot value baked at resolve time — always
        /// `YYYY-MM-DD`, never a localised string.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// Earliest selectable date as `YYYY-MM-DD`. Inclusive.
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<String>,
        /// Latest selectable date as `YYYY-MM-DD`. Inclusive.
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        required: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        form_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    /// Two-way bound boolean checkbox. Distinct from `Toggle` — a
    /// checkbox is a form input with an inline label, while `Toggle`
    /// is a switch widget with separate label semantics.
    Checkbox {
        id: String,
        bind: Bindings,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        form_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<NodeStyle>,
    },

    // ---- composite ------------------------------------------------
    /// JSON-Schema-driven form. `schema_ref` is resolved from
    /// bindings; `submit` fires on form submission.
    Form {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// JSON-Schema reference or inline schema.
        ///
        /// The wire type is `String`. When the value is a binding
        /// expression (e.g. `"$target.settings_schema"`), the server
        /// resolves it before emission. When the value is an inline
        /// JSON Schema, it must be serialised with
        /// `serde_json::to_string` — the renderer's `parseSchema()`
        /// calls `JSON.parse()` to recover the object.
        schema_ref: String,
        /// Current form values — resolved from bindings.
        #[serde(skip_serializing_if = "Option::is_none")]
        bindings: Option<JsonValue>,
        #[serde(skip_serializing_if = "Option::is_none")]
        submit: Option<Action>,
        /// Override label for the submit button. When absent the renderer
        /// uses its default ("Submit").
        #[serde(skip_serializing_if = "Option::is_none")]
        submit_label: Option<String>,
    },

    // ---- placeholder stubs ----------------------------------------
    /// ACL-redacted widget — the caller lacks permission to see the
    /// bound data. Renderer shows a neutral stub.
    Forbidden { id: String, reason: String },

    /// Widget whose bound node has been deleted. Renderer shows a
    /// neutral "missing" stub.
    Dangling { id: String },

    // ---- node-actions widget --------------------------------------
    /// Embeddable kind-action card. The renderer resolves
    /// `action_ref` against `GET /api/v1/node/actions?path=<target>`
    /// for the given target node, picks the action with that id, and
    /// renders the form (when `args_schema` is present) or a
    /// confirm/fire button (no-args). Submit calls
    /// `POST /api/v1/ui/action` and replaces the widget content area
    /// with the response (`patch` / `full_render` / `toast`).
    ///
    /// See `rubix-agent/docs/design/node/NODE-ACTIONS.md` § Widget
    /// surface — the manifest entry must declare `surfaces: [widget]`
    /// for this to resolve.
    ActionWidget {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Globally-namespaced action id: `<kind>.<action_id>`. Same
        /// shape returned by the resolve endpoint's `id` field.
        action_ref: String,
        /// Target node path. Bindings (`$target.id`) resolve before
        /// this reaches the renderer.
        target: String,
        /// Optional title override. Falls back to the action's
        /// `display_name` when omitted.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Optional description override. Falls back to the action's
        /// `description` when omitted.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },

    // ---- escape hatch ---------------------------------------------
    /// Opaque custom component rendered by a block-registered
    /// client-side renderer. The server emits `props` verbatim; the
    /// React app looks up `renderer_id` in its component registry and
    /// delegates. Falls back to a neutral stub when the renderer is
    /// not installed.
    ///
    /// Ships in S3 — unblocks UC1 floor-plan, UC2 flow canvas, UC3
    /// state-machine diagram screens before the S4 acceptance demo.
    Custom {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Unique renderer identifier, e.g. `"acme.floorplan"`.
        renderer_id: String,
        /// Opaque props forwarded verbatim to the renderer.
        #[serde(skip_serializing_if = "Option::is_none")]
        props: Option<JsonValue>,
        /// Subscription subjects the renderer wants to watch for live
        /// updates. Mirrors the resolver's subscription plan shape.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subscribe: Vec<String>,
    },
}

// -------------------------------------------------------------------
// Supporting types
// -------------------------------------------------------------------

/// An action reference carried by interactive components.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Action {
    /// Handler name registered in the handler registry, e.g.
    /// `"node.update_settings"`.
    pub handler: String,
    /// Opaque arguments forwarded to the handler.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<JsonValue>,
    /// Client-side hint for making the UI feel instant: apply this
    /// patch to the tree immediately on click; when the server
    /// responds, either it confirms (no-op) or it returns an
    /// authoritative Patch/FullRender that replaces the optimistic
    /// one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimistic: Option<OptimisticHint>,
}

/// Client-side optimistic-update hint — see SDUI.md § "Optimistic
/// hints". Applied before the round-trip fires; the server's response
/// overrides.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OptimisticHint {
    /// Target IR component id to patch. The client walks the current
    /// tree, finds the node with this id, and shallow-merges `fields`
    /// into it.
    pub target_component_id: String,
    /// Object of field-name → value pairs to merge into the target
    /// component. `serde_json::Value` so any typed field can be
    /// updated.
    pub fields: JsonValue,
}

/// Data source for a [`Component::Table`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableSource {
    /// RSQL query string.
    pub query: String,
    /// Whether the client should subscribe to live updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
}

/// Column definition for [`Component::JsonTable`].
///
/// Lighter than [`TableColumn`] because the JsonTable is in-memory and
/// has no sort hint or per-cell render registry — formatting is driven
/// by the column key's shape (the `*_ms` / `*_at` heuristic) plus an
/// optional explicit override.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JsonTableColumn {
    /// The (possibly dot-flattened) row key, e.g. `"payload.ts_ms"`.
    pub key: String,
    /// Header text. Defaults to `key` when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Format hint. `"datetime"` formats epoch-ms through user prefs;
    /// `"raw"` skips the timestamp heuristic. Omit to inherit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<JsonTableColumnFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JsonTableColumnFormat {
    /// Render an epoch-ms integer through the caller's date/time prefs.
    Datetime,
    /// Print the cell value as-is, suppressing the timestamp heuristic.
    Raw,
}

/// Column definition for a [`Component::Table`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableColumn {
    pub title: String,
    /// Dot-path into the row object, e.g. `"slots.present_value.value"`.
    pub field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sortable: Option<bool>,
    /// Optional renderer applied to each cell value before display.
    /// Allows extensions to request semantic formatting without per-block React.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render: Option<ColumnRender>,
}

/// Per-row button rendered in a [`Component::Table`]'s synthetic trailing
/// "Actions" column. Clicking the button dispatches `action`; if `confirm`
/// is set the renderer first shows a confirmation dialog.
///
/// `action.args` may contain `{{$row.<path>}}` tokens (e.g.
/// `{ "path": "{{$row.path}}" }`) which the renderer substitutes from
/// the clicked row before dispatch.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RowAction {
    /// Stable id used for keys / telemetry.
    pub id: String,
    /// Visible button label.
    pub label: String,
    /// Optional icon name (lucide on web; material on Flutter — the
    /// renderer maps the canonical name to its toolkit's icon).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Visual intent: `"primary"` | `"danger"` | `"warning"` | …
    /// Renderers map to button variant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    /// Optional confirm dialog shown before dispatching `action`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm: Option<ConfirmDialog>,
    pub action: Action,
}

/// Page-level button rendered above a [`Component::Table`] (e.g. an
/// "Add Project" or "Export" button). Unlike [`RowAction`], `args`
/// here cannot reference `$row` — there is no row context.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolbarAction {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    pub action: Action,
}

/// Confirm dialog declared by a [`RowAction`]. The renderer shows it
/// before dispatching the action; clicking the cancel button is a
/// no-op, the confirm button fires `action`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConfirmDialog {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Confirm button label. Defaults to `"Confirm"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_label: Option<String>,
    /// Cancel button label. Defaults to `"Cancel"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_label: Option<String>,
}

/// Cell-level renderer hint for [`TableColumn`].
///
/// The SDUI renderer maps each value through a small fixed table so extensions
/// never need custom React to display coloured pills, currencies, or dates.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ColumnRender {
    /// Render the string as an intent-coloured pill (maps common status words
    /// to `default | info | success | warning | danger`).
    IntentPill,
    /// Render using the `Badge` visual primitive.
    Badge,
    /// Format as localised currency (USD by default; use `x-currency` header
    /// annotation to override).
    Currency,
    /// Format as a percentage, e.g. `0.42` → `"42 %"`.
    Percent,
    /// Render a horizontal progress bar (value expected in `[0, 1]`).
    Progress,
    /// Format as a locale date string (ISO 8601 source accepted).
    Date,
    /// Format as a relative time string, e.g. `"3 days ago"`.
    RelativeTime,
    /// Render an interactive boolean Switch widget bound to the row's slot.
    /// The column `field` must point to a boolean slot value
    /// (e.g. `slots.config.enabled.value`); the cell derives the slot path
    /// from the field and writes back via `POST /api/v1/slots` on toggle.
    Toggle,
    /// Render an interactive Slider widget bound to the row's slot.
    /// The column `field` must point to a numeric slot value
    /// (e.g. `slots.config.value.value`); the cell derives the slot path
    /// from the field and writes back on drag release.
    Slider,
}

/// Per-line annotation on a [`Component::Diff`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiffAnnotation {
    pub line: u32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// One node inside a [`Component::Tree`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TreeItem {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub children: Vec<TreeItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// One event inside a [`Component::Timeline`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineEvent {
    /// RFC 3339 timestamp or raw ms-since-epoch string.
    pub ts: String,
    pub text: String,
    /// `"info"` | `"ok"` | `"warn"` | `"danger"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}

/// One entry in a [`Component::Select`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectOption {
    pub label: String,
    pub value: JsonValue,
}

/// One preset button on a [`Component::DateRange`]. `duration_ms` of
/// `None` means "unbounded / all time" — consumers should drop any
/// time clamp when this preset is selected.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DateRangePreset {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

/// One step of a [`Component::Wizard`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WizardStep {
    pub label: String,
    #[serde(default)]
    pub children: Vec<Component>,
}

/// A single tab inside a [`Component::Tabs`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Tab {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub label: String,
    /// Optional icon rendered before the label. Value is a lucide-react
    /// icon name in PascalCase (e.g. `"LayoutGrid"`, `"Workflow"`); the
    /// renderer maps unknown names to no icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub children: Vec<Component>,
}

// -------------------------------------------------------------------
// ShowWhen — V1.7 conditional visibility
// -------------------------------------------------------------------

/// Conditional-visibility predicate for any node. The renderer
/// evaluates this against the page state on every relevant change;
/// when the predicate is false the node is **unmounted** (not hidden).
///
/// V1.7 of SDUI-VISUAL.md. Op set is closed:
/// - `"eq"`, `"ne"` — equality
/// - `"in"`, `"not_in"` — value ∈ array
/// - `"gt"`, `"lt"` — numeric ordering
/// - `"truthy"` — JS-truthy on `binding`; `value` ignored
/// - `"matches"` — `value` is a regex string
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShowWhen {
    /// Binding expression (e.g. `"$page.advanced"`) or literal string
    /// the renderer reads. The page-state bag is the only source —
    /// other binding scopes are not evaluated client-side.
    pub binding: String,
    /// Comparison op — see struct docs for the closed set.
    pub op: String,
    /// Comparison value. Required for every op except `"truthy"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<JsonValue>,
}

// -------------------------------------------------------------------
// FieldError — V1.3 field_group error slot
// -------------------------------------------------------------------

/// Structured field-level error consumed by [`Component::FieldGroup`].
/// Mirrors the diagnostics-item wire shape (severity / message /
/// optional stable code) so a returned diagnostic for a given
/// `field` can drop straight into a field group's `error` slot.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FieldError {
    /// `"error"` | `"warning"` | `"info"`. The renderer picks the
    /// matching colour and icon. Defaults to `"error"` when the
    /// server omits the field.
    #[serde(default = "default_field_error_severity")]
    pub severity: String,
    /// Human, already-localized message rendered verbatim.
    pub message: String,
    /// Optional stable diagnostic code (`block.<id>.<...>`). The
    /// renderer never parses it — it's surfaced only via the DOM
    /// `data-field-error-code` attribute for tests / telemetry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

fn default_field_error_severity() -> String {
    "error".to_string()
}

/// V1.4 local validation contract for `text_field` / `textarea`.
/// Async / server-side validation is V2.3 — `regex` is the only
/// pattern hook today; mismatches produce a client-side error
/// without a round-trip.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct TextValidate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// JS regex source (no flags, no slashes). Anchored
    /// implicitly — the renderer wraps with `^…$`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
}

/// V1.4 local validation contract for `number_field`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct NumberValidate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Number of decimal places permitted. `0` = integers only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<u32>,
}

// -------------------------------------------------------------------
// RowBreakpoints — V1.9 responsive layout
// -------------------------------------------------------------------

/// Responsive layout hints for a [`Component::Row`]. The IR carries
/// only token names; the renderer maps them to its breakpoint system.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RowBreakpoints {
    /// Collapse the row to a column below this breakpoint —
    /// `"sm"` (~640 logical px) or `"md"` (~768 logical px). Above the
    /// breakpoint the row stays horizontal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_below: Option<String>,
}

// -------------------------------------------------------------------
// NodeStyle — Gap #9 style triplet
// -------------------------------------------------------------------

/// Optional rendering-hint triplet for any component node.
///
/// All three fields are optional; the renderer substitutes the
/// component's own context-default when a field is absent.
///
/// These map to design tokens in `@rubix/ui-kit`; extensions must not
/// embed raw hex colors here — use the semantic token names.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct NodeStyle {
    /// Semantic intent colour: `"info"` | `"success"` | `"warning"` |
    /// `"danger"` | `"muted"`. Absent = component default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    /// Spacing density: `"compact"` | `"normal"` | `"comfortable"`.
    /// Absent = component default (usually `"normal"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<String>,
    /// Visual surface elevation: `"default"` | `"raised"` | `"subtle"` |
    /// `"transparent"`. Absent = component default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    /// V1.6 radius token — `"none" | "sm" | "md" | "lg" | "full"`.
    /// Closed set; pixel values must not appear here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<String>,
    /// V1.6 spacing token — `"xs" | "sm" | "md" | "lg" | "xl"`. Applies
    /// as the node's outer breathing room (padding on cards/sections,
    /// margin on standalone primitives). Closed set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<String>,
    /// V1.8 a11y — accessible label for interactive nodes (button,
    /// toggle, slider, select, ref_picker, date_range). Renders as the
    /// element's `aria-label`. Optional in V1; planned to become a
    /// typestate-required field in V3.3 once authoring ergonomics
    /// are reworked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_label: Option<String>,
    /// V1.7 conditional visibility — when set, the renderer
    /// evaluates this predicate every time a binding it reads
    /// changes; a false outcome **unmounts** the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_when: Option<ShowWhen>,
    /// V1.9 layout constraint — flex behaviour when this node is a
    /// child of a `row` or `col`. `"auto"` keeps intrinsic size, `"0"`
    /// disables grow/shrink, `"1"`–`"5"` distribute remaining space
    /// proportionally. Ignored outside row/col parents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flex: Option<String>,
    /// V1.9 layout constraint — minimum width as a token
    /// (`"xs" | "sm" | "md" | "lg" | "xl" | "2xl"`) or `"auto"`.
    /// Pixel values must not appear here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_width: Option<String>,
    /// V1.9 layout constraint — maximum width, token or `"auto"`.
    /// Same scale as `min_width`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<String>,
    /// Per-node override for the parent `row`/`col`'s cross-axis
    /// `align`. Ignored outside row/col parents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_self: Option<FlexAlign>,
    /// Explicit width for this node. Token set:
    /// `"auto"` (intrinsic) | `"fit"` (shrink-wrap content) |
    /// `"full"` (fill parent) | `"xs" | "sm" | "md" | "lg" | "xl" | "2xl"`.
    /// Pixel values must not appear here — use a token. Useful when
    /// you want a button or input narrower than its parent col.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    /// Explicit height. Same token scale as `width` (auto/fit/full
    /// plus the size tokens). Most widgets size to content; this is
    /// for the rare "this card must be 24rem tall" case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
}

// -------------------------------------------------------------------
// RowLayout
// -------------------------------------------------------------------

/// Layout primitive for a [`Component::Row`]. Controls whether the row
/// renders as a 12-track column grid or a content-sized flex strip.
///
/// `Grid` is the default and what most authored layouts want — a
/// stable column system where children claim N of 12 tracks.
///
/// `Auto` is for "I just want these widgets next to each other" cases
/// where the column system is overkill and visually misleading
/// (cols allocating empty whitespace for shrink-wrap content).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RowLayout {
    Grid,
    Auto,
}

// -------------------------------------------------------------------
// FlexAlign / FlexJustify
// -------------------------------------------------------------------

/// Cross-axis alignment shared by `row`, `col`, and `align_self`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FlexAlign {
    Start,
    Center,
    End,
    Stretch,
}

/// Main-axis distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FlexJustify {
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}

/// Comparison delta for a KPI tile — "↑ +12% vs prior period".
///
/// Structured so the renderer formats and localises; extensions must not
/// pre-format this into a string.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KpiDelta {
    /// Numeric magnitude (always non-negative; `direction` encodes sign).
    pub amount: f64,
    /// `"percent"` or `"absolute"` (raw unit value).
    pub unit: String,
    /// `"up"`, `"down"`, or `"flat"`.
    pub direction: String,
    /// Optional intent (`"success"`, `"danger"`, …) — when omitted,
    /// the renderer derives it from direction (up → success, down →
    /// danger, flat → neutral).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}

/// One tile in a `kpi_grid`. Equivalent to the fields on `Kpi` but
/// inlined so the whole grid serialises as a single node.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KpiGridItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub label: String,
    /// Pre-resolved numeric value baked in at resolve time.
    pub value: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<KpiDelta>,
    /// Per-tile click action. Overrides the `kpi_grid`-level `on_tile_click`
    /// when set. The action context receives `tile_id` and `tile_label`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_click: Option<Action>,
    /// Unit symbol displayed after the value (e.g. `"°C"`, `"kPa"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_symbol: Option<String>,
}

/// One row in a `detail` property sheet.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DetailItem {
    pub label: String,
    /// Pre-resolved string value baked in at resolve time.
    pub value: String,
    /// Optional semantic intent (`"info"`, `"success"`, `"warning"`,
    /// `"danger"`). Controls the value text colour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    /// Optional inline action (e.g. a navigation link or dialog open).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    /// Unit symbol appended after the value string (e.g. `"°C"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_symbol: Option<String>,
}

/// One action button in a `card` header.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CardAction {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    pub action: Action,
}

/// One button in a `dialog` footer.
///
/// `action` follows the same dispatch path as `Component::Button`.
/// An `Action` with an empty handler (`handler: ""`) closes the dialog
/// without dispatching — equivalent to "Cancel".
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DialogAction {
    pub label: String,
    /// Semantic intent: `"default"`, `"secondary"`, `"danger"`, or `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    pub action: Action,
}

/// One item in a [`Component::Menu`] items list.
///
/// Use `type: "separator"` for a visual divider. All other items carry a
/// `label` and `action`. `intent` maps to the same tokens as `CardAction`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MenuItem {
    /// A clickable item with a label and action.
    Item {
        label: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<String>,
        action: Action,
    },
    /// A non-interactive horizontal divider line.
    Separator,
}

// -------------------------------------------------------------------
// Portable subset (G5 — cross-platform contract)
// -------------------------------------------------------------------

impl Component {
    /// `true` for variants in the portable IR subset — those a
    /// non-web renderer (react-native, SwiftUI, Flutter) can
    /// implement faithfully without DOM/CSS assumptions. Variants
    /// that embed CSS-length strings, HTML elements as part of their
    /// rendering contract, or verbatim platform-specific props are
    /// not portable; non-web renderers must either implement them
    /// with a documented platform mapping or downgrade them to
    /// `Dangling` via the capability handshake.
    ///
    /// The same set is exported as the runtime-readable list
    /// [`crate::IR_PORTABLE_VARIANTS`].
    pub const fn is_portable(&self) -> bool {
        matches!(
            self,
            Component::Page { .. }
                | Component::Row { .. }
                | Component::Col { .. }
                | Component::Grid { .. }
                | Component::Card { .. }
                | Component::Tabs { .. }
                | Component::Divider { .. }
                | Component::Text { .. }
                | Component::Kpi { .. }
                | Component::Chart { .. }
                | Component::Table { .. }
                | Component::Form { .. }
                | Component::Select { .. }
                | Component::Toggle { .. }
                | Component::Slider { .. }
                | Component::DateRange { .. }
                | Component::RefPicker { .. }
                | Component::Repeat { .. }
        )
    }
}

// -------------------------------------------------------------------
// Synthetic id helpers (G3/G5 — Repeat expansion stable keying)
// -------------------------------------------------------------------

impl Component {
    /// Synthesize a deterministic id for a Repeat-expanded clone.
    /// Format: `"<parent_id>-<index>"`. Used by the Repeat expander
    /// so each per-item clone carries a stable id derived from the
    /// authored parent — re-resolving the same tree produces the
    /// same ids.
    pub fn synthetic_id(parent_id: &str, index: usize) -> String {
        format!("{parent_id}-{index}")
    }

    /// Set this component's id to the synthetic value
    /// `synthetic_id(parent_id, index)` **only when the existing id
    /// is blank** (i.e. `Option::None` or `String::new()` for
    /// required-id variants). Existing ids are preserved.
    pub fn assign_synthetic_id(&mut self, parent_id: &str, index: usize) {
        let synth = Self::synthetic_id(parent_id, index);
        match self {
            // Optional-id variants.
            Component::Row { id, .. }
            | Component::Col { id, .. }
            | Component::Grid { id, .. }
            | Component::Tabs { id, .. }
            | Component::Repeat { id, .. }
            | Component::Text { id, .. }
            | Component::Heading { id, .. }
            | Component::Badge { id, .. }
            | Component::Diff { id, .. }
            | Component::Divider { id, .. }
            | Component::FieldGroup { id, .. }
            | Component::Section { id, .. }
            | Component::Chart { id, .. }
            | Component::Sparkline { id, .. }
            | Component::Table { id, .. }
            | Component::ArrayTable { id, .. }
            | Component::JsonTable { id, .. }
            | Component::List { id, .. }
            | Component::Dialog { id, .. }
            | Component::Menu { id, .. }
            | Component::Tree { id, .. }
            | Component::Timeline { id, .. }
            | Component::Markdown { id, .. }
            | Component::RichText { id, .. }
            | Component::RefPicker { id, .. }
            | Component::Select { id, .. }
            | Component::Kpi { id, .. }
            | Component::KpiGrid { id, .. }
            | Component::Detail { id, .. }
            | Component::Card { id, .. }
            | Component::DateRange { id, .. }
            | Component::Wizard { id, .. }
            | Component::Drawer { id, .. }
            | Component::Button { id, .. }
            | Component::ActionWidget { id, .. }
            | Component::Custom { id, .. }
            | Component::Form { id, .. } => {
                if id.is_none() {
                    *id = Some(synth);
                }
            }
            // Required-id variants: only fill when empty.
            Component::Page { id, .. }
            | Component::MarkdownEditor { id, .. }
            | Component::Toggle { id, .. }
            | Component::Slider { id, .. }
            | Component::TextField { id, .. }
            | Component::NumberField { id, .. }
            | Component::Textarea { id, .. }
            | Component::SelectField { id, .. }
            | Component::RadioGroup { id, .. }
            | Component::Segmented { id, .. }
            | Component::DateField { id, .. }
            | Component::Checkbox { id, .. }
            | Component::Forbidden { id, .. }
            | Component::Dangling { id, .. } => {
                if id.is_empty() {
                    *id = synth;
                }
            }
        }
    }
}

// -------------------------------------------------------------------
// Bindable trait impl
// -------------------------------------------------------------------

// Hand-written `impl Bindable for Component` — forwards `id()` and
// `binding()` to the inherent methods emitted by `#[derive(Bindable)]`
// (proven correct by the field-shape witnesses in the derive's
// expansion) and owns `set_resolved_value` per-variant coerce policy.
//
// Why split the trait impl from the derive: per-variant coerce is
// product policy (a slider wants a number, a toggle a bool); changing
// it should not require touching the proc-macro crate. Keeping it
// here also makes review concrete — the policy decisions sit next to
// the variant declarations they coerce for.
impl crate::Bindable for Component {
    fn id(&self) -> ::core::option::Option<&str> {
        // Hand-written dispatch (replaces `#[derive(Bindable)]` so
        // `starter-ui-ir` stays free of proc-macro deps per R1).
        match self {
            // Variants with required `id: String`.
            Component::Page { id, .. }
            | Component::MarkdownEditor { id, .. }
            | Component::Toggle { id, .. }
            | Component::Slider { id, .. }
            | Component::TextField { id, .. }
            | Component::NumberField { id, .. }
            | Component::Textarea { id, .. }
            | Component::SelectField { id, .. }
            | Component::RadioGroup { id, .. }
            | Component::Segmented { id, .. }
            | Component::DateField { id, .. }
            | Component::Checkbox { id, .. }
            | Component::Forbidden { id, .. }
            | Component::Dangling { id, .. } => Some(id.as_str()),

            // Variants with optional `id: Option<String>`.
            Component::Row { id, .. }
            | Component::Col { id, .. }
            | Component::Grid { id, .. }
            | Component::Tabs { id, .. }
            | Component::Repeat { id, .. }
            | Component::Text { id, .. }
            | Component::Heading { id, .. }
            | Component::Badge { id, .. }
            | Component::Diff { id, .. }
            | Component::Divider { id, .. }
            | Component::FieldGroup { id, .. }
            | Component::Section { id, .. }
            | Component::Chart { id, .. }
            | Component::Sparkline { id, .. }
            | Component::Table { id, .. }
            | Component::ArrayTable { id, .. }
            | Component::JsonTable { id, .. }
            | Component::List { id, .. }
            | Component::Dialog { id, .. }
            | Component::Menu { id, .. }
            | Component::Tree { id, .. }
            | Component::Timeline { id, .. }
            | Component::Markdown { id, .. }
            | Component::RichText { id, .. }
            | Component::RefPicker { id, .. }
            | Component::Select { id, .. }
            | Component::Kpi { id, .. }
            | Component::KpiGrid { id, .. }
            | Component::Detail { id, .. }
            | Component::Card { id, .. }
            | Component::DateRange { id, .. }
            | Component::Wizard { id, .. }
            | Component::Drawer { id, .. }
            | Component::Button { id, .. }
            | Component::Form { id, .. }
            | Component::ActionWidget { id, .. }
            | Component::Custom { id, .. } => id.as_deref(),
        }
    }

    fn read_binding(&self) -> ::core::option::Option<&BindingSpec> {
        // Read-side singular: first entry by convention (SDUI-VALUES.md §3.1).
        match self {
            Component::MarkdownEditor { bind, .. }
            | Component::Toggle { bind, .. }
            | Component::Slider { bind, .. }
            | Component::TextField { bind, .. }
            | Component::NumberField { bind, .. }
            | Component::Textarea { bind, .. }
            | Component::SelectField { bind, .. }
            | Component::RadioGroup { bind, .. }
            | Component::Segmented { bind, .. }
            | Component::DateField { bind, .. }
            | Component::Checkbox { bind, .. } => bind.first(),
            _ => None,
        }
    }

    fn write_bindings(&self) -> &[BindingSpec] {
        // Write-side plural: full slice in declaration order.
        match self {
            Component::MarkdownEditor { bind, .. }
            | Component::Toggle { bind, .. }
            | Component::Slider { bind, .. }
            | Component::TextField { bind, .. }
            | Component::NumberField { bind, .. }
            | Component::Textarea { bind, .. }
            | Component::SelectField { bind, .. }
            | Component::RadioGroup { bind, .. }
            | Component::Segmented { bind, .. }
            | Component::DateField { bind, .. }
            | Component::Checkbox { bind, .. } => bind.as_slice(),
            _ => &[],
        }
    }

    fn set_resolved_value(
        &mut self,
        v: serde_json::Value,
        issues: &mut ::std::vec::Vec<crate::ResolveIssue>,
    ) {
        use crate::bindable::{expected_shape, json_shape};
        use crate::ResolveIssue;

        match self {
            Component::Toggle { id, value, .. } => match v {
                serde_json::Value::Bool(b) => *value = ::core::option::Option::Some(b),
                serde_json::Value::Null => *value = ::core::option::Option::None,
                other => issues.push(ResolveIssue::type_mismatch(
                    ::core::option::Option::Some(id.as_str()),
                    expected_shape::BOOL,
                    json_shape(&other),
                )),
            },
            Component::Slider { id, value, .. } => match v {
                serde_json::Value::Number(n) => {
                    // f64 covers both integer and float JSON numbers
                    // for slider's purposes; an integer slot value is
                    // a perfectly valid slider position. Any number
                    // that overflows f64 falls through to mismatch.
                    *value = n.as_f64();
                    if value.is_none() {
                        issues.push(ResolveIssue::type_mismatch(
                            ::core::option::Option::Some(id.as_str()),
                            expected_shape::NUMBER,
                            "number(unrepresentable as f64)",
                        ));
                    }
                }
                serde_json::Value::Null => *value = ::core::option::Option::None,
                other => issues.push(ResolveIssue::type_mismatch(
                    ::core::option::Option::Some(id.as_str()),
                    expected_shape::NUMBER,
                    json_shape(&other),
                )),
            },
            Component::MarkdownEditor { id, value, .. } => match v {
                serde_json::Value::String(s) => *value = ::core::option::Option::Some(s),
                serde_json::Value::Null => *value = ::core::option::Option::None,
                other => issues.push(ResolveIssue::type_mismatch(
                    ::core::option::Option::Some(id.as_str()),
                    expected_shape::STRING,
                    json_shape(&other),
                )),
            },

            Component::TextField { id, value, .. } => match v {
                serde_json::Value::String(s) => *value = ::core::option::Option::Some(s),
                serde_json::Value::Null => *value = ::core::option::Option::None,
                other => issues.push(ResolveIssue::type_mismatch(
                    ::core::option::Option::Some(id.as_str()),
                    expected_shape::STRING,
                    json_shape(&other),
                )),
            },

            Component::NumberField { id, value, .. } => match v {
                serde_json::Value::Number(n) => {
                    *value = n.as_f64();
                    if value.is_none() {
                        issues.push(ResolveIssue::type_mismatch(
                            ::core::option::Option::Some(id.as_str()),
                            expected_shape::NUMBER,
                            "number(unrepresentable as f64)",
                        ));
                    }
                }
                serde_json::Value::Null => *value = ::core::option::Option::None,
                other => issues.push(ResolveIssue::type_mismatch(
                    ::core::option::Option::Some(id.as_str()),
                    expected_shape::NUMBER,
                    json_shape(&other),
                )),
            },

            Component::Textarea { id, value, .. } => match v {
                serde_json::Value::String(s) => *value = ::core::option::Option::Some(s),
                serde_json::Value::Null => *value = ::core::option::Option::None,
                other => issues.push(ResolveIssue::type_mismatch(
                    ::core::option::Option::Some(id.as_str()),
                    expected_shape::STRING,
                    json_shape(&other),
                )),
            },

            // ISO-8601 string. The IR keeps it as a raw `String` (no
            // chrono dependency on the contract) — the renderer is
            // responsible for parsing into its date control. We do
            // not validate the format here because the slot may
            // legitimately carry partial dates (`"2026-05"`) for
            // month-pickers; the renderer rejects malformed values
            // with its own UI-side hint.
            Component::DateField { id, value, .. } => match v {
                serde_json::Value::String(s) => *value = ::core::option::Option::Some(s),
                serde_json::Value::Null => *value = ::core::option::Option::None,
                other => issues.push(ResolveIssue::type_mismatch(
                    ::core::option::Option::Some(id.as_str()),
                    expected_shape::STRING,
                    json_shape(&other),
                )),
            },

            Component::Checkbox { id, value, .. } => match v {
                serde_json::Value::Bool(b) => *value = ::core::option::Option::Some(b),
                serde_json::Value::Null => *value = ::core::option::Option::None,
                other => issues.push(ResolveIssue::type_mismatch(
                    ::core::option::Option::Some(id.as_str()),
                    expected_shape::BOOL,
                    json_shape(&other),
                )),
            },

            // Pickers (SelectField, RadioGroup, Segmented) treat the
            // bound slot as opaque — the value's `JsonValue` shape
            // is whatever the option set declared. The renderer
            // matches the value against the option list; if no
            // option matches, the renderer paints "no selection".
            // No coerce is meaningful at this layer; we install the
            // raw value as-is and emit no issue. `null` clears the
            // selection.
            Component::SelectField { value, .. } => {
                *value = match v {
                    serde_json::Value::Null => ::core::option::Option::None,
                    other => ::core::option::Option::Some(other),
                };
                let _ = issues;
            }
            Component::RadioGroup { value, .. } => {
                *value = match v {
                    serde_json::Value::Null => ::core::option::Option::None,
                    other => ::core::option::Option::Some(other),
                };
                let _ = issues;
            }
            Component::Segmented { value, .. } => {
                *value = match v {
                    serde_json::Value::Null => ::core::option::Option::None,
                    other => ::core::option::Option::Some(other),
                };
                let _ = issues;
            }

            // Layout-only and other non-bound variants: walker
            // reaches them only via the catch-all that
            // `#[derive(Bindable)]` generates for the `binding()`
            // method. Their `binding()` returns `None`, so
            // `resolve_layout` never calls `set_resolved_value` on
            // them in practice — but the trait method is total over
            // `Component`, and the contract says no-op is correct
            // for any variant that doesn't carry a `value`.
            _ => {
                let _ = (v, issues);
            }
        }
    }

    fn visit_bindings<F>(&mut self, visit: &mut F)
    where
        F: FnMut(&mut String),
    {
        // Per-node string visitor — does NOT recurse into children;
        // the walker in `starter-ui-bindings::substitute` owns
        // descent. Only fields that may carry a `{{...}}` tag are
        // visited; pure layout-token strings (gap, columns, classes)
        // are intentionally excluded.
        match self {
            Component::Page { title, .. } => {
                if let Some(t) = title {
                    visit(t);
                }
            }
            Component::Text { content, .. } | Component::Heading { content, .. } => {
                visit(content);
            }
            Component::Badge { label, .. } => visit(label),
            Component::Section { title, subtitle, .. } => {
                visit(title);
                if let Some(s) = subtitle {
                    visit(s);
                }
            }
            Component::Card { title, subtitle, .. } => {
                if let Some(t) = title {
                    visit(t);
                }
                if let Some(s) = subtitle {
                    visit(s);
                }
            }
            Component::FieldGroup { label, helper, .. } => {
                visit(label);
                if let Some(h) = helper {
                    visit(h);
                }
            }
            Component::Tabs { tabs, default, .. } => {
                for tab in tabs {
                    visit(&mut tab.label);
                    if let Some(id) = &mut tab.id {
                        visit(id);
                    }
                }
                if let Some(d) = default {
                    visit(d);
                }
            }
            Component::Repeat { source, .. } => {
                visit(source);
            }
            Component::Chart { sources, .. } => {
                for src in sources {
                    match src {
                        crate::ChartSource::Series { node_id, slot, field } => {
                            visit(node_id);
                            visit(slot);
                            if let Some(f) = field {
                                visit(f);
                            }
                        }
                        crate::ChartSource::SeriesByKind { kind, slot, field, .. } => {
                            visit(kind);
                            visit(slot);
                            if let Some(f) = field {
                                visit(f);
                            }
                        }
                        crate::ChartSource::Rows { rsql, group_by, .. } => {
                            visit(rsql);
                            if let Some(g) = group_by {
                                visit(g);
                            }
                        }
                        crate::ChartSource::SeriesFromRsql { rsql, group_by, .. } => {
                            visit(rsql);
                            if let Some(g) = group_by {
                                visit(g);
                            }
                        }
                        crate::ChartSource::Static { .. }
                        | crate::ChartSource::AnalyticsTemplate { .. }
                        | crate::ChartSource::Unknown => {}
                    }
                }
            }
            Component::Sparkline { subscribe, .. } => {
                if let Some(s) = subscribe {
                    visit(s);
                }
            }
            Component::Table { source, columns, row_actions, .. } => {
                visit(&mut source.query);
                for col in columns {
                    visit(&mut col.field);
                    visit(&mut col.title);
                }
                for ra in row_actions {
                    if let Some(args) = ra.action.args.as_mut() {
                        visit_json_strings(args, visit);
                    }
                }
            }
            Component::ArrayTable { source, columns, row_actions, .. } => {
                visit(source);
                for col in columns {
                    visit(&mut col.field);
                    visit(&mut col.title);
                }
                for ra in row_actions {
                    if let Some(args) = ra.action.args.as_mut() {
                        visit_json_strings(args, visit);
                    }
                }
            }
            Component::JsonTable { .. } => {}
            Component::List { source, .. } => {
                visit(&mut source.query);
            }
            Component::Dialog { title, description, .. } => {
                if let Some(t) = title {
                    visit(t);
                }
                if let Some(d) = description {
                    visit(d);
                }
            }
            Component::Markdown { content, subscribe, .. } => {
                if let Some(c) = content {
                    visit(c);
                }
                if let Some(s) = subscribe {
                    visit(s);
                }
            }
            Component::RichText { value, .. } => {
                if let Some(v) = value {
                    visit(v);
                }
            }
            Component::MarkdownEditor { label, value, .. } => {
                if let Some(l) = label {
                    visit(l);
                }
                if let Some(v) = value {
                    visit(v);
                }
            }
            Component::RefPicker { query, value, .. } => {
                if let Some(q) = query {
                    visit(q);
                }
                if let Some(v) = value {
                    visit(v);
                }
            }
            Component::Select { options, .. } => {
                for opt in options {
                    visit(&mut opt.label);
                }
            }
            Component::Kpi { label, source, unit_symbol, .. } => {
                visit(label);
                if let Some(u) = unit_symbol {
                    visit(u);
                }
                // ChartSource — same logic as Chart, but for one source.
                match source {
                    crate::ChartSource::Series { node_id, slot, field } => {
                        visit(node_id);
                        visit(slot);
                        if let Some(f) = field {
                            visit(f);
                        }
                    }
                    crate::ChartSource::SeriesByKind { kind, slot, field, .. } => {
                        visit(kind);
                        visit(slot);
                        if let Some(f) = field {
                            visit(f);
                        }
                    }
                    crate::ChartSource::Rows { rsql, group_by, .. } => {
                        visit(rsql);
                        if let Some(g) = group_by {
                            visit(g);
                        }
                    }
                    crate::ChartSource::SeriesFromRsql { rsql, group_by, .. } => {
                        visit(rsql);
                        if let Some(g) = group_by {
                            visit(g);
                        }
                    }
                    crate::ChartSource::Static { .. }
                    | crate::ChartSource::AnalyticsTemplate { .. }
                    | crate::ChartSource::Unknown => {}
                }
            }
            Component::KpiGrid { items, .. } => {
                for item in items {
                    visit(&mut item.label);
                    if let Some(u) = &mut item.unit_symbol {
                        visit(u);
                    }
                }
            }
            Component::Detail { items, .. } => {
                for item in items {
                    visit(&mut item.label);
                    visit(&mut item.value);
                    if let Some(u) = &mut item.unit_symbol {
                        visit(u);
                    }
                }
            }
            Component::Button { label, .. } => visit(label),
            Component::Toggle { label, .. }
            | Component::Slider { label, .. } => {
                if let Some(l) = label {
                    visit(l);
                }
            }
            Component::TextField { label, placeholder, value, .. }
            | Component::Textarea { label, placeholder, value, .. }
            | Component::DateField { label, placeholder, value, .. } => {
                if let Some(l) = label {
                    visit(l);
                }
                if let Some(p) = placeholder {
                    visit(p);
                }
                if let Some(v) = value {
                    visit(v);
                }
            }
            Component::NumberField { label, placeholder, .. } => {
                if let Some(l) = label {
                    visit(l);
                }
                if let Some(p) = placeholder {
                    visit(p);
                }
            }
            Component::SelectField { label, placeholder, options, .. } => {
                if let Some(l) = label {
                    visit(l);
                }
                if let Some(p) = placeholder {
                    visit(p);
                }
                for opt in options {
                    visit(&mut opt.label);
                }
            }
            Component::RadioGroup { label, options, .. } => {
                if let Some(l) = label {
                    visit(l);
                }
                for opt in options {
                    visit(&mut opt.label);
                }
            }
            Component::Segmented { label, options, .. } => {
                if let Some(l) = label {
                    visit(l);
                }
                for opt in options {
                    visit(&mut opt.label);
                }
            }
            Component::Checkbox { label, .. } => {
                if let Some(l) = label {
                    visit(l);
                }
            }
            Component::Form { schema_ref, submit_label, .. } => {
                visit(schema_ref);
                if let Some(s) = submit_label {
                    visit(s);
                }
            }
            Component::ActionWidget {
                action_ref,
                target,
                title,
                description,
                ..
            } => {
                visit(action_ref);
                visit(target);
                if let Some(t) = title {
                    visit(t);
                }
                if let Some(d) = description {
                    visit(d);
                }
            }
            Component::Drawer { title, .. } => {
                if let Some(t) = title {
                    visit(t);
                }
            }
            Component::Wizard { steps, .. } => {
                for step in steps {
                    visit(&mut step.label);
                }
            }
            Component::Timeline { events, subscribe, .. } => {
                for ev in events {
                    visit(&mut ev.text);
                }
                if let Some(s) = subscribe {
                    visit(s);
                }
            }
            Component::Tree { nodes, .. } => {
                for n in nodes {
                    visit(&mut n.label);
                }
            }
            Component::Menu { .. }
            | Component::Diff { .. }
            | Component::Divider { .. }
            | Component::Row { .. }
            | Component::Col { .. }
            | Component::Grid { .. }
            | Component::DateRange { .. }
            | Component::Forbidden { .. }
            | Component::Dangling { .. }
            | Component::Custom { .. } => {
                // No bindable string fields at this node level
                // (children are visited by the walker, not here).
                let _ = visit;
            }
        }
    }
}

/// Recursively walk a `serde_json::Value` and apply `visit` to every
/// string inside. Used for `Action.args` JSON which can carry
/// `{{...}}` tokens in arbitrary nested positions.
fn visit_json_strings<F: FnMut(&mut String)>(v: &mut JsonValue, visit: &mut F) {
    match v {
        JsonValue::String(s) => visit(s),
        JsonValue::Array(arr) => {
            for item in arr {
                visit_json_strings(item, visit);
            }
        }
        JsonValue::Object(map) => {
            for (_k, val) in map.iter_mut() {
                visit_json_strings(val, visit);
            }
        }
        _ => {}
    }
}

// -------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bindable_text_field_coerces_string_and_flags_mismatch() {
        use crate::Bindable;

        let mut c = Component::TextField {
            id: "tf".into(),
            bind: Bindings::one(BindingSpec::Short("$target.s".into())),
            label: None,
            placeholder: None,
            value: None,
            format: None,
            validate: None,
            form_id: None,
            style: None,
        };

        let mut issues = Vec::new();
        c.set_resolved_value(json!("hello"), &mut issues);
        assert!(issues.is_empty());
        if let Component::TextField { value, .. } = &c {
            assert_eq!(value.as_deref(), Some("hello"));
        } else {
            panic!("variant changed");
        }

        let mut issues = Vec::new();
        c.set_resolved_value(json!(42), &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0],
            crate::ResolveIssue::TypeMismatch {
                expected: "string",
                ..
            }
        ));
    }

    #[test]
    fn bindable_number_field_coerces_number_and_flags_mismatch() {
        use crate::Bindable;

        let mut c = Component::NumberField {
            id: "nf".into(),
            bind: Bindings::one(BindingSpec::Short("$target.n".into())),
            label: None,
            placeholder: None,
            value: None,
            format: None,
            validate: None,
            form_id: None,
            style: None,
        };

        let mut issues = Vec::new();
        c.set_resolved_value(json!(7.5), &mut issues);
        assert!(issues.is_empty());
        if let Component::NumberField { value, .. } = &c {
            assert_eq!(*value, Some(7.5));
        }

        let mut issues = Vec::new();
        c.set_resolved_value(json!("not a number"), &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0],
            crate::ResolveIssue::TypeMismatch {
                expected: "number",
                ..
            }
        ));
    }

    #[test]
    fn bindable_textarea_coerces_string_and_flags_mismatch() {
        use crate::Bindable;

        let mut c = Component::Textarea {
            id: "ta".into(),
            bind: Bindings::one(BindingSpec::Short("$target.body".into())),
            label: None,
            placeholder: None,
            value: None,
            rows: None,
            validate: None,
            form_id: None,
            style: None,
        };

        let mut issues = Vec::new();
        c.set_resolved_value(json!("multi\nline"), &mut issues);
        assert!(issues.is_empty());
        if let Component::Textarea { value, .. } = &c {
            assert_eq!(value.as_deref(), Some("multi\nline"));
        }

        let mut issues = Vec::new();
        c.set_resolved_value(json!(true), &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0],
            crate::ResolveIssue::TypeMismatch {
                expected: "string",
                ..
            }
        ));
    }

    #[test]
    fn bindable_date_field_coerces_string_and_flags_mismatch() {
        use crate::Bindable;

        let mut c = Component::DateField {
            id: "df".into(),
            bind: Bindings::one(BindingSpec::Short("$target.when".into())),
            label: None,
            placeholder: None,
            value: None,
            min: None,
            max: None,
            required: None,
            form_id: None,
            style: None,
        };

        let mut issues = Vec::new();
        c.set_resolved_value(json!("2026-05-10"), &mut issues);
        assert!(issues.is_empty());
        if let Component::DateField { value, .. } = &c {
            assert_eq!(value.as_deref(), Some("2026-05-10"));
        }

        let mut issues = Vec::new();
        c.set_resolved_value(json!(202605), &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0],
            crate::ResolveIssue::TypeMismatch {
                expected: "string",
                ..
            }
        ));
    }

    #[test]
    fn bindable_checkbox_coerces_bool_and_flags_mismatch() {
        use crate::Bindable;

        let mut c = Component::Checkbox {
            id: "cb".into(),
            bind: Bindings::one(BindingSpec::Short("$target.agreed".into())),
            label: None,
            value: None,
            form_id: None,
            style: None,
        };

        let mut issues = Vec::new();
        c.set_resolved_value(json!(true), &mut issues);
        assert!(issues.is_empty());
        if let Component::Checkbox { value, .. } = &c {
            assert_eq!(*value, Some(true));
        }

        let mut issues = Vec::new();
        c.set_resolved_value(json!("yes"), &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            issues[0],
            crate::ResolveIssue::TypeMismatch {
                expected: "bool",
                ..
            }
        ));
    }

    #[test]
    fn bindable_select_field_passes_value_through() {
        use crate::Bindable;

        let mut c = Component::SelectField {
            id: "sf".into(),
            bind: Bindings::one(BindingSpec::Short("$target.choice".into())),
            options: vec![],
            label: None,
            placeholder: None,
            value: None,
            required: None,
            form_id: None,
            style: None,
        };

        // Pickers carry opaque JsonValue. The coerce arm installs
        // any non-null value verbatim; null clears.
        let mut issues = Vec::new();
        c.set_resolved_value(json!("opt-a"), &mut issues);
        assert!(issues.is_empty());
        if let Component::SelectField { value, .. } = &c {
            assert_eq!(value.as_ref(), Some(&json!("opt-a")));
        }

        let mut issues = Vec::new();
        c.set_resolved_value(json!(null), &mut issues);
        assert!(issues.is_empty());
        if let Component::SelectField { value, .. } = &c {
            assert!(value.is_none());
        }

        // A complex value (e.g. a JSON object) is also acceptable —
        // the option list is the authority on what's valid; the
        // resolver doesn't second-guess.
        let mut issues = Vec::new();
        c.set_resolved_value(json!({"key": "value"}), &mut issues);
        assert!(issues.is_empty());
    }

    #[test]
    fn bindable_radio_group_passes_value_through() {
        use crate::Bindable;

        let mut c = Component::RadioGroup {
            id: "rg".into(),
            bind: Bindings::one(BindingSpec::Short("$target.pick".into())),
            options: vec![],
            label: None,
            value: None,
            required: None,
            form_id: None,
            style: None,
        };

        let mut issues = Vec::new();
        c.set_resolved_value(json!(2), &mut issues);
        assert!(issues.is_empty());
        if let Component::RadioGroup { value, .. } = &c {
            assert_eq!(value.as_ref(), Some(&json!(2)));
        }
    }

    #[test]
    fn bindable_segmented_passes_value_through() {
        use crate::Bindable;

        let mut c = Component::Segmented {
            id: "sg".into(),
            bind: Bindings::one(BindingSpec::Short("$target.seg".into())),
            options: vec![],
            label: None,
            value: None,
            form_id: None,
            style: None,
        };

        let mut issues = Vec::new();
        c.set_resolved_value(json!("medium"), &mut issues);
        assert!(issues.is_empty());
        if let Component::Segmented { value, .. } = &c {
            assert_eq!(value.as_ref(), Some(&json!("medium")));
        }
    }

    #[test]
    fn bindable_toggle_coerces_bool_and_flags_mismatch() {
        use crate::Bindable;

        let mut c = Component::Toggle {
            id: "t1".into(),
            bind: Bindings::one(BindingSpec::Short("$target.enabled".into())),
            label: None,
            value: None,
            style: None,
        };

        // id() returns Some on a `String`-typed id.
        assert_eq!(c.id(), Some("t1"));
        // read_binding() returns Some because the variant carries a bind;
        // write_bindings() exposes the same single entry.
        assert!(c.read_binding().is_some());
        assert_eq!(c.write_bindings().len(), 1);

        // Coerce a real bool.
        let mut issues = Vec::new();
        c.set_resolved_value(json!(true), &mut issues);
        assert!(issues.is_empty());
        if let Component::Toggle { value, .. } = &c {
            assert_eq!(*value, Some(true));
        } else {
            panic!("variant changed");
        }

        // Mismatched type emits a TypeMismatch issue, leaves value unset.
        let mut issues = Vec::new();
        if let Component::Toggle { value, .. } = &mut c {
            *value = None;
        }
        c.set_resolved_value(json!("not a bool"), &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            &issues[0],
            crate::ResolveIssue::TypeMismatch {
                expected: "bool",
                ..
            }
        ));
        if let Component::Toggle { value, .. } = &c {
            assert!(value.is_none());
        }
    }

    #[test]
    fn bindable_layout_variant_returns_none_binding() {
        use crate::Bindable;

        let c = Component::Row {
            id: Some("r1".into()),
            children: vec![],
            gap: None,
            layout: None,
            breakpoints: None,
            align: None,
            justify: None,
            wrap: None,
            style: None,
        };
        // Row has Option<String> id, so id() returns Some when set.
        assert_eq!(c.id(), Some("r1"));
        // Row has no `bind` field; read/write must reflect that.
        assert!(c.read_binding().is_none());
        assert!(c.write_bindings().is_empty());
    }

    #[test]
    fn page_serialises_as_type_page() {
        let c = Component::Page {
            id: "p1".into(),
            title: Some("Hello".into()),
            header_actions: vec![],
            children: vec![],
            style: None,
            default_row_gap: None,
            default_column_gap: None,
            default_page_padding: None,
            default_max_width: None,
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "page");
        assert_eq!(v["id"], "p1");
    }

    #[test]
    fn forbidden_round_trip() {
        let c = Component::Forbidden {
            id: "w1".into(),
            reason: "acl".into(),
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Component = serde_json::from_str(&json).unwrap();
        match back {
            Component::Forbidden { id, reason } => {
                assert_eq!(id, "w1");
                assert_eq!(reason, "acl");
            }
            other => panic!("expected Forbidden, got {other:?}"),
        }
    }

    #[test]
    fn table_with_source_and_columns() {
        let c = Component::Table {
            id: Some("tbl".into()),
            source: TableSource {
                query: "kind==sys.driver.point".into(),
                subscribe: Some(true),
            },
            columns: vec![TableColumn {
                title: "Name".into(),
                field: "path".into(),
                sortable: Some(true),
                render: None,
            }],
            row_action: None,
            row_actions: vec![],
            toolbar_actions: vec![],
            searchable: false,
            page_size: Some(50),
            style: None,
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "table");
        assert_eq!(v["source"]["query"], "kind==sys.driver.point");
        assert_eq!(v["columns"][0]["title"], "Name");
    }

    #[test]
    fn form_with_schema_ref() {
        let c = Component::Form {
            id: Some("f1".into()),
            schema_ref: "$target.settings_schema".into(),
            bindings: Some(json!({"name": "test"})),
            submit: Some(Action {
                handler: "node.update_settings".into(),
                args: Some(json!({"target": "$target.id"})),
                optimistic: None,
            }),
            submit_label: None,
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "form");
        assert_eq!(v["schema_ref"], "$target.settings_schema");
    }

    #[test]
    fn table_row_actions_round_trip() {
        let c = Component::Table {
            id: Some("tbl".into()),
            source: TableSource {
                query: "kind==com.acme.thing".into(),
                subscribe: None,
            },
            columns: vec![TableColumn {
                title: "Path".into(),
                field: "path".into(),
                sortable: Some(true),
                render: None,
            }],
            row_action: None,
            row_actions: vec![RowAction {
                id: "delete".into(),
                label: "Delete".into(),
                icon: Some("trash".into()),
                intent: Some("danger".into()),
                confirm: Some(ConfirmDialog {
                    title: "Delete this?".into(),
                    description: Some("Cannot be undone.".into()),
                    confirm_label: None,
                    cancel_label: None,
                }),
                action: Action {
                    handler: "thing.delete".into(),
                    args: Some(json!({ "path": "{{$row.path}}" })),
                    optimistic: None,
                },
            }],
            toolbar_actions: vec![ToolbarAction {
                id: "create".into(),
                label: "Add".into(),
                icon: Some("plus".into()),
                intent: Some("primary".into()),
                action: Action {
                    handler: "thing.create_dialog".into(),
                    args: None,
                    optimistic: None,
                },
            }],
            searchable: false,
            page_size: None,
            style: None,
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "table");
        assert_eq!(v["row_actions"][0]["id"], "delete");
        assert_eq!(v["row_actions"][0]["confirm"]["title"], "Delete this?");
        assert_eq!(v["toolbar_actions"][0]["id"], "create");
        // Empty defaults must elide on the wire so v3 clients see a v3-shaped table.
        let bare = Component::Table {
            id: None,
            source: TableSource {
                query: "k==x".into(),
                subscribe: None,
            },
            columns: vec![],
            row_action: None,
            row_actions: vec![],
            toolbar_actions: vec![],
            searchable: false,
            page_size: None,
            style: None,
        };
        let bare_v = serde_json::to_value(&bare).unwrap();
        assert!(
            bare_v.get("row_actions").is_none(),
            "empty row_actions must elide"
        );
        assert!(
            bare_v.get("toolbar_actions").is_none(),
            "empty toolbar_actions must elide"
        );
        // Round-trip.
        let _back: Component = serde_json::from_value(v).unwrap();
    }

    #[test]
    fn diff_with_annotations() {
        let c = Component::Diff {
            id: None,
            old_text: "a\nb\n".into(),
            new_text: "a\nc\n".into(),
            language: Some("rust".into()),
            annotations: vec![DiffAnnotation {
                line: 2,
                text: "changed line".into(),
                author: Some("alice".into()),
                created_at: None,
            }],
            line_action: None,
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Component = serde_json::from_str(&json).unwrap();
        match back {
            Component::Diff { annotations, .. } => assert_eq!(annotations.len(), 1),
            other => panic!("expected Diff, got {other:?}"),
        }
    }

    #[test]
    fn component_json_schema() {
        let schema = schemars::schema_for!(Component);
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("\"type\""));
    }

    #[test]
    fn custom_escape_hatch_round_trip() {
        let c = Component::Custom {
            id: Some("map1".into()),
            renderer_id: "acme.floorplan".into(),
            props: Some(serde_json::json!({ "floor": 2 })),
            subscribe: vec!["node.123.slot.state".into()],
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "custom");
        assert_eq!(v["renderer_id"], "acme.floorplan");
        assert_eq!(v["subscribe"][0], "node.123.slot.state");
        let back: Component = serde_json::from_value(v).unwrap();
        assert!(matches!(back, Component::Custom { .. }));
    }

    #[test]
    fn toggle_short_form_round_trip() {
        let c = Component::Toggle {
            id: "t1".into(),
            bind: Bindings::one(BindingSpec::Short("$target.enabled".into())),
            label: Some("Enabled".into()),
            value: Some(true),
            style: None,
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "toggle");
        assert_eq!(v["id"], "t1");
        // Short form serialises as a bare string in the `bind` field.
        assert_eq!(v["bind"], "$target.enabled");
        assert_eq!(v["value"], true);

        let back: Component = serde_json::from_value(v).unwrap();
        match back {
            Component::Toggle { id, bind, .. } => {
                assert_eq!(id, "t1");
                assert_eq!(bind.slot_expr(), Some("$target.enabled"));
                assert_eq!(bind.concurrency(), Some(Concurrency::Lww));
            }
            other => panic!("expected Toggle, got {other:?}"),
        }
    }

    #[test]
    fn toggle_full_binding_form() {
        let c = Component::Toggle {
            id: "t2".into(),
            bind: Bindings::one(BindingSpec::Full {
                slot: "$target.enabled".into(),
                concurrency: Concurrency::Occ,
                debounce_ms: None,
            }),
            label: None,
            value: Some(false),
            style: None,
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["bind"]["slot"], "$target.enabled");
        assert_eq!(v["bind"]["concurrency"], "occ");
        // debounce_ms absent when None
        assert!(v["bind"].get("debounce_ms").is_none() || v["bind"]["debounce_ms"].is_null());

        let back: Component = serde_json::from_value(v).unwrap();
        let bind = match back {
            Component::Toggle { bind, .. } => bind,
            other => panic!("expected Toggle, got {other:?}"),
        };
        assert_eq!(bind.concurrency(), Some(Concurrency::Occ));
    }

    #[test]
    fn slider_round_trip() {
        let c = Component::Slider {
            id: "s1".into(),
            bind: Bindings::one(BindingSpec::Short("$target.brightness".into())),
            label: Some("Brightness".into()),
            value: Some(42.0),
            min: Some(0.0),
            max: Some(100.0),
            step: Some(1.0),
            style: None,
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "slider");
        assert_eq!(v["id"], "s1");
        assert_eq!(v["bind"], "$target.brightness");
        assert_eq!(v["min"], 0.0);
        assert_eq!(v["max"], 100.0);
        assert_eq!(v["value"], 42.0);

        let back: Component = serde_json::from_value(v).unwrap();
        assert!(matches!(back, Component::Slider { .. }));
    }

    #[test]
    fn bindings_accepts_string_object_and_array_forms() {
        // String form — the common case; sugar for one Short.
        let s: Bindings = serde_json::from_value(json!("$target.a")).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s.slot_expr(), Some("$target.a"));

        // Object form — one Full entry with modifiers.
        let o: Bindings = serde_json::from_value(json!({
            "slot": "$target.a",
            "concurrency": "occ",
            "debounce_ms": 250,
        }))
        .unwrap();
        assert_eq!(o.len(), 1);
        assert_eq!(o.concurrency(), Some(Concurrency::Occ));

        // Array form — fan-out. Each element is its own spec, in
        // declaration order. Mixing Short and Full in one array is
        // allowed; the wire form per-element is just BindingSpec.
        let a: Bindings = serde_json::from_value(json!([
            "$target.a",
            { "slot": "$mirror.value", "concurrency": "lww", "debounce_ms": 500 },
            "$target/notes.body",
        ]))
        .unwrap();
        assert_eq!(a.len(), 3);
        assert_eq!(a.0[0].slot_expr(), "$target.a");
        assert_eq!(a.0[1].slot_expr(), "$mirror.value");
        assert_eq!(a.0[1].debounce_ms(), Some(500));
        assert_eq!(a.0[2].slot_expr(), "$target/notes.body");
    }

    #[test]
    fn slider_fan_out_round_trip() {
        // Two-element bind array on a Slider — the fan-out shape from
        // SDUI-VALUES.md §3.1. Round-trip through JSON proves the
        // wire form (array) deserialises back into the typed
        // `Bindings` and that `write_bindings()` exposes both entries
        // in declaration order.
        use crate::Bindable;

        let c = Component::Slider {
            id: "fan".into(),
            bind: Bindings(vec![
                BindingSpec::Short("$target.a".into()),
                BindingSpec::Short("$mirror.value".into()),
            ]),
            label: None,
            value: None,
            min: None,
            max: None,
            step: None,
            style: None,
        };

        // Serialised form is an array; multi-entry never collapses.
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["bind"], json!(["$target.a", "$mirror.value"]));

        // Round-trip preserves both entries.
        let back: Component = serde_json::from_value(v).unwrap();
        let writes = back.write_bindings();
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[0].slot_expr(), "$target.a");
        assert_eq!(writes[1].slot_expr(), "$mirror.value");
        // Read source is the first entry by convention.
        assert_eq!(
            back.read_binding().map(BindingSpec::slot_expr),
            Some("$target.a"),
        );
    }

    #[test]
    fn concurrency_default_is_lww() {
        let c = Concurrency::default();
        assert_eq!(c, Concurrency::Lww);
        let v = serde_json::to_value(c).unwrap();
        assert_eq!(v, "lww");
    }
}
