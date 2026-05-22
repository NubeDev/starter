//! Input primitives — `select()`, `toggle()`, `slider()`,
//! `date_range()`, `ref_picker()`.

use serde_json::Value as JsonValue;
use starter_ui_ir::{BindingSpec, Bindings, Component, Concurrency, DateRangePreset, SelectOption};

use crate::rsql::RsqlBuilder;

// =====================================================================
// Select
// =====================================================================

/// Construct a [`Component::Select`]. Selecting an option writes its
/// `value` into `$page[<page_state_key>]`.
pub fn select(id: impl Into<String>, page_state_key: impl Into<String>) -> SelectBuilder {
    SelectBuilder {
        id: id.into(),
        page_state_key: page_state_key.into(),
        options: Vec::new(),
        placeholder: None,
        default: None,
    }
}

/// Builder for [`Component::Select`].
#[derive(Debug, Clone)]
pub struct SelectBuilder {
    id: String,
    page_state_key: String,
    options: Vec<SelectOption>,
    placeholder: Option<String>,
    default: Option<JsonValue>,
}

impl SelectBuilder {
    /// Append an option whose value is a string.
    pub fn option(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.push(SelectOption {
            label: label.into(),
            value: JsonValue::String(value.into()),
        });
        self
    }

    /// Append an option with an arbitrary JSON value.
    pub fn option_value(mut self, label: impl Into<String>, value: JsonValue) -> Self {
        self.options.push(SelectOption {
            label: label.into(),
            value,
        });
        self
    }

    /// Set the placeholder shown when no option is selected.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = Some(p.into());
        self
    }

    /// Set the initial value applied on mount.
    pub fn default_value(mut self, v: JsonValue) -> Self {
        self.default = Some(v);
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Select {
            id: Some(self.id),
            page_state_key: self.page_state_key,
            options: self.options,
            placeholder: self.placeholder,
            default: self.default,
        }
    }
}

// =====================================================================
// Toggle
// =====================================================================

/// Construct a two-way bound [`Component::Toggle`]. `bind` is a
/// binding expression (e.g. `$target.enabled`) — use the
/// [`crate::bindings`] helpers to compose it without quoting
/// mistakes.
pub fn toggle(id: impl Into<String>, bind: impl Into<String>) -> ToggleBuilder {
    ToggleBuilder {
        id: id.into(),
        bind: BindingSpec::Short(bind.into()),
        label: None,
        value: None,
    }
}

/// Builder for [`Component::Toggle`].
#[derive(Debug, Clone)]
pub struct ToggleBuilder {
    id: String,
    bind: BindingSpec,
    label: Option<String>,
    value: Option<bool>,
}

impl ToggleBuilder {
    /// Set the visible label.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    /// Switch the binding to OCC (optimistic concurrency) — server
    /// rejects stale writes with 409.
    pub fn occ(mut self) -> Self {
        self.bind = BindingSpec::Full {
            slot: self.bind.slot_expr().to_string(),
            concurrency: Concurrency::Occ,
            debounce_ms: None,
        };
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Toggle {
            id: self.id,
            bind: Bindings(vec![self.bind]),
            label: self.label,
            value: self.value,
            style: None,
        }
    }
}

// =====================================================================
// Slider
// =====================================================================

/// Construct a two-way bound [`Component::Slider`].
pub fn slider(id: impl Into<String>, bind: impl Into<String>) -> SliderBuilder {
    SliderBuilder {
        id: id.into(),
        bind: BindingSpec::Short(bind.into()),
        label: None,
        value: None,
        min: None,
        max: None,
        step: None,
    }
}

/// Builder for [`Component::Slider`].
#[derive(Debug, Clone)]
pub struct SliderBuilder {
    id: String,
    bind: BindingSpec,
    label: Option<String>,
    value: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
}

impl SliderBuilder {
    /// Set the visible label.
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    /// Set the rendering range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    /// Set the step size.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Override the trailing-debounce window (default 150 ms).
    pub fn debounce_ms(mut self, ms: u32) -> Self {
        self.bind = BindingSpec::Full {
            slot: self.bind.slot_expr().to_string(),
            concurrency: self.bind.concurrency(),
            debounce_ms: Some(ms),
        };
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::Slider {
            id: self.id,
            bind: Bindings(vec![self.bind]),
            label: self.label,
            value: self.value,
            min: self.min,
            max: self.max,
            step: self.step,
            style: None,
        }
    }
}

