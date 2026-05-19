//! Conversion to canonical SI storage units.
//!
//! Per R4 conversion factors are delegated to `uom`; this module is
//! the only place in the workspace that names `uom` directly. Add a
//! new unit variant → add a match arm here that routes to the right
//! `uom` quantity.

use uom::si::f64::{Length, Mass, Pressure, ThermodynamicTemperature, Velocity};
use uom::si::length::{foot, meter};
use uom::si::mass::{kilogram, pound};
use uom::si::pressure::{bar, kilopascal, psi};
use uom::si::thermodynamic_temperature::{degree_celsius, degree_fahrenheit};
use uom::si::velocity::{kilometer_per_hour, knot, meter_per_second, mile_per_hour};

use super::{Quantity, Unit, UnitError};

/// Convert `value` (in `source_unit`) to the canonical SI unit for
/// `quantity` and return the canonical numeric. Identity when
/// `source_unit` is already canonical.
///
/// Returns [`UnitError::UnitNotInQuantity`] if the `(quantity, unit)`
/// pair is not in the registry (e.g.
/// `(Quantity::Temperature, Unit::Pound)`).
pub fn normalize_for_storage(
    quantity: Quantity,
    value: f64,
    source_unit: Unit,
) -> Result<f64, UnitError> {
    match (quantity, source_unit) {
        (Quantity::Temperature, Unit::Celsius) => {
            Ok(ThermodynamicTemperature::new::<degree_celsius>(value).get::<degree_celsius>())
        }
        (Quantity::Temperature, Unit::Fahrenheit) => {
            Ok(ThermodynamicTemperature::new::<degree_fahrenheit>(value).get::<degree_celsius>())
        }

        (Quantity::Pressure, Unit::Kilopascal) => {
            Ok(Pressure::new::<kilopascal>(value).get::<kilopascal>())
        }
        (Quantity::Pressure, Unit::Psi) => Ok(Pressure::new::<psi>(value).get::<kilopascal>()),
        (Quantity::Pressure, Unit::Bar) => Ok(Pressure::new::<bar>(value).get::<kilopascal>()),

        (Quantity::Speed, Unit::MeterPerSecond) => {
            Ok(Velocity::new::<meter_per_second>(value).get::<meter_per_second>())
        }
        (Quantity::Speed, Unit::KilometerPerHour) => {
            Ok(Velocity::new::<kilometer_per_hour>(value).get::<meter_per_second>())
        }
        (Quantity::Speed, Unit::MilePerHour) => {
            Ok(Velocity::new::<mile_per_hour>(value).get::<meter_per_second>())
        }
        (Quantity::Speed, Unit::Knot) => Ok(Velocity::new::<knot>(value).get::<meter_per_second>()),

        (Quantity::Length, Unit::Meter) => Ok(Length::new::<meter>(value).get::<meter>()),
        (Quantity::Length, Unit::Foot) => Ok(Length::new::<foot>(value).get::<meter>()),

        (Quantity::Mass, Unit::Kilogram) => Ok(Mass::new::<kilogram>(value).get::<kilogram>()),
        (Quantity::Mass, Unit::Pound) => Ok(Mass::new::<pound>(value).get::<kilogram>()),

        (q, u) => Err(UnitError::UnitNotInQuantity {
            quantity: q,
            unit: u,
        }),
    }
}
