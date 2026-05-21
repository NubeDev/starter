// # @nube/starter-ui-skills
//
// Reusable React components and hooks for managing a `starter-skills`
// registry: list bundles, inspect their `SKILL.md` body and resources,
// and approve / revoke quarantined ones. Transport-agnostic — supply a
// `SkillsAdapter` (an in-memory mock ships for demos; a REST adapter
// follows once the backend route lands).
//
// Quick start:
//
// ```tsx
// import {
//   SkillsManager,
//   createInMemorySkillsAdapter,
// } from "@nube/starter-ui-skills";
//
// const adapter = createInMemorySkillsAdapter({ skills: fixtureSkills });
// export const Page = () => <SkillsManager adapter={adapter} />;
// ```
//
// For full control compose the primitives directly: `SkillList`,
// `SkillFilterBar`, `SkillDetail`, `SkillActionButton` with the
// `useSkills` / `useSkill` hooks.
//
// See DOCS/agent/SKILLS.md for the registry contract this UI talks to.

export * from "./types/index.js";
export * from "./hooks/index.js";
export * from "./adapters/index.js";

export { SkillsManager } from "./components/skills-manager.js";
export type { SkillsManagerProps } from "./components/skills-manager.js";
export { SkillList } from "./components/skill-list.js";
export type { SkillListProps } from "./components/skill-list.js";
export { SkillListItem } from "./components/skill-list-item.js";
export type { SkillListItemProps } from "./components/skill-list-item.js";
export { SkillDetail } from "./components/skill-detail.js";
export type { SkillDetailProps } from "./components/skill-detail.js";
export { SkillFilterBar } from "./components/skill-filter-bar.js";
export type { SkillFilterBarProps } from "./components/skill-filter-bar.js";
export { SkillTrustBadge } from "./components/skill-trust-badge.js";
export type { SkillTrustBadgeProps } from "./components/skill-trust-badge.js";
export { SkillHash } from "./components/skill-hash.js";
export type { SkillHashProps } from "./components/skill-hash.js";
export { SkillActionButton } from "./components/skill-action-button.js";
export type { SkillActionButtonProps } from "./components/skill-action-button.js";

export { cn, shortHash, formatBytes, formatRelative } from "./lib/utils.js";
