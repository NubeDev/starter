//! The YAML device template — parse, validate, expand.
//!
//! One template per device model (BARCODE.md §3). The template
//! declares the points a device exposes, their default widgets, and
//! which features (trend / alarm) can be toggled. Parsing is
//! **strict**: an unknown key is a load-time error so a typo can
//! never silently drop a point or an alarm.
//!
//! `repeat: N` on a point spec expands `key` into `key1..keyN` so a
//! 22-channel controller isn't 22 copy-pasted blocks.
//!
//! Parsing goes YAML → `serde_json::Value` → manual walk so the
//! extension keeps a single starter-workspace dependency
//! (`starter-ext-sdk`) and never pulls in `serde` derive
//! (BARCODE.md §11 removability / SCOPE single-dep rule).

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::Error;

mod parse;

pub use parse::AlarmRule;
use parse::{parse_points, parse_widget_group};

/// A parsed device template.
#[derive(Debug, Clone)]
pub struct DeviceTemplate {
    /// Lookup key, matched against the barcode `model`.
    pub template: String,
    pub version: i64,
    pub display_name: Option<String>,
    pub network: Option<String>,
    pub category: Option<String>,
    pub icon: Option<String>,
    /// Point specs as authored (before `repeat` expansion).
    points: Vec<PointSpec>,
    widget_group: Option<WidgetGroup>,
}

/// One point spec from the template (possibly expanded via `repeat`).
#[derive(Debug, Clone)]
pub struct PointSpec {
    pub key: String,
    pub name: Option<String>,
    pub unit: Option<String>,
    pub kind: Option<String>,
    pub widget: Option<String>,
    pub writable: bool,
    /// Expand this spec into `key1..keyN` when present and > 1.
    pub repeat: Option<u32>,
    pub trend_default: bool,
    pub trend_interval: Option<String>,
    pub alarm_default: bool,
    pub alarm_rules: Vec<AlarmRule>,
}

/// How the device renders as a unit on a page.
#[derive(Debug, Clone)]
pub struct WidgetGroup {
    /// `card` | `row` | `bento-tile`.
    pub layout: Option<String>,
    pub title: Option<String>,
    /// Point key shown big.
    pub primary: Option<String>,
    pub secondary: Vec<String>,
}

impl DeviceTemplate {
    /// Strict-parse a template from its YAML source.
    pub fn parse(yaml: &str) -> starter_ext_sdk::Result<Self> {
        let root: Value = starter_ext_sdk::serde_yaml::from_str(yaml)
            .map_err(|e| Error::Validation(format!("bc template parse error: {e}")))?;
        let obj = root.as_object().ok_or_else(|| {
            Error::Validation("bc template: top level must be a YAML mapping".into())
        })?;
        reject_unknown(
            obj,
            &[
                "template",
                "version",
                "display_name",
                "network",
                "category",
                "icon",
                "points",
                "widget_group",
            ],
            "template",
        )?;

        let template = req_str(obj, "template")?;
        if template.trim().is_empty() {
            return Err(Error::Validation(
                "bc template: `template` must be a non-empty string".into(),
            ));
        }
        let points = parse_points(obj.get("points"), &template)?;
        if points.is_empty() {
            return Err(Error::Validation(format!(
                "bc template `{template}`: declares no points"
            )));
        }

        Ok(Self {
            template,
            version: obj.get("version").and_then(Value::as_i64).unwrap_or(1),
            display_name: opt_str(obj, "display_name"),
            network: opt_str(obj, "network"),
            category: opt_str(obj, "category"),
            icon: opt_str(obj, "icon"),
            points,
            widget_group: parse_widget_group(obj.get("widget_group"))?,
        })
    }

    /// Expand `repeat:` specs into concrete points. A spec with
    /// `repeat: 12` and `key: di` yields `di1..di12`, each with its
    /// channel number appended to the name. A spec without `repeat`
    /// (or `repeat: 1`) yields one point with the verbatim key.
    pub fn expanded_points(&self) -> Vec<ExpandedPoint> {
        let mut out = Vec::new();
        for spec in &self.points {
            match spec.repeat {
                Some(n) if n > 1 => {
                    for i in 1..=n {
                        out.push(ExpandedPoint::from_spec(spec, Some(i)));
                    }
                }
                _ => out.push(ExpandedPoint::from_spec(spec, None)),
            }
        }
        out
    }

    /// JSON cache of the expanded points, stored in
    /// `bc_templates.points_json` for fast UI reads.
    pub fn points_json(&self) -> Value {
        Value::Array(self.expanded_points().iter().map(ExpandedPoint::to_json).collect())
    }

    /// JSON cache of the widget group for `bc_templates.widget_group_json`.
    pub fn widget_group_json(&self) -> Value {
        match &self.widget_group {
            Some(g) => json!({
                "layout": g.layout,
                "title": g.title,
                "primary": g.primary,
                "secondary": g.secondary,
            }),
            None => Value::Null,
        }
    }

    /// The widget group, if the template declares one.
    pub fn widget_group(&self) -> Option<&WidgetGroup> {
        self.widget_group.as_ref()
    }

