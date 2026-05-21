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
//! Phase 2 adds the content-hash algorithm under
//! [`approval`] (`hash_bundle` + `EXCLUDED`, R-skills-2 / agent
//! R4). Subsequent stages add the registry, approval store,
//! selectors, and ai-agent on-mount verification. Nothing in this
//! module performs templating, interpolation, or env expansion —
//! a `{{x}}` inside a `SKILL.md` body is literal text the model
//! will eventually see (the R4 anti-prompt-injection guarantee).

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod approval;
pub mod bundle;
pub mod error;
pub mod mount;
pub mod parser;
pub mod registry;
pub mod selector;
pub mod store;

pub use bundle::{load_bundle, Bundle, Resource};
pub use error::SkillParseError;
pub use parser::{
    parse_skill_md, Frontmatter, ParsedSkill, Trust, SUPPORTED_RESOURCE_SCHEMES,
};
pub use registry::{ContributedSkill, LoadError, Skill, SkillRegistry, SkillRegistryBuilder};
pub use selector::{
    FirstSkillSelector, KeywordSkillSelector, LlmSkillSelector, LlmSkillSelectorBuilder,
    SelectorStrategy, DEFAULT_LLM_SELECTOR_MODEL, DEFAULT_LLM_SELECTOR_TIMEOUT,
};
pub use mount::{read_and_verify, ResourceMountError};
pub use store::{ApprovalRow, ApprovalStore, ApprovalStoreError, InMemoryApprovalStore};
