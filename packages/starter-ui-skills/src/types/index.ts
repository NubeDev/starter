// Domain types for the skills-management UI. Mirrors the public
// `starter-skills` Rust crate (see DOCS/agent/SKILLS.md) without
// taking any opinion on transport — a `SkillsAdapter` is the only
// I/O seam.

export type SkillTrust = "approved" | "quarantined";

/** Why a bundle is quarantined. Mirrors the registry's load-time matrix. */
export type SkillQuarantineReason =
  | "extension-contributed"
  | "frontmatter-opt-in"
  | "no-approval-row"
  | "hash-mismatch";

export interface SkillResourceRef {
  /** `file://` URL relative to the bundle directory (v1 only allows `file://`). */
  uri: string;
  /** blake3 hex of the resource bytes, frozen at bundle parse time. */
  contentHash: string;
  /** Convenience filename, derived from `uri`. May be omitted by the adapter. */
  name?: string;
  sizeBytes?: number;
}

export interface SkillSummary {
  /** Reverse-DNS skill id; equals the bundle dir name. */
  id: string;
  description: string;
  trust: SkillTrust;
  /** blake3 hex of the bundle (the "version"). */
  bundleHash: string;
  allowedTools: string[];
  modelHint?: string;
  /** Where the bundle was loaded from (host dir vs extension). */
  source: "host" | "extension";
  /** Populated when `trust === "quarantined"`. */
  quarantineReason?: SkillQuarantineReason;
  /** ISO-8601 timestamp of the latest approval row, if any. */
  approvedAt?: string;
  /** Principal that approved the current `bundleHash`, if any. */
  approvedBy?: string;
}

export interface Skill extends SkillSummary {
  /** Verbatim SKILL.md body (after frontmatter). */
  body: string;
  resources: SkillResourceRef[];
}

export interface SkillApprovalRow {
  skillId: string;
  bundleHash: string;
  /** ISO-8601 timestamp. */
  approvedAt: string;
  approvedBy: string;
}

export interface SkillsListResult {
  skills: SkillSummary[];
}

/**
 * The single transport seam. Library code never speaks HTTP/RPC/etc.
 * itself — the consumer plugs whatever stack they use behind this
 * interface (REST against starter-server, tauri commands, an
 * in-memory mock for tests, …).
 *
 * Methods take an optional `AbortSignal` so the hooks can cancel
 * stale requests when a component unmounts or its inputs change.
 */
export interface SkillsAdapter {
  list(signal?: AbortSignal): Promise<SkillsListResult>;
  get(id: string, signal?: AbortSignal): Promise<Skill>;
  /** Operator path. The backend records an `ApprovalRow`. */
  approve(
    id: string,
    bundleHash: string,
    signal?: AbortSignal,
  ): Promise<SkillApprovalRow>;
  /** Operator path. The backend removes the matching approval row. */
  revoke(
    id: string,
    bundleHash: string,
    signal?: AbortSignal,
  ): Promise<void>;
  /** Re-run `load_dir` on the host registry. Optional — adapters
   *  without a backing registry (mocks) can omit it. */
  reload?(signal?: AbortSignal): Promise<void>;
  /** History of approval rows. Optional — UIs that don't surface
   *  it can ignore this method, and adapters can throw. */
  listApprovals?(signal?: AbortSignal): Promise<SkillApprovalRow[]>;
}
