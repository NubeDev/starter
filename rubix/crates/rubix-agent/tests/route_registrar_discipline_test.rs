//! Discipline gate: every rubix-agent HTTP route MUST be mounted
//! through [`crate::routes::RouteRegistrar`]. The registrar is
//! the single chokepoint that records `(method, path, description,
//! tags, schemas)` alongside the live `axum::Router`, so the
//! catalog projection (`GET /api/v1/admin/openapi.json`) and the
//! live router cannot drift.
//!
//! Rule: no file under `src/` (apart from `routes/registrar.rs`
//! itself) may contain the substring `.route(`. The registrar
//! exposes `mount(...)` so this text rule has no false positives.
//!
//! See docs/design/admin/README.md §"Route catalog".

use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED: &str = "routes/registrar.rs";

#[test]
fn no_raw_axum_route_mounts_outside_registrar() {
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders: Vec<String> = Vec::new();
    walk(&crate_src, &mut |path, contents| {
        let rel = path
            .strip_prefix(&crate_src)
            .expect("walk yields paths under src/");
        if rel.to_string_lossy().replace('\\', "/") == ALLOWED {
            return;
        }
        for (line_no, line) in contents.lines().enumerate() {
            // Strip line comments (Rust `//`) before checking. This
            // lets module-level doc-comments mention `.route(` when
            // documenting the rule without tripping the gate.
            let code = match line.find("//") {
                Some(idx) => &line[..idx],
                None => line,
            };
            if code.contains(".route(") {
                offenders.push(format!(
                    "{}:{}: {}",
                    rel.display(),
                    line_no + 1,
                    line.trim()
                ));
            }
        }
    });
    assert!(
        offenders.is_empty(),
        "raw axum `.route(` calls found outside `{}` — migrate them through \
         `RouteRegistrar::mount(...)`:\n{}",
        ALLOWED,
        offenders.join("\n"),
    );
}

fn walk(dir: &Path, visit: &mut impl FnMut(&PathBuf, &str)) {
    for entry in fs::read_dir(dir).expect("read_dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            walk(&path, visit);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let contents = fs::read_to_string(&path).expect("read_to_string");
        visit(&path, &contents);
    }
}
