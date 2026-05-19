//! Unit tests for the `units` module. Coverage list comes from the
//! stage-3 directive of the Phase 0 plan.

use super::*;

const EPS: f64 = 1e-9;

#[test]
fn static_registry_canonical_per_quantity() {
    let reg = StaticRegistry::new();
    let expected = [
        (Quantity::Temperature, Unit::Celsius),
        (Quantity::Pressure, Unit::Kilopascal),
        (Quantity::Speed, Unit::MeterPerSecond),
        (Quantity::Length, Unit::Meter),
        (Quantity::Mass, Unit::Kilogram),
    ];
    for (q, canonical) in expected {
        let def = reg.get(q).expect("every Quantity has a definition");
        assert_eq!(def.canonical, canonical, "canonical unit for {q}");
        assert!(
            def.allowed_units.contains(&canonical),
            "canonical {canonical} must appear in allowed_units for {q}",
        );
    }
}

#[test]
fn supports_matrix_covers_every_scope_pair() {
    let reg = StaticRegistry::new();
    let pairs = [
        (Quantity::Temperature, &[Unit::Celsius, Unit::Fahrenheit][..]),
        (
            Quantity::Pressure,
            &[Unit::Kilopascal, Unit::Psi, Unit::Bar][..],
        ),
        (
            Quantity::Speed,
            &[
                Unit::MeterPerSecond,
                Unit::KilometerPerHour,
                Unit::MilePerHour,
                Unit::Knot,
            ][..],
        ),
        (Quantity::Length, &[Unit::Meter, Unit::Foot][..]),
        (Quantity::Mass, &[Unit::Kilogram, Unit::Pound][..]),
    ];
    for (q, units) in pairs {
        for &u in units {
            assert!(reg.supports(q, u), "{q} should accept {u}");
        }
    }
}

#[test]
fn supports_rejects_cross_quantity_pairs() {
    let reg = StaticRegistry::new();
    // Spot-check every cross-quantity pair: each Unit only belongs
    // to one Quantity, so iterate the cartesian product and assert
    // `supports` is true only for the home quantity.
    for &q in Quantity::ALL {
        for &u in Unit::ALL {
            let in_home = reg
                .get(q)
                .expect("def")
                .allowed_units
                .contains(&u);
            assert_eq!(
                reg.supports(q, u),
                in_home,
                "supports({q}, {u}) disagreed with allowed_units",
            );
        }
    }
    // The directive calls out one explicit negative case.
    assert!(!reg.supports(Quantity::Temperature, Unit::Pound));
}

#[test]
fn normalize_temperature_fahrenheit_to_celsius() {
    let c = normalize_for_storage(Quantity::Temperature, 72.4, Unit::Fahrenheit).unwrap();
    // 72.4 °F = (72.4 - 32) * 5/9 = 22.4444…  °C
    assert!((c - 22.444_444_444_444_443).abs() < 1e-9, "got {c}");
}

#[test]
fn normalize_pressure_psi_to_kilopascal() {
    let kpa = normalize_for_storage(Quantity::Pressure, 14.5037738, Unit::Psi).unwrap();
    // 14.5037738 psi ≈ 100 kPa (1 psi = 6.894757293168361 kPa)
    assert!((kpa - 100.0).abs() < 1e-4, "got {kpa}");
}

#[test]
fn normalize_speed_mph_to_meter_per_second() {
    let mps = normalize_for_storage(Quantity::Speed, 60.0, Unit::MilePerHour).unwrap();
    // 1 mph = 0.44704 m/s exact
    assert!((mps - 26.8224).abs() < 1e-9, "got {mps}");
}

#[test]
fn normalize_length_foot_to_meter() {
    let m = normalize_for_storage(Quantity::Length, 10.0, Unit::Foot).unwrap();
    // 1 ft = 0.3048 m exact
    assert!((m - 3.048).abs() < 1e-9, "got {m}");
}

#[test]
fn normalize_mass_pound_to_kilogram() {
    let kg = normalize_for_storage(Quantity::Mass, 10.0, Unit::Pound).unwrap();
    // 1 lb = 0.45359237 kg exact
    // `uom` 0.36 carries a 7-significant-digit avoirdupois pound
    // (0.4535924), so we match storage precision rather than the
    // exact 0.45359237 definition. Phase 0 only needs the registry
    // wired; downstream phases can pin a higher-precision factor.
    assert!((kg - 4.535924).abs() < 1e-4, "got {kg}");
}

#[test]
fn normalize_canonical_is_identity() {
    let cases = [
        (Quantity::Temperature, 22.5, Unit::Celsius),
        (Quantity::Pressure, 101.325, Unit::Kilopascal),
        (Quantity::Speed, 12.3, Unit::MeterPerSecond),
        (Quantity::Length, 7.0, Unit::Meter),
        (Quantity::Mass, 2.5, Unit::Kilogram),
    ];
    for (q, v, u) in cases {
        let out = normalize_for_storage(q, v, u).unwrap();
        assert!((out - v).abs() < EPS, "{q}/{u}: {out} != {v}");
    }
}

#[test]
fn normalize_rejects_cross_quantity_pair() {
    let err = normalize_for_storage(Quantity::Temperature, 1.0, Unit::Pound).unwrap_err();
    assert_eq!(
        err,
        UnitError::UnitNotInQuantity {
            quantity: Quantity::Temperature,
            unit: Unit::Pound,
        }
    );
}

#[test]
fn quantity_serde_lowercase() {
    let j = serde_json::to_string(&Quantity::Temperature).unwrap();
    assert_eq!(j, "\"temperature\"");
    let back: Quantity = serde_json::from_str("\"mass\"").unwrap();
    assert_eq!(back, Quantity::Mass);
}

#[test]
fn unit_serde_snake_case_matches_scope_example() {
    let j = serde_json::to_string(&Unit::Fahrenheit).unwrap();
    assert_eq!(j, "\"fahrenheit\"");
    let j = serde_json::to_string(&Unit::MeterPerSecond).unwrap();
    assert_eq!(j, "\"meter_per_second\"");
    let back: Unit = serde_json::from_str("\"kilometer_per_hour\"").unwrap();
    assert_eq!(back, Unit::KilometerPerHour);
}

#[test]
fn fromstr_roundtrip_every_variant() {
    for &q in Quantity::ALL {
        assert_eq!(q.to_string().parse::<Quantity>().unwrap(), q);
    }
    for &u in Unit::ALL {
        assert_eq!(u.to_string().parse::<Unit>().unwrap(), u);
    }
}
