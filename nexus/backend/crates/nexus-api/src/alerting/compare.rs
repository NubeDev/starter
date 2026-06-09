//! The threshold comparison — does an evaluated value breach the rule?
//!
//! Pure and tiny: the rule stores an operator string and a threshold; this maps
//! the operator and answers whether `value op threshold` holds. An unknown
//! operator is a config error surfaced to the caller, not a silent false (which
//! would make a misconfigured rule never fire).

/// Whether `value op threshold` holds. `Err` names an unknown operator.
pub fn breaches(value: f64, op: &str, threshold: f64) -> Result<bool, String> {
    Ok(match op {
        "gt" => value > threshold,
        "gte" => value >= threshold,
        "lt" => value < threshold,
        "lte" => value <= threshold,
        "eq" => value == threshold,
        "ne" => value != threshold,
        other => return Err(format!("unknown alert operator `{other}`")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operators_compare_as_expected() {
        assert!(breaches(95.0, "gt", 90.0).unwrap());
        assert!(!breaches(90.0, "gt", 90.0).unwrap());
        assert!(breaches(90.0, "gte", 90.0).unwrap());
        assert!(breaches(5.0, "lt", 10.0).unwrap());
        assert!(breaches(10.0, "lte", 10.0).unwrap());
        assert!(breaches(1.0, "eq", 1.0).unwrap());
        assert!(breaches(2.0, "ne", 1.0).unwrap());
    }

    #[test]
    fn unknown_operator_is_an_error() {
        assert!(breaches(1.0, "between", 0.0).is_err());
    }
}
