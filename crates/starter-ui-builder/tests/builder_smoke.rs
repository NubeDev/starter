//! Phase 3 smoke — "Builder DSL produces valid IR".
//!
//! Every public builder function in `starter-ui-builder` is
//! exercised; the resulting `Component` is wrapped in a
//! [`ComponentTree`] and round-tripped through the IR's typed
//! deserialiser (the runtime validator-of-record for the wire
//! shape) as well as the committed JSON Schema artifact for the
//! variants whose schemars emission matches the runtime
//! `Serialize` impl.
//!
//! The JSON Schema artifact in
//! `crates/starter-ui-ir/schema/starter-ui-ir.schema.json` ships
//! today with two known Phase 1 divergences from the wire shape —
//! `Bindings` (custom `Serialize` unwraps a single spec, schemars
//! emits an array) and `ChartKind` (custom `Serialize` emits
//! snake_case, schemars emits PascalCase variant names). Variants
//! that touch those types are validated via serde round-trip only.
//! Every other variant gets full JSON Schema validation. When the
//! schemars output is reconciled with the wire shape, the
//! variant-allowlist below collapses to one path.

use std::sync::OnceLock;

use jsonschema::JSONSchema;
use serde_json::Value;

use starter_ui_builder::prelude::*;

/// Compile the IR JSON Schema once per process; downstream tests
/// reuse the validator handle.
fn validator() -> &'static JSONSchema {
    static V: OnceLock<JSONSchema> = OnceLock::new();
    V.get_or_init(|| {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../starter-ui-ir/schema/starter-ui-ir.schema.json",
        );
        let bytes = std::fs::read(path).expect("read IR schema artifact");
        let schema: Value = serde_json::from_slice(&bytes).expect("parse IR schema");
        JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("compile IR schema")
    })
}

/// Wrap a single component as the root of a `ComponentTree` so the
/// schema's `ComponentTree` definition can validate it.
fn wrap_in_tree(c: Component) -> ComponentTree {
    let root = match &c {
        Component::Page { .. } => c,
        _ => page("root", "test").child(c).build(),
    };
    ComponentTree {
        ir_version: IR_VERSION,
        root,
        vars: Default::default(),
    }
}

/// Validation modes — schema check is skipped where Phase 1 schema
/// artifact diverges from the wire shape (see module docs).
#[derive(Copy, Clone)]
enum Validate {
    /// Run both serde round-trip and JSON Schema validation.
    SchemaAndSerde,
    /// Skip JSON Schema; rely on serde round-trip only.
    /// (Variant touches `Bindings` or `ChartKind`.)
    SerdeOnly,
}

#[track_caller]
fn assert_valid_ir(label: &str, c: Component, mode: Validate) {
    let tree = wrap_in_tree(c);
    assert_tree_valid(label, tree, mode);
}

#[track_caller]
fn assert_tree_valid(label: &str, tree: ComponentTree, mode: Validate) {
    let value = serde_json::to_value(&tree).expect("ComponentTree serialises");
    // Serde round-trip — the runtime validator-of-record.
    let _back: ComponentTree = serde_json::from_value(value.clone())
        .unwrap_or_else(|e| panic!("{label}: serde round-trip failed: {e}\n{value:#}"));

    if matches!(mode, Validate::SchemaAndSerde) {
        let v = validator();
        let messages: Vec<String> = match v.validate(&value) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| format!("{e}")).collect(),
        };
        if !messages.is_empty() {
            panic!(
                "{label}: IR JSON Schema rejected the builder output: {}\n{value:#}",
                messages.join("; ")
            );
        }
    }
}

// =====================================================================
// Per-builder smoke — every public surface is exercised.
// =====================================================================

#[test]
fn page_builder_valid_ir() {
    assert_valid_ir("page", page("p", "Title").build(), Validate::SchemaAndSerde);
}