// =====================================================================
// DateRange
// =====================================================================

/// Construct a [`Component::DateRange`] with preset buttons. The
/// component writes `{from, to}` (Unix ms) into
/// `$page[<page_state_key>]`.
pub fn date_range(id: impl Into<String>, page_state_key: impl Into<String>) -> DateRangeBuilder {
    DateRangeBuilder {
        id: id.into(),
        page_state_key: page_state_key.into(),
        presets: Vec::new(),
    }
}

/// Builder for [`Component::DateRange`].
#[derive(Debug, Clone)]
pub struct DateRangeBuilder {
    id: String,
    page_state_key: String,
    presets: Vec<DateRangePreset>,
}

impl DateRangeBuilder {
    /// Append a preset with a fixed window in milliseconds.
    pub fn preset(mut self, label: impl Into<String>, duration_ms: i64) -> Self {
        self.presets.push(DateRangePreset {
            label: label.into(),
            duration_ms: Some(duration_ms),
        });
        self
    }

    /// Append an "all time" preset (writes `null` for both endpoints).
    pub fn preset_all_time(mut self, label: impl Into<String>) -> Self {
        self.presets.push(DateRangePreset {
            label: label.into(),
            duration_ms: None,
        });
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::DateRange {
            id: Some(self.id),
            page_state_key: self.page_state_key,
            presets: self.presets,
        }
    }
}

// =====================================================================
// RefPicker
// =====================================================================

/// Construct a [`Component::RefPicker`] restricted by an RSQL filter.
pub fn ref_picker(id: impl Into<String>, query: RsqlBuilder) -> RefPickerBuilder {
    RefPickerBuilder {
        id: id.into(),
        query: Some(query.build()),
        value: None,
        placeholder: None,
    }
}

/// Builder for [`Component::RefPicker`].
#[derive(Debug, Clone)]
pub struct RefPickerBuilder {
    id: String,
    query: Option<String>,
    value: Option<String>,
    placeholder: Option<String>,
}

impl RefPickerBuilder {
    /// Pre-select a node.
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = Some(v.into());
        self
    }

    /// Set the placeholder text.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = Some(p.into());
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        Component::RefPicker {
            id: Some(self.id),
            query: self.query,
            value: self.value,
            placeholder: self.placeholder,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rsql::rsql;

    #[test]
    fn select_with_options() {
        let s = select("severity", "severity")
            .option("Low", "low")
            .option("High", "high")
            .placeholder("Pick one")
            .build();
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "select");
        assert_eq!(v["options"][0]["value"], "low");
    }

    #[test]
    fn toggle_with_occ() {
        let t = toggle("enable", "$target.enabled")
            .occ()
            .label("On")
            .build();
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["type"], "toggle");
        assert_eq!(v["bind"]["slot"], "$target.enabled");
        assert_eq!(v["bind"]["concurrency"], "occ");
    }

    #[test]
    fn slider_with_range_and_debounce() {
        let s = slider("brightness", "$target.brightness")
            .range(0.0, 100.0)
            .step(5.0)
            .debounce_ms(300)
            .build();
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["type"], "slider");
        assert_eq!(v["min"], 0.0);
        assert_eq!(v["max"], 100.0);
        assert_eq!(v["bind"]["debounce_ms"], 300);
    }

    #[test]
    fn date_range_with_presets() {
        let d = date_range("range", "window")
            .preset("Last hour", 3_600_000)
            .preset_all_time("All")
            .build();
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["type"], "date_range");
        assert_eq!(v["presets"][0]["duration_ms"], 3_600_000);
        assert!(v["presets"][1].get("duration_ms").is_none());
    }

    #[test]
    fn ref_picker_uses_rsql() {
        let r = ref_picker("p", rsql().kind("com.acme.task")).build();
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["query"], "kind==com.acme.task");
    }
}
