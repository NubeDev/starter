//! Named-placeholder interpolator for translation templates.
//!
//! Two entry points: [`interpolate_typed`] takes the canonical
//! [`DiagnosticParam`] envelope and is used by transports that hold
//! typed params (the [`crate::MessageBundle::render`] path); the
//! [`crate::diagnostics`] HTTP layer keeps its own JSON-shaped helper
//! for the response-body rewrite case where params arrive as
//! `serde_json::Value`.
//!
//! The substitution grammar is `{name}` only. Unknown placeholders
//! are left literal so a template referencing a missing param
//! surfaces as text rather than silently dropping. Plural / select
//! / number formatting is the client's job — the same templates
//! hand off to `react-intl` client-side.

use std::collections::BTreeMap;

use starter_spi::i18n::DiagnosticParam;

/// Substitute `{name}` placeholders in `template` against typed
/// `params`. The output is allocated once at the template's length
/// plus a small headroom; allocations grow lazily as values are
/// written in.
#[must_use]
pub fn interpolate_typed(template: &str, params: &BTreeMap<String, DiagnosticParam>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '{' {
            out.push(c);
            continue;
        }
        // Collect until the matching '}'. Bail if we never find one
        // — emit the original characters so a malformed template
        // survives.
        let mut name = String::new();
        let mut closed = false;
        for nc in chars.by_ref() {
            if nc == '}' {
                closed = true;
                break;
            }
            name.push(nc);
        }
        if !closed {
            out.push('{');
            out.push_str(&name);
            continue;
        }
        match params.get(name.as_str()) {
            Some(p) => write_param(p, &mut out),
            None => {
                out.push('{');
                out.push_str(&name);
                out.push('}');
            }
        }
    }
    out
}

fn write_param(p: &DiagnosticParam, out: &mut String) {
    use std::fmt::Write;
    match p {
        DiagnosticParam::String(s) => out.push_str(s),
        DiagnosticParam::I64(n) => {
            let _ = write!(out, "{n}");
        }
        DiagnosticParam::F64(n) => {
            let _ = write!(out, "{n}");
        }
        DiagnosticParam::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        DiagnosticParam::Timestamp(ms) => {
            // Wire form per starter user SCOPE R1: epoch ms. The
            // client renders into the resolved timezone; the
            // server-side render path leaves the raw ms in the
            // output so the consumer can format if needed.
            let _ = write!(out, "{ms}");
        }
        #[cfg(feature = "units")]
        DiagnosticParam::Quantity { canonical, quantity: _ } => {
            // Canonical-only render path (no prefs in scope here).
            // Callers that want prefs-aware unit conversion use
            // `MessageBundle::render_diagnostic` which threads a
            // `ResolvedPreferences` through. This branch is the
            // fallback for the bare `render` API where the caller
            // wants the canonical SI value formatted as-is.
            let _ = write!(out, "{canonical}");
        }
    }
}

/// Prefs-aware variant of [`interpolate_typed`] used by
/// [`crate::MessageBundle::render_diagnostic`]. `Quantity`-typed
/// params are converted from canonical SI to the caller's preferred
/// unit and rendered with the unit's display symbol (e.g. `25 °C`,
/// `15 kPa`). Everything else routes through the same plain
/// formatter.
#[cfg(feature = "preferences")]
#[must_use]
pub fn interpolate_typed_with_prefs(
    template: &str,
    params: &BTreeMap<String, DiagnosticParam>,
    prefs: &starter_spi::preferences::ResolvedPreferences,
) -> String {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '{' {
            out.push(c);
            continue;
        }
        let mut name = String::new();
        let mut closed = false;
        for nc in chars.by_ref() {
            if nc == '}' {
                closed = true;
                break;
            }
            name.push(nc);
        }
        if !closed {
            out.push('{');
            out.push_str(&name);
            continue;
        }
        match params.get(name.as_str()) {
            Some(p) => write_param_with_prefs(p, prefs, &mut out),
            None => {
                out.push('{');
                out.push_str(&name);
                out.push('}');
            }
        }
    }
    out
}

#[cfg(feature = "preferences")]
fn write_param_with_prefs(
    p: &DiagnosticParam,
    prefs: &starter_spi::preferences::ResolvedPreferences,
    out: &mut String,
) {
    use starter_spi::units::{convert_for_display, Quantity};
    use std::fmt::Write;
    match p {
        DiagnosticParam::Quantity { canonical, quantity } => {
            let target = match quantity {
                Quantity::Temperature => prefs.temperature_unit,
                Quantity::Pressure => prefs.pressure_unit,
                Quantity::Speed => prefs.speed_unit,
                Quantity::Length => prefs.length_unit,
                Quantity::Mass => prefs.mass_unit,
                // Quantities without a per-quantity pref render in
                // their canonical unit. New variants land here when
                // a prefs slot is added.
                _ => {
                    let _ = write!(out, "{canonical}");
                    return;
                }
            };
            match convert_for_display(*quantity, *canonical, target) {
                Ok(c) => {
                    let _ = write!(out, "{} {}", c.value, c.symbol);
                }
                Err(_) => {
                    // Conversion failed — fall through to canonical
                    // so the user still sees a number.
                    let _ = write!(out, "{canonical}");
                }
            }
        }
        // Every other variant uses the same plain formatter.
        other => write_param(other, out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_named_params() {
        let mut params = BTreeMap::new();
        params.insert("name".into(), DiagnosticParam::String("alice".into()));
        params.insert("n".into(), DiagnosticParam::I64(3));
        assert_eq!(
            interpolate_typed("Hi {name}, n={n}", &params),
            "Hi alice, n=3",
        );
    }

    #[test]
    fn leaves_unknown_placeholders_literal() {
        let mut params = BTreeMap::new();
        params.insert("name".into(), DiagnosticParam::String("alice".into()));
        assert_eq!(
            interpolate_typed("Hi {name}, missing={who}", &params),
            "Hi alice, missing={who}",
        );
    }

    #[test]
    fn handles_no_params() {
        let params = BTreeMap::new();
        assert_eq!(interpolate_typed("plain", &params), "plain");
    }

    #[test]
    fn handles_malformed_template() {
        let params = BTreeMap::new();
        assert_eq!(interpolate_typed("oops {name", &params), "oops {name");
    }
}
