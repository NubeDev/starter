//! `Unit` — the closed enum of unit codes the platform recognises.
//!
//! The wire form is the lowercase, underscore-joined name (e.g.
//! `"fahrenheit"`, `"meter_per_second"`) per the SCOPE Per-series unit
//! metadata example. The same identifier is what `GET /v1/units`
//! exposes to clients. See [`super::quantity::Quantity`] for the R4
//! verbatim quote on why this enum is closed.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::UnitError;

/// Closed enum of unit codes for the v1 surface. Variants are locked
/// in stage 1 of the Phase 0 plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    // -- Temperature ----------------------------------------------------
    /// Degree Celsius — canonical unit for `Quantity::Temperature`.
    Celsius,
    /// Degree Fahrenheit.
    Fahrenheit,

    // -- Pressure -------------------------------------------------------
    /// Kilopascal — canonical unit for `Quantity::Pressure`.
    Kilopascal,
    /// Pound-force per square inch.
    Psi,
    /// Bar.
    Bar,

    // -- Speed ----------------------------------------------------------
    /// Metre per second — canonical unit for `Quantity::Speed`.
    MeterPerSecond,
    /// Kilometre per hour.
    KilometerPerHour,
    /// Mile per hour.
    MilePerHour,
    /// Knot (nautical mile per hour).
    Knot,

    // -- Length ---------------------------------------------------------
    /// Metre — canonical unit for `Quantity::Length`.
    Meter,
    /// International foot.
    Foot,

    // -- Mass -----------------------------------------------------------
    /// Kilogram — canonical unit for `Quantity::Mass`.
    Kilogram,
    /// International avoirdupois pound.
    Pound,
}

impl Unit {
    /// Lowercase wire identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Celsius => "celsius",
            Self::Fahrenheit => "fahrenheit",
            Self::Kilopascal => "kilopascal",
            Self::Psi => "psi",
            Self::Bar => "bar",
            Self::MeterPerSecond => "meter_per_second",
            Self::KilometerPerHour => "kilometer_per_hour",
            Self::MilePerHour => "mile_per_hour",
            Self::Knot => "knot",
            Self::Meter => "meter",
            Self::Foot => "foot",
            Self::Kilogram => "kilogram",
            Self::Pound => "pound",
        }
    }

    /// Every variant in declaration order.
    pub const ALL: &'static [Unit] = &[
        Unit::Celsius,
        Unit::Fahrenheit,
        Unit::Kilopascal,
        Unit::Psi,
        Unit::Bar,
        Unit::MeterPerSecond,
        Unit::KilometerPerHour,
        Unit::MilePerHour,
        Unit::Knot,
        Unit::Meter,
        Unit::Foot,
        Unit::Kilogram,
        Unit::Pound,
    ];
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Unit {
    type Err = UnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "celsius" => Ok(Self::Celsius),
            "fahrenheit" => Ok(Self::Fahrenheit),
            "kilopascal" => Ok(Self::Kilopascal),
            "psi" => Ok(Self::Psi),
            "bar" => Ok(Self::Bar),
            "meter_per_second" => Ok(Self::MeterPerSecond),
            "kilometer_per_hour" => Ok(Self::KilometerPerHour),
            "mile_per_hour" => Ok(Self::MilePerHour),
            "knot" => Ok(Self::Knot),
            "meter" => Ok(Self::Meter),
            "foot" => Ok(Self::Foot),
            "kilogram" => Ok(Self::Kilogram),
            "pound" => Ok(Self::Pound),
            other => Err(UnitError::UnknownUnit(other.to_owned())),
        }
    }
}