#[test]
fn row_builder_valid_ir() {
    assert_valid_ir(
        "row",
        row("r").child(text("hi").build()).build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn col_builder_valid_ir() {
    assert_valid_ir(
        "col",
        col("c").child(text("hi").build()).build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn grid_builder_valid_ir() {
    assert_valid_ir(
        "grid",
        grid("g").columns("1fr 1fr").build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn tabs_builder_valid_ir() {
    assert_valid_ir(
        "tabs",
        tabs("t")
            .tab("a", "Alpha", [text("A").build()])
            .tab("b", "Beta", [text("B").build()])
            .build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn card_builder_valid_ir() {
    assert_valid_ir(
        "card",
        card("c", "Title")
            .subtitle("desc")
            .intent("info")
            .children([text("body").build()])
            .build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn heading_builder_valid_ir() {
    assert_valid_ir(
        "heading",
        heading("Hello").level(2).build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn text_builder_valid_ir() {
    assert_valid_ir(
        "text",
        text("Hello").intent("info").build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn badge_builder_valid_ir() {
    assert_valid_ir(
        "badge",
        badge("new").intent("success").build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn kpi_builder_valid_ir() {
    // KPI carries a ChartSource but no ChartKind — schema match
    // depends on whether ChartKind appears anywhere in the rendered
    // subtree; KPI itself does not embed ChartKind.
    assert_valid_ir(
        "kpi",
        kpi("k", "Active", series("n1", "value")),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn kpi_grid_builder_valid_ir() {
    assert_valid_ir(
        "kpi_grid",
        kpi_grid(
            "kpis",
            "1fr 1fr",
            [
                kpi("a", "A", series("n1", "v")),
                kpi("b", "B", series("n2", "v")),
            ],
        ),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn table_builder_valid_ir() {
    assert_valid_ir(
        "table",
        table("alarms", rsql().kind("alarm.active"))
            .live()
            .column("Time", "slots.ts.value")
            .column("Severity", "slots.severity.value")
            .build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn line_chart_builder_valid_ir() {
    // Touches ChartKind — schemars emits PascalCase, wire emits
    // snake_case; serde round-trip is the practical validator.
    assert_valid_ir(
        "line_chart",
        line_chart("temp").source(series("n1", "value")).build(),
        Validate::SerdeOnly,
    );
}

#[test]
fn bar_chart_builder_valid_ir() {
    assert_valid_ir(
        "bar_chart",
        bar_chart("by_gate")
            .source(
                rows(rsql().kind("com.acme.task"))
                    .group_by("settings.gate")
                    .count(),
            )
            .build(),
        Validate::SerdeOnly,
    );
}

#[test]
fn gauge_builder_valid_ir() {
    assert_valid_ir(
        "gauge",
        gauge("g").source(series("n1", "value")).build(),
        Validate::SerdeOnly,
    );
}

#[test]
fn sparkline_builder_valid_ir() {
    assert_valid_ir(
        "sparkline",
        sparkline("s", "node.n1.slot.value"),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn form_builder_valid_ir() {
    assert_valid_ir(
        "form",
        form(
            "f",
            "com.acme.task.create",
            json!({ "type": "object", "properties": { "name": { "type": "string" } } }),
        )
        .submit_label("Create")
        .build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn select_builder_valid_ir() {
    assert_valid_ir(
        "select",
        select("severity", "severity")
            .option("Low", "low")
            .option("High", "high")
            .build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn toggle_builder_valid_ir() {
    // Touches Bindings — custom Serialize unwraps single-spec to a
    // bare string; schemars emits an array. Serde round-trip stays
    // strict; the schema-vs-wire mismatch is Phase 1's to resolve.
    assert_valid_ir(
        "toggle",
        toggle("t", "$target.enabled").label("Enabled").build(),
        Validate::SerdeOnly,
    );
}

#[test]
fn slider_builder_valid_ir() {
    assert_valid_ir(
        "slider",
        slider("s", "$target.brightness")
            .range(0.0, 100.0)
            .step(5.0)
            .build(),
        Validate::SerdeOnly,
    );
}

#[test]
fn date_range_builder_valid_ir() {
    assert_valid_ir(
        "date_range",
        date_range("range", "window")
            .preset("Last hour", 3_600_000)
            .preset_all_time("All")
            .build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn ref_picker_builder_valid_ir() {
    assert_valid_ir(
        "ref_picker",
        ref_picker("p", rsql().kind("com.acme.task")).build(),
        Validate::SchemaAndSerde,
    );
}

#[test]
fn dashboard_builder_valid_ir() {
    let tree = dashboard("d", "Title", [heading("Hello").build()]);
    assert_tree_valid("dashboard", tree, Validate::SchemaAndSerde);
}

#[test]
fn action_form_emits_two_components() {
    let pieces = action_form(ActionForm {
        handler: "com.acme.task.create",
        heading: "Create Task",
        button_label: "Create",
        intent: "primary",
        schema: json!({ "type": "object" }),
        id_prefix: None,
    });
    assert_eq!(pieces.len(), 2);

    let heading_c: Component = serde_json::from_value(pieces[0].clone()).unwrap();
    let form_c: Component = serde_json::from_value(pieces[1].clone()).unwrap();
    let tree = dashboard("d", "Title", [heading_c, form_c]);
    assert_tree_valid("action_form", tree, Validate::SchemaAndSerde);
}

#[test]
fn bindings_helpers_emit_well_formed_expressions() {
    // The bindings module returns Strings; the smoke is that the
    // wrapped expressions parse downstream. We don't link
    // starter-ui-bindings into the contract surface (per the M4
    // dependency split), so the assertion is structural.
    assert_eq!(target("name"), "{{$target.name}}");
    assert_eq!(stack(0, "id"), "{{$stack[0].id}}");
    assert_eq!(user("email"), "{{$user.email}}");
    assert_eq!(self_("layout"), "{{$self.layout}}");
    assert_eq!(page_state("severity"), "{{$page.severity}}");
    assert_eq!(vars("api_base"), "{{$vars.api_base}}");
}
