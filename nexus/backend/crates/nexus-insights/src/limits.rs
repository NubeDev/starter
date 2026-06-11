//! The sandbox limits an insight runs under. Conservative defaults that stop a
//! pathological script fast while leaving generous headroom for a real
//! orchestration script (which delegates the heavy compute to the engine, so it
//! executes very few interpreter operations itself).

use std::time::Duration;

/// Per-execution sandbox bounds. Constructed via [`Limits::default`] and tunable
/// by the host if a deployment needs different ceilings.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum interpreter operations before the run is aborted. The script only
    /// orchestrates vetted primitives, so even a complex insight stays well under
    /// this; an infinite loop trips it.
    pub max_operations: u64,
    /// Maximum call/recursion depth.
    pub max_call_levels: usize,
    /// Maximum size of any single string a script builds, in bytes.
    pub max_string_size: usize,
    /// Maximum length of any single array a script builds.
    pub max_array_size: usize,
    /// Maximum size of any single object map a script builds.
    pub max_map_size: usize,
    /// Maximum AST expression nesting depth (parser bound).
    pub max_expr_depth: usize,
    /// Wall-clock budget for the whole execution. Enforced via `on_progress`, so a
    /// script that is slow without looping is still stopped.
    pub deadline: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_operations: 5_000_000,
            max_call_levels: 32,
            max_string_size: 256 * 1024,
            max_array_size: 100_000,
            max_map_size: 10_000,
            max_expr_depth: 64,
            deadline: Duration::from_secs(5),
        }
    }
}
