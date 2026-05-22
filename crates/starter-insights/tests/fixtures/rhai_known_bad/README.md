# Rhai known-bad-script fixtures (R-ins-4)

Each `.rhai` file in this directory is a script the locked Rhai
sandbox profile (`crate::rhai_sandbox::make_engine`) MUST reject —
either at compile time or at runtime within the operation budget.

The CI smoke `rhai_sandbox_known_bad.rs` loads every `*.rhai`
sibling, runs it through the locked engine, and asserts each one
fails. Categories covered:

| File | Category | Expected failure mode |
|---|---|---|
| `eval.rhai` | code injection | `eval` symbol disabled |
| `import_module.rhai` | sandbox escape | `import` symbol disabled / compile error |
| `export_module.rhai` | sandbox escape | `export` symbol disabled / compile error |
| `tight_loop_dos.rhai` | CPU DoS | operation budget exhausted |
| `recursive_dos.rhai` | stack DoS | recursion / expression-depth cap |
| `huge_string.rhai` | memory DoS | `max_string_size` cap |
| `huge_array.rhai` | memory DoS | `max_array_size` cap |
| `huge_map.rhai` | memory DoS | `max_map_size` cap |

Adding new attack patterns: drop a new `.rhai` file here; the smoke
picks it up via `glob`. If a script is expected to succeed under the
sandbox (regression guard for a known-good idiom), put it in the
sibling `rhai_known_good/` directory instead — that one is read by a
separate smoke that asserts success.
