//! Rubix bundled skills.
//!
//! Six `SKILL.md` bundles, one per rubix goal, embedded into the crate
//! via `include_dir!` and exposed through a single `bundled()` helper
//! that returns the directory tree. The agent binary feeds this into
//! the host's `SkillRegistry` at boot.
//!
//! Rubix-bundled skills are **approved** by default (host-dir trust
//! level). Extension-contributed skills default to quarantined;
//! operator-dropped skills are approved. All three buckets flow into
//! the same registry. See [docs/design/skills/](../../docs/design/skills/README.md).

use include_dir::{include_dir, Dir};

/// All bundled rubix skills, embedded at compile time.
///
/// The directory layout is `skills/<goal>/SKILL.md` plus any
/// `resources/*` files referenced by the skill frontmatter.
pub static BUNDLED: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/skills");

/// Return the embedded skill bundle. The agent binary wraps this in
/// the starter-skills loader; rubix-skills itself does not depend on
/// starter-skills to keep this crate tiny — content crates ship
/// content, not behaviour. See [docs/design/skills/](../../docs/design/skills/README.md).
pub fn bundled() -> &'static Dir<'static> {
    &BUNDLED
}
