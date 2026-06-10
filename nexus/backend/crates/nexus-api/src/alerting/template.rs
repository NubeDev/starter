//! Render a notification message from a template, safely.
//!
//! A channel or rule may carry a message template with `{{rule_name}}`,
//! `{{value}}`, `{{threshold}}`, `{{state}}`, `{{op}}` placeholders. Rendering is
//! pure single-pass substitution of a fixed, closed set of tokens — never an eval
//! of caller-supplied expressions — so a template cannot inject into the channel
//! payload beyond the values the alert already carries. Unknown `{{tokens}}` are
//! left verbatim (a typo is visible in the output, not silently dropped). No I/O.

/// The values a template can reference. A closed set: the renderer only ever
/// substitutes these, so there is no expression surface to exploit.
pub struct TemplateContext<'a> {
    pub rule_name: &'a str,
    pub state: &'a str,
    pub op: &'a str,
    pub threshold: f64,
    pub value: Option<f64>,
}

/// The default template used when a rule/channel sets none.
pub const DEFAULT_TEMPLATE: &str =
    "Alert {{rule_name}} is {{state}} (value {{value}} {{op}} threshold {{threshold}})";

/// Render `template` against `ctx` in a single left-to-right pass. Each known
/// `{{token}}` is replaced by its value; an absent numeric value renders as
/// `n/a`. Unknown tokens are preserved verbatim. Single-pass is the security
/// property: a value that itself contains `{{token}}` text is never re-scanned,
/// so a template cannot be made to expand caller-controlled data into a token.
pub fn render(template: &str, ctx: &TemplateContext<'_>) -> String {
    let value = ctx
        .value
        .map(format_number)
        .unwrap_or_else(|| "n/a".to_string());
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        let Some(close) = after_open.find("}}") else {
            // No closing braces — the remainder is literal.
            out.push_str(&rest[open..]);
            return out;
        };
        let token = &after_open[..close];
        match token {
            "rule_name" => out.push_str(ctx.rule_name),
            "state" => out.push_str(ctx.state),
            "op" => out.push_str(ctx.op),
            "threshold" => out.push_str(&format_number(ctx.threshold)),
            "value" => out.push_str(&value),
            // Unknown token: preserve it verbatim so a typo is visible.
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

/// Format a float without a trailing `.0` for whole numbers — the form a human
/// reading an alert expects ("90", not "90.0").
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

    fn ctx() -> TemplateContext<'static> {
        TemplateContext {
            rule_name: "CPU high",
            state: "firing",
            op: "gt",
            threshold: 90.0,
            value: Some(95.5),
        }
    }

    #[test]
    fn default_template_renders_all_tokens() {
        let out = render(DEFAULT_TEMPLATE, &ctx());
        assert_eq!(out, "Alert CPU high is firing (value 95.5 gt threshold 90)");
    }

    #[test]
    fn missing_value_renders_n_a() {
        let mut c = ctx();
        c.value = None;
        assert!(render("{{value}}", &c).contains("n/a"));
    }

    #[test]
    fn unknown_tokens_are_preserved_not_dropped() {
        assert_eq!(render("{{bogus}}", &ctx()), "{{bogus}}");
    }

    #[test]
    fn no_expression_injection_only_fixed_tokens_substitute() {
        // A template that tries to nest a token inside a value is not re-expanded:
        // substitution is single-pass over the fixed set.
        let mut c = ctx();
        c.rule_name = "{{threshold}}";
        // The injected token stays literal — it is part of a *value*, not the
        // template text the renderer scans.
        assert_eq!(render("{{rule_name}}", &c), "{{threshold}}");
    }
}
