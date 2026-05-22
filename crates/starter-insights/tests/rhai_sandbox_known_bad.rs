//! R-ins-4 known-bad-script CI smoke.
//!
//! Loads every `.rhai` fixture under
//! `tests/fixtures/rhai_known_bad/` and asserts the locked Rhai
//! sandbox profile (`starter_insights::rhai_sandbox::make_engine`)
//! rejects it — either at compile time or at runtime within the
//! operation / size budgets.
//!
//! This is the R-ins-4 mechanical guard. Adding a new attack
//! category is one file in the fixture directory; the smoke picks
//! it up automatically.

use std::fs;
use std::path::PathBuf;

use starter_insights::rhai_sandbox::make_engine;

fn fixtures_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests");
    p.push("fixtures");
    p.push("rhai_known_bad");
    p
}

fn load_scripts() -> Vec<(String, String)> {
    let dir = fixtures_dir();
    let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("read fixtures dir {dir:?}: {e}"));
    let mut out = Vec::new();
    for e in entries {
        let e = e.unwrap();
        let path = e.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rhai") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let body = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        out.push((name, body));
    }
    // Stable iteration order for diagnosable CI output.
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn every_known_bad_script_is_rejected_by_the_locked_sandbox() {
    let scripts = load_scripts();
    assert!(
        scripts.len() >= 8,
        "expected at least 8 attack fixtures; got {}",
        scripts.len()
    );

    // Use a tight per-fixture operation cap so CPU-DoS scripts fail
    // in test time rather than chewing through the default 1M budget.
    // Memory-cap fixtures fail on a different code path; the
    // operation cap only matters for tight_loop_dos.rhai +
    // recursive_dos.rhai.
    let engine = make_engine(Some(50_000));
    for (name, body) in scripts {
        // Try compile first so the disabled-symbol path is tested
        // separately from runtime caps. `import` / `export` fail
        // here; `eval` fails at runtime (eval is a function call,
        // disable_symbol surfaces the disabled symbol at runtime).
        // "Rejected" means either compile errored OR eval_ast errored.
        // The `compile` Err arm IS a rejection — the sandbox refused
        // the symbol at parse time. The `eval_ast` Err arm IS a
        // rejection — the sandbox blew the operation / size budget at
        // runtime. Only an `Ok` from eval_ast is a leak.
        let rejected = match engine.compile(&body) {
            Err(_) => true,
            Ok(ast) => engine.eval_ast::<rhai::Dynamic>(&ast).is_err(),
        };
        assert!(
            rejected,
            "known-bad fixture `{name}` was NOT rejected by the locked sandbox: \
             this is a R-ins-4 regression — investigate `crates/starter-insights/src/rhai_sandbox.rs`"
        );
    }
}

#[test]
fn fixture_directory_has_attack_categories() {
    let names: Vec<String> = load_scripts().into_iter().map(|(n, _)| n).collect();
    for expected in [
        "eval.rhai",
        "import_module.rhai",
        "export_module.rhai",
        "tight_loop_dos.rhai",
        "recursive_dos.rhai",
        "huge_string.rhai",
        "huge_array.rhai",
        "huge_map.rhai",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "fixture `{expected}` missing from rhai_known_bad/ — \
             attack-category coverage is load-bearing for R-ins-4"
        );
    }
}
