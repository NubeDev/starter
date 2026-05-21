import type {
  Skill,
  SkillApprovalRow,
  SkillSummary,
  SkillsAdapter,
} from "../types/index.js";

export interface InMemorySkillsAdapterOptions {
  skills: Skill[];
  approvals?: SkillApprovalRow[];
  /** Artificial latency on each call (ms). Default: 0. */
  latencyMs?: number;
  /** Principal recorded on `approve()`. Default: `"demo"`. */
  defaultApprovedBy?: string;
}

// Reference adapter for demos and tests. Keeps everything in memory
// and mutates the provided arrays in place so re-renders observe
// approve/revoke immediately.
export function createInMemorySkillsAdapter(
  opts: InMemorySkillsAdapterOptions,
): SkillsAdapter {
  const latency = opts.latencyMs ?? 0;
  const approvedBy = opts.defaultApprovedBy ?? "demo";
  const state: {
    skills: Skill[];
    approvals: SkillApprovalRow[];
  } = {
    skills: [...opts.skills],
    approvals: [...(opts.approvals ?? [])],
  };

  const wait = (signal?: AbortSignal) =>
    new Promise<void>((resolve, reject) => {
      if (!latency) return resolve();
      const t = setTimeout(resolve, latency);
      signal?.addEventListener(
        "abort",
        () => {
          clearTimeout(t);
          reject(signal.reason ?? new DOMException("aborted", "AbortError"));
        },
        { once: true },
      );
    });

  const summary = (s: Skill): SkillSummary => {
    const row = state.approvals.find(
      (a) => a.skillId === s.id && a.bundleHash === s.bundleHash,
    );
    return {
      id: s.id,
      description: s.description,
      trust: row ? "approved" : s.trust,
      bundleHash: s.bundleHash,
      allowedTools: s.allowedTools,
      modelHint: s.modelHint,
      source: s.source,
      quarantineReason: row ? undefined : s.quarantineReason,
      approvedAt: row?.approvedAt ?? s.approvedAt,
      approvedBy: row?.approvedBy ?? s.approvedBy,
    };
  };

  return {
    async list(signal) {
      await wait(signal);
      return { skills: state.skills.map(summary) };
    },
    async get(id, signal) {
      await wait(signal);
      const skill = state.skills.find((s) => s.id === id);
      if (!skill) throw new Error(`skill not found: ${id}`);
      const s = summary(skill);
      return { ...skill, ...s };
    },
    async approve(id, bundleHash, signal) {
      await wait(signal);
      const skill = state.skills.find((s) => s.id === id);
      if (!skill) throw new Error(`skill not found: ${id}`);
      if (skill.bundleHash !== bundleHash) {
        throw new Error(
          `hash mismatch: bundle is at ${skill.bundleHash}, asked to approve ${bundleHash}`,
        );
      }
      const row: SkillApprovalRow = {
        skillId: id,
        bundleHash,
        approvedAt: new Date().toISOString(),
        approvedBy,
      };
      // Replace any existing row for the same (id, hash) pair.
      state.approvals = state.approvals.filter(
        (a) => !(a.skillId === id && a.bundleHash === bundleHash),
      );
      state.approvals.push(row);
      return row;
    },
    async revoke(id, bundleHash, signal) {
      await wait(signal);
      state.approvals = state.approvals.filter(
        (a) => !(a.skillId === id && a.bundleHash === bundleHash),
      );
    },
    async reload(signal) {
      await wait(signal);
      // In-memory adapter has nothing to re-load; no-op for parity.
    },
    async listApprovals(signal) {
      await wait(signal);
      return [...state.approvals];
    },
  };
}
