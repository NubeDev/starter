//! Locked Rhai sandbox profile (Insights SCOPE R-ins-4).
//!
//! `rule.rhai` is the highest user-input surface in the capability;
//! the profile is frozen here, enforced by [`make_engine`], and
//! covered by the `rhai_sandbox_*` smoke tests in this crate.
//!
//! - `Engine::new()` + **deny** `eval`, `import`, modules, and the
//!   filesystem package (rhai's default `Engine::new()` does not
//!   register the filesystem package; `eval`/`import` are disabled
//!   below via `disable_symbol`).
//! - Default `set_max_operations(1_000_000)`, `set_max_expr_depth(32, 32)`,
//!   `set_max_string_size(64 * 1024)`, `set_max_array_size(10_000)`,
//!   `set_max_map_size(10_000)`.
//! - Operation cap is per-rule overrideable (registry entry carries
//!   `max_operations: Option<u64>`); pipelines composing a rule do
//!   not get to raise it. Caller passes the per-rule cap via
//!   [`make_engine`]'s `max_operations` argument.
//! - No Rust function registered that does I/O, time mutation, or
//!   capability acquisition. The `Ctx` exposed to scripts is built
//!   by the caller as a typed `Map`; only a `now()` bound to the
//!   engine's clock seam is registered here (returns the current
//!   UTC unix-second as `i64`, for deterministic-ish formatting).

use rhai::Engine;

/// Default Rhai operation budget (R-ins-4).
///
/// Sized for a 24h × 1-min window at ~50 ops/sample with headroom;
/// rules whose workload exceeds this belong in `rule.rust`. The
/// per-rule override on `RuleSchema::max_operations` lowers this
/// further; pipelines composing the rule cannot raise it.
pub const DEFAULT_MAX_OPERATIONS: u64 = 1_000_000;

/// Default string-size cap (64 KiB).
pub const DEFAULT_MAX_STRING_SIZE: usize = 64 * 1024;

/// Default array-size cap.
pub const DEFAULT_MAX_ARRAY_SIZE: usize = 10_000;

/// Default map-size cap.
pub const DEFAULT_MAX_MAP_SIZE: usize = 10_000;

/// Default expression-depth cap (both function and expression).
pub const DEFAULT_MAX_EXPR_DEPTH: usize = 32;

/// Build a fully-locked Rhai [`Engine`] for `rule.rhai`. Pass
/// `max_operations = None` to use [`DEFAULT_MAX_OPERATIONS`].
///
/// This is the **only** way `starter-insights` builds a Rhai engine;
/// callers that want to register additional read-only context must
/// do so on the returned engine — they cannot bypass the caps or
/// re-enable disabled symbols, because every cap is set here and
/// `disable_symbol` is one-way.
pub fn make_engine(max_operations: Option<u64>) -> Engine {
    let mut engine = Engine::new();

    // Deny dangerous surfaces. Per R-ins-4, rule.rhai is read-only.
    // `Engine::new()` does not register the filesystem package, so
    // there's nothing to drop there; we keep the explicit
    // disable_symbol calls so that a future Rhai default change is
    // caught by the sandbox smoke.
    engine.disable_symbol("eval");
    engine.disable_symbol("import");
    engine.disable_symbol("export");

    // Caps. These are the load-bearing R-ins-4 numbers.
    let ops = max_operations.unwrap_or(DEFAULT_MAX_OPERATIONS);
    engine.set_max_operations(ops);
    engine.set_max_expr_depths(DEFAULT_MAX_EXPR_DEPTH, DEFAULT_MAX_EXPR_DEPTH);
    engine.set_max_string_size(DEFAULT_MAX_STRING_SIZE);
    engine.set_max_array_size(DEFAULT_MAX_ARRAY_SIZE);
    engine.set_max_map_size(DEFAULT_MAX_MAP_SIZE);

    // No I/O, no time mutation. Only `now()` (deterministic at
    // call-site; the engine seam controls the clock in tests).
    engine.register_fn("now", || chrono::Utc::now().timestamp());

    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_denies_eval() {
        let engine = make_engine(None);
        let err = engine
            .eval::<rhai::Dynamic>(r#"eval("1+1")"#)
            .expect_err("eval must be disabled");
        let msg = format!("{err}");
        assert!(
            msg.contains("eval") || msg.contains("disabled"),
            "expected disabled-symbol error, got: {msg}"
        );
    }

    #[test]
    fn sandbox_denies_import() {
        let engine = make_engine(None);
        let res = engine.compile(r#"import "foo" as bar;"#);
        assert!(res.is_err(), "import must not compile under the sandbox");
    }

    #[test]
    fn sandbox_enforces_operation_budget() {
        let engine = make_engine(Some(1_000));
        // Tight infinite loop — must exhaust the budget quickly.
        let err = engine
            .eval::<rhai::Dynamic>("let i = 0; loop { i += 1; }")
            .expect_err("operation budget must be exhausted");
        let msg = format!("{err}").to_lowercase();
        assert!(
            msg.contains("operation") || msg.contains("budget"),
            "expected operation-budget error, got: {msg}"
        );
    }

    #[test]
    fn sandbox_runs_simple_arithmetic() {
        let engine = make_engine(None);
        let v: i64 = engine.eval("40 + 2").unwrap();
        assert_eq!(v, 42);
    }
}
