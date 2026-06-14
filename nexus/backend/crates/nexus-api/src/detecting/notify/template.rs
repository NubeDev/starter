//! Render a detection notification message from a template, safely.
//!
//! A detection may carry a message template with `{{detection_name}}`,
//! `{{transition}}`, `{{target}}`, `{{value}}` placeholders. Rendering is pure
//! single-pass substitution of a fixed, closed set of tokens — never an eval of
//! caller-supplied expressions — so a template cannot inject into the channel
//! payload beyond the values the finding already carries. Unknown `{{tokens}}`
//! are left verbatim (a typo is visible, not silently dropped). No I/O.

use serde_json::Value;

/// The values a template can reference. A closed set: the renderer only ever
/// substitutes these, so there is no expression surface to exploit.
pub struct TemplateContext<'a> {
    pub detection_name: &'a str,
    pub transition: &'a str,
    pub target: &'a Value,
    pub value: Option<f64>,
}

/// The default template used when a detection sets none.
pub const DEFAULT_TEMPLATE: &str =
    "Detection {{detection_name}} {{transition}} for {{target}} (value {{value}})";

/// Render `template` against `ctx` in a single left-to-right pass. Each known
/// `{{token}}` is replaced; an absent numeric value renders as `n/a`. Unknown
/// tokens are preserved verbatim. Single-pass is the security property: a value
/// that itself contains `{{token}}` text is never re-scanned.
pub fn render(template: &str, ctx: &TemplateContext<'_>) -> String {
    let value = ctx
        .value
        .map(format_number)
        .unwrap_or_else(|| "n/a".to_string());
    let target = compact_target(ctx.target);
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        let Some(close) = after_open.find("}}") else {
            out.push_str(&rest[open..]);
            return out;
        };
        let token = &after_open[..close];
        match token {
            "detection_name" => out.push_str(ctx.detection_name),
            "transition" => out.push_str(ctx.transition),
            "target" => out.push_str(&target),
            "value" => out.push_str(&value),
            _ => {
                out.push_str("{{");
                out.push_str(token);
                out.push_str("}}");
            }
        }
        rest = &after_open[close + 2..];
    }
    out.push_str(rest);
    out
}

/// Render the target object as a compact `k=v, k=v` string for the message — far
/// more readable in a Slack line than raw JSON. Non-object targets fall back to
/// their JSON form.
fn compact_target(target: &Value) -> String {
    match target.as_object() {
        Some(obj) => obj
            .iter()
            .map(|(k, v)| format!("{k}={}", scalar(v)))
            .collect::<Vec<_>>()
            .join(", "),
        None => target.to_string(),
    }
}

/// A JSON scalar without surrounding quotes for strings.
fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Format a float without a trailing `.0` for whole numbers.
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> TemplateContext<'static> {
        TemplateContext {
            detection_name: "High usage",
            transition: "opened",
            target: TARGET.get_or_init(|| json!({ "site": "s1", "meter": "m2" })),
            value: Some(95.5),
        }
    }
    use std::sync::OnceLock;
    static TARGET: OnceLock<Value> = OnceLock::new();

    #[test]
    fn default_template_renders_all_tokens() {
        let out = render(DEFAULT_TEMPLATE, &ctx());
        // Target key order follows the JSON object's map iteration, so assert on
        // the pieces rather than a fixed key order.
        assert!(out.starts_with("Detection High usage opened for "));
        assert!(out.contains("site=s1"));
        assert!(out.contains("meter=m2"));
        assert!(out.ends_with("(value 95.5)"));
    }

    #[test]
    fn missing_value_renders_n_a() {
        let mut c = ctx();
        c.value = None;
        assert!(render("{{value}}", &c).contains("n/a"));
    }

    #[test]
    fn unknown_tokens_are_preserved() {
        assert_eq!(render("{{bogus}}", &ctx()), "{{bogus}}");
    }
}
