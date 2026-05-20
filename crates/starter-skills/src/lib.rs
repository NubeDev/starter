//! `starter-skills` — skill bundle loader, content-hash quarantine,
//! and `SkillSelector` implementations behind the
//! [`starter_flow_spi::SkillSelector`] seam.
//!
//! Per the per-job SCOPE.md (Phase 1) and `DOCS/agent/SKILLS.md`
//! Decisions section, this stage ships only:
//!
//! - the `SKILL.md` frontmatter [`parser`] with `serde`
//!   `deny_unknown_fields`,
//! - the [`Bundle`] type and [`load_bundle`] directory walker that
//!   reads `SKILL.md` + every listed resource once at load time
//!   (R-skills-1: parse-once / no-templating),
//! - the [`SkillParseError`] structured-error enum that names the
//!   offending path on every failure mode.
//!
//! Subsequent stages add the bundle-hash, registry, approval store,
//! selectors, and ai-agent on-mount verification. Nothing in this
//! module performs templating, interpolation, or env expansion —
//! a `{{x}}` inside a `SKILL.md` body is literal text the model
//! will eventually see (the R4 anti-prompt-injection guarantee).

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod bundle;
pub mod error;
pub mod parser;

pub use bundle::{load_bundle, Bundle, Resource};
pub use error::SkillParseError;
pub use parser::{
    parse_skill_md, Frontmatter, ParsedSkill, Trust, SUPPORTED_RESOURCE_SCHEMES,
};
