//! The Rhai engine factory: a fresh, locked-down engine per insight execution.
//!
//! Every limit a hostile script could abuse is set here, and the dangerous
//! capabilities are removed rather than merely left unregistered:
//!
//! - `set_max_operations` caps total interpreter steps → an infinite loop trips
//!   it and returns a limit error instead of hanging.
//! - `set_max_call_levels` caps recursion depth → no stack blowout.
//! - `set_max_string_size` / `set_max_array_size` / `set_max_map_size` cap the
//!   in-script data a script can build.
//! - `on_progress` checks a wall-clock deadline every N operations → a script
//!   that is slow without looping (a pathological single expression) still stops.
//! - `set_module_resolver(DummyModuleResolver)` makes `import` fail: per the Rhai
//!   safety guidance, *not registering* a module is not enough to block `import`,
//!   so the resolver is explicitly a no-op that errors.
//!
//! No file, network, or `eval` API is ever registered — the only surface a script
//! sees is the curated `Frame` methods from [`crate::api`]. One engine per
//! execution is cheap and carries no cross-tenant state.

use std::time::Instant;

use rhai::module_resolvers::DummyModuleResolver;
use rhai::{Engine, EvalAltResult};

use crate::limits::Limits;

/// Build a sandboxed engine bounded by `limits`, with a wall-clock deadline
/// measured from `started`. The deadline is enforced in `on_progress`, which Rhai
/// calls periodically during evaluation; tripping it aborts with a token the run
/// layer maps to [`crate::InsightError::LimitExceeded`].
pub fn build(limits: &Limits, started: Instant) -> Engine {
    let mut engine = Engine::new_raw();

    engine.set_max_operations(limits.max_operations);
    engine.set_max_call_levels(limits.max_call_levels);
    engine.set_max_string_size(limits.max_string_size);
    engine.set_max_array_size(limits.max_array_size);
    engine.set_max_map_size(limits.max_map_size);
    // Bound expression nesting so a deeply-nested literal cannot blow the parser.
    engine.set_max_expr_depths(limits.max_expr_depth, limits.max_expr_depth);

    // `import` must fail even though no modules are registered: a dummy resolver
    // turns every import into an error rather than a silent or partial load.
    engine.set_module_resolver(DummyModuleResolver::new());

    let deadline = limits.deadline;
    engine.on_progress(move |_ops| {
        if started.elapsed() >= deadline {
            // A non-unit value from on_progress aborts the script; the message is
            // surfaced as a limit error.
            Some(WALL_CLOCK_TOKEN.into())
        } else {
            None
        }
    });

    engine
}

/// The sentinel an aborted-by-deadline script carries, recognised by the run
/// layer so a wall-clock stop is reported as a limit (not a runtime) error.
pub const WALL_CLOCK_TOKEN: &str = "insight:wall-clock-deadline";

/// Classify a Rhai evaluation error as a limit breach vs an ordinary runtime
/// error, so the tenant sees the right [`crate::InsightError`] variant.
pub fn is_limit_error(err: &EvalAltResult) -> bool {
    matches!(
        err,
        EvalAltResult::ErrorTooManyOperations(_)
            | EvalAltResult::ErrorStackOverflow(_)
            | EvalAltResult::ErrorDataTooLarge(_, _)
            | EvalAltResult::ErrorTerminated(_, _)
    )
}
