import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  Skill,
  SkillsAdapter,
  SkillSummary,
} from "../types/index.js";

export type SkillsFilter = "all" | "approved" | "quarantined";

export interface UseSkillsOptions {
  adapter: SkillsAdapter;
  /** Auto-load on mount. Default: true. */
  autoLoad?: boolean;
  /** Optional poll interval in ms. Omit to disable. */
  refreshIntervalMs?: number;
  onError?: (err: unknown) => void;
}

export interface UseSkillsReturn {
  skills: SkillSummary[];
  loading: boolean;
  error: string | null;
  /** Filtered view. */
  filter: SkillsFilter;
  setFilter: (f: SkillsFilter) => void;
  /** Free-text search across id + description. */
  search: string;
  setSearch: (q: string) => void;
  visible: SkillSummary[];
  /** Re-fetch the list. */
  refresh: () => Promise<void>;
  /** Operator actions. Optimistic — the list is refreshed after. */
  approve: (id: string, bundleHash: string) => Promise<void>;
  revoke: (id: string, bundleHash: string) => Promise<void>;
  reload: () => Promise<void>;
}

export function useSkills(opts: UseSkillsOptions): UseSkillsReturn {
  const { adapter, autoLoad = true, refreshIntervalMs, onError } = opts;
  const [skills, setSkills] = useState<SkillSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<SkillsFilter>("all");
  const [search, setSearch] = useState("");
  const abortRef = useRef<AbortController | null>(null);

  const refresh = useCallback(async () => {
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;
    setLoading(true);
    setError(null);
    try {
      const res = await adapter.list(ctrl.signal);
      if (ctrl.signal.aborted) return;
      setSkills(res.skills);
    } catch (err) {
      if (ctrl.signal.aborted) return;
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      onError?.(err);
    } finally {
      if (abortRef.current === ctrl) abortRef.current = null;
      setLoading(false);
    }
  }, [adapter, onError]);

  useEffect(() => {
    if (autoLoad) void refresh();
    return () => abortRef.current?.abort();
  }, [autoLoad, refresh]);

  useEffect(() => {
    if (!refreshIntervalMs) return;
    const t = window.setInterval(() => void refresh(), refreshIntervalMs);
    return () => window.clearInterval(t);
  }, [refresh, refreshIntervalMs]);

  const approve = useCallback(
    async (id: string, bundleHash: string) => {
      try {
        await adapter.approve(id, bundleHash);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        onError?.(err);
        throw err;
      }
      await refresh();
    },
    [adapter, onError, refresh],
  );

  const revoke = useCallback(
    async (id: string, bundleHash: string) => {
      try {
        await adapter.revoke(id, bundleHash);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        onError?.(err);
        throw err;
      }
      await refresh();
    },
    [adapter, onError, refresh],
  );

  const reload = useCallback(async () => {
    if (!adapter.reload) {
      await refresh();
      return;
    }
    try {
      await adapter.reload();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      onError?.(err);
      throw err;
    }
    await refresh();
  }, [adapter, onError, refresh]);

  const visible = useMemo(() => {
    const q = search.trim().toLowerCase();
    return skills.filter((s) => {
      if (filter !== "all" && s.trust !== filter) return false;
      if (!q) return true;
      return (
        s.id.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q)
      );
    });
  }, [skills, filter, search]);

  return {
    skills,
    loading,
    error,
    filter,
    setFilter,
    search,
    setSearch,
    visible,
    refresh,
    approve,
    revoke,
    reload,
  };
}

export interface UseSkillOptions {
  adapter: SkillsAdapter;
  id: string | null | undefined;
  onError?: (err: unknown) => void;
}

export interface UseSkillReturn {
  skill: Skill | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function useSkill(opts: UseSkillOptions): UseSkillReturn {
  const { adapter, id, onError } = opts;
  const [skill, setSkill] = useState<Skill | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const refresh = useCallback(async () => {
    if (!id) {
      setSkill(null);
      return;
    }
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;
    setLoading(true);
    setError(null);
    try {
      const s = await adapter.get(id, ctrl.signal);
      if (ctrl.signal.aborted) return;
      setSkill(s);
    } catch (err) {
      if (ctrl.signal.aborted) return;
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      onError?.(err);
    } finally {
      if (abortRef.current === ctrl) abortRef.current = null;
      setLoading(false);
    }
  }, [adapter, id, onError]);

  useEffect(() => {
    void refresh();
    return () => abortRef.current?.abort();
  }, [refresh]);

  return { skill, loading, error, refresh };
}
