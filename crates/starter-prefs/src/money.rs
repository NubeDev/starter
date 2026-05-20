//! Money: minor-units integer + ISO 4217 code, per SCOPE.md R1.
//!
//! > Money is **minor-units integer + ISO 4217 code**. Never floats.
//! > No implicit currency.
//!
//! This module deliberately does **not** do FX conversion. Cross-
//! currency math needs a rate-source service (date, source, spread,
//! caching) that hasn't been scoped yet — when it is, it lands as a
//! separate trait that takes [`Money`] in and returns [`Money`] out.
//! Until then, callers either show the stored currency as-is or
//! refuse to mix.
//!
//! The presentation-edge job is just formatting: take a [`Money`] +
//! the resolved currency preference, render a string with the
//! correct symbol and decimal placement.

use std::fmt;

use iso_currency::Currency;
use serde::{Deserialize, Serialize};

// NOTE: no `utoipa::ToSchema` derive — `iso_currency::Currency`
// doesn't implement `PartialSchema`. When this type joins the wire
// surface, wrap `currency` in a serde-string newtype that does.

/// An amount of money in a specific currency. Stored as minor units
/// (cents, pence, yen, …) so arithmetic stays exact.
///
/// Example: `Money { amount_minor: 12_345, currency: Currency::USD }`
/// is `$123.45`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    /// The amount expressed in the currency's minor unit (e.g. cents
    /// for USD, pence for GBP, yen for JPY — JPY has zero minor
    /// digits, so the minor unit *is* the major unit).
    pub amount_minor: i64,

    /// ISO 4217 currency code. Serialised as the 3-letter code
    /// (`"USD"`, `"AUD"`, …) by `iso_currency`'s serde impl.
    pub currency: Currency,
}

impl Money {
    /// Construct directly. Trivial; lets the field order be obvious
    /// at the call site.
    pub const fn new(amount_minor: i64, currency: Currency) -> Self {
        Self { amount_minor, currency }
    }

    /// The value as a `f64` in the major unit (`amount_minor /
    /// 10^exponent`). **Only** for display — never store this number
    /// back, never do arithmetic on it. See SCOPE R1.
    pub fn to_major_f64(&self) -> f64 {
        let scale = 10_i64.pow(self.currency.exponent().unwrap_or(0) as u32) as f64;
        self.amount_minor as f64 / scale
    }

    /// Render as `"<symbol><value>"` using the currency's own symbol
    /// (e.g. `"$123.45"`, `"£10.00"`, `"¥1234"`).
    ///
    /// This is the minimum-viable formatter — no locale-aware
    /// grouping, no negative-parenthesis convention. Wire ICU /
    /// `Intl.NumberFormat` on the client when you need richer output;
    /// `starter-i18n` is the right home for that work.
    pub fn format(&self) -> String {
        let exp = self.currency.exponent().unwrap_or(0) as usize;
        let major = self.to_major_f64();
        format!("{}{:.*}", self.currency.symbol().symbol, exp, major)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usd_formats_with_two_decimals() {
        let m = Money::new(12_345, Currency::USD);
        assert_eq!(m.format(), "$123.45");
    }

    #[test]
    fn jpy_has_no_minor_units() {
        // JPY exponent is 0 — minor unit *is* the yen.
        let m = Money::new(1_234, Currency::JPY);
        assert_eq!(m.format(), "¥1234");
    }

    #[test]
    fn gbp_formats_with_pound_symbol() {
        let m = Money::new(1_000, Currency::GBP);
        assert_eq!(m.format(), "£10.00");
    }

    #[test]
    fn negative_amount_renders_with_minus() {
        let m = Money::new(-500, Currency::USD);
        assert_eq!(m.format(), "$-5.00");
    }

    #[test]
    fn display_matches_format() {
        let m = Money::new(99, Currency::EUR);
        assert_eq!(format!("{m}"), m.format());
    }
}