    /// Template summary for the `bc_decode` response — enough for the
    /// UI to render an identify card without a second round-trip.
    pub fn decode_summary(&self) -> Value {
        let points: Vec<Value> = self
            .expanded_points()
            .iter()
            .map(|p| json!({ "key": p.key, "name": p.name, "widget": p.widget }))
            .collect();
        json!({
            "template": self.template,
            "display_name": self.display_name,
            "icon": self.icon,
            "category": self.category,
            "points": points,
            "widget_group": self.widget_group_json(),
        })
    }
}

/// A point after `repeat` expansion, ready to become a `bc_points`
/// row and (where it carries rules) `bc_alarms` rows.
#[derive(Debug, Clone)]
pub struct ExpandedPoint {
    pub key: String,
    pub name: String,
    pub unit: Option<String>,
    pub kind: Option<String>,
    pub widget: Option<String>,
    pub writable: bool,
    pub trend_default: bool,
    pub trend_interval: Option<String>,
    pub alarm_default: bool,
    pub alarm_rules: Vec<AlarmRule>,
}

impl ExpandedPoint {
    /// Build a concrete point from a spec, optionally suffixing the
    /// `repeat` channel index onto the key and name.
    fn from_spec(spec: &PointSpec, channel: Option<u32>) -> Self {
        let (key, name) = match channel {
            Some(i) => (
                format!("{}{i}", spec.key),
                match &spec.name {
                    Some(n) => format!("{n} {i}"),
                    None => format!("{} {i}", spec.key),
                },
            ),
            None => (
                spec.key.clone(),
                spec.name.clone().unwrap_or_else(|| spec.key.clone()),
            ),
        };
        Self {
            key,
            name,
            unit: spec.unit.clone(),
            kind: spec.kind.clone(),
            widget: spec.widget.clone(),
            writable: spec.writable,
            trend_default: spec.trend_default,
            trend_interval: spec.trend_interval.clone(),
            alarm_default: spec.alarm_default,
            alarm_rules: spec.alarm_rules.clone(),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "key": self.key,
            "name": self.name,
            "unit": self.unit,
            "kind": self.kind,
            "widget": self.widget,
            "writable": self.writable,
            "trend_default": self.trend_default,
            "trend_interval": self.trend_interval,
            "alarm_default": self.alarm_default,
        })
    }
}

// -- shared mapping accessors used by this module and `parse` --------

/// Reject any key in `obj` not present in `allowed` — the strict-parse
/// guarantee that a typo never silently drops a point or an alarm.
pub(super) fn reject_unknown(
    obj: &Map<String, Value>,
    allowed: &[&str],
    ctx: &str,
) -> starter_ext_sdk::Result<()> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(Error::Validation(format!(
                "bc template: unknown key `{key}` in {ctx} (allowed: {})",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

/// Required string field.
pub(super) fn req_str(obj: &Map<String, Value>, key: &str) -> starter_ext_sdk::Result<String> {
    obj.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::Validation(format!("bc template: `{key}` (string) is required")))
}

/// Optional string field.
pub(super) fn opt_str(obj: &Map<String, Value>, key: &str) -> Option<String> {
    obj.get(key).and_then(Value::as_str).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DROPLET: &str = include_str!("../../../templates/droplet.yaml");
    const IO_22: &str = include_str!("../../../templates/io_22.yaml");
    const MICRO_EDGE: &str = include_str!("../../../templates/micro_edge.yaml");

    #[test]
    fn parses_droplet_seed() {
        let tpl = DeviceTemplate::parse(DROPLET).unwrap();
        assert_eq!(tpl.template, "droplet");
        let points = tpl.expanded_points();
        assert_eq!(points.len(), 4);
        let temp = &points[0];
        assert_eq!(temp.key, "temp");
        assert!(temp.trend_default);
        assert_eq!(temp.trend_interval.as_deref(), Some("5m"));
        let battery = points.iter().find(|p| p.key == "battery").unwrap();
        assert!(battery.alarm_default);
        assert_eq!(battery.alarm_rules.len(), 2);
    }

    #[test]
    fn parses_micro_edge_seed() {
        let tpl = DeviceTemplate::parse(MICRO_EDGE).unwrap();
        assert_eq!(tpl.expanded_points().len(), 3);
    }

    #[test]
    fn expands_io22_repeat() {
        let tpl = DeviceTemplate::parse(IO_22).unwrap();
        let points = tpl.expanded_points();
        assert_eq!(points.len(), 22);
        assert!(points.iter().any(|p| p.key == "di1"));
        assert!(points.iter().any(|p| p.key == "di12"));
        assert!(points.iter().any(|p| p.key == "do10"));
        let do1 = points.iter().find(|p| p.key == "do1").unwrap();
        assert!(do1.writable);
    }

    #[test]
    fn rejects_unknown_key() {
        let yaml = "template: x\npoints:\n  - key: a\n    bogus_field: 1\n";
        assert!(DeviceTemplate::parse(yaml).is_err());
    }

    #[test]
    fn rejects_no_points() {
        let yaml = "template: x\npoints: []\n";
        assert!(DeviceTemplate::parse(yaml).is_err());
    }
}
