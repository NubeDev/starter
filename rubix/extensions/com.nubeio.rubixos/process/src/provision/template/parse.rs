//! Manual strict-parse of the `points[]` and `widget_group` sections
//! of a device template (BARCODE.md §3). Split out of `template.rs`
//! so the model types stay readable and each file keeps one job.

use starter_ext_sdk::serde_json::{Map, Value};
use starter_ext_sdk::Error;

use super::{opt_str, reject_unknown, req_str, PointSpec, WidgetGroup};

/// One alarm rule: a predicate, severity and operator-facing message.
#[derive(Debug, Clone)]
pub struct AlarmRule {
    /// Predicate against the point value, e.g. `"> 35"` or `"< 5"`.
    pub when: String,
    pub severity: Option<String>,
    pub message: Option<String>,
}

/// Parse the `points:` list into specs, strict on unknown keys.
pub fn parse_points(value: Option<&Value>, tpl: &str) -> starter_ext_sdk::Result<Vec<PointSpec>> {
    let arr = value
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Validation(format!("bc template `{tpl}`: `points` must be a list")))?;
    arr.iter().map(parse_point).collect()
}

fn parse_point(value: &Value) -> starter_ext_sdk::Result<PointSpec> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::Validation("bc template: each point must be a mapping".into()))?;
    reject_unknown(
        obj,
        &[
            "key", "name", "unit", "kind", "widget", "writable", "repeat", "trend", "alarm",
        ],
        "point",
    )?;

    let (trend_default, trend_interval) = parse_trend(obj.get("trend"))?;
    let (alarm_default, alarm_rules) = parse_alarm(obj.get("alarm"))?;

    Ok(PointSpec {
        key: req_str(obj, "key")?,
        name: opt_str(obj, "name"),
        unit: opt_str(obj, "unit"),
        kind: opt_str(obj, "kind"),
        widget: opt_str(obj, "widget"),
        writable: obj.get("writable").and_then(Value::as_bool).unwrap_or(false),
        repeat: obj
            .get("repeat")
            .and_then(Value::as_u64)
            .map(|n| n as u32),
        trend_default,
        trend_interval,
        alarm_default,
        alarm_rules,
    })
}

/// Parse the per-point `trend: { default, interval }` block.
fn parse_trend(value: Option<&Value>) -> starter_ext_sdk::Result<(bool, Option<String>)> {
    let Some(value) = value else {
        return Ok((false, None));
    };
    let obj = value
        .as_object()
        .ok_or_else(|| Error::Validation("bc template: `trend` must be a mapping".into()))?;
    reject_unknown(obj, &["default", "interval"], "trend")?;
    Ok((
        obj.get("default").and_then(Value::as_bool).unwrap_or(false),
        opt_str(obj, "interval"),
    ))
}

/// Parse the per-point `alarm: { default, rules[] }` block.
fn parse_alarm(value: Option<&Value>) -> starter_ext_sdk::Result<(bool, Vec<AlarmRule>)> {
    let Some(value) = value else {
        return Ok((false, Vec::new()));
    };
    let obj = value
        .as_object()
        .ok_or_else(|| Error::Validation("bc template: `alarm` must be a mapping".into()))?;
    reject_unknown(obj, &["default", "rules"], "alarm")?;
    let default = obj.get("default").and_then(Value::as_bool).unwrap_or(false);
    let rules = match obj.get("rules") {
        Some(Value::Array(arr)) => arr.iter().map(parse_rule).collect::<Result<_, _>>()?,
        Some(_) => {
            return Err(Error::Validation(
                "bc template: `alarm.rules` must be a list".into(),
            ))
        }
        None => Vec::new(),
    };
    Ok((default, rules))
}

fn parse_rule(value: &Value) -> starter_ext_sdk::Result<AlarmRule> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::Validation("bc template: each alarm rule must be a mapping".into()))?;
    reject_unknown(obj, &["when", "severity", "message"], "alarm rule")?;
    Ok(AlarmRule {
        when: req_str(obj, "when")?,
        severity: opt_str(obj, "severity"),
        message: opt_str(obj, "message"),
    })
}

/// Parse the optional `widget_group:` block.
pub fn parse_widget_group(
    value: Option<&Value>,
) -> starter_ext_sdk::Result<Option<WidgetGroup>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let obj = value
        .as_object()
        .ok_or_else(|| Error::Validation("bc template: `widget_group` must be a mapping".into()))?;
    reject_unknown(
        obj,
        &["layout", "title", "primary", "secondary"],
        "widget_group",
    )?;
    Ok(Some(WidgetGroup {
        layout: opt_str(obj, "layout"),
        title: opt_str(obj, "title"),
        primary: opt_str(obj, "primary"),
        secondary: parse_str_list(obj, "secondary"),
    }))
}

/// Read a `[a, b, c]` string list, ignoring non-string entries.
fn parse_str_list(obj: &Map<String, Value>, key: &str) -> Vec<String> {
    obj.get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
