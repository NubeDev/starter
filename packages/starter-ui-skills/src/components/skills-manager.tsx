import * as React from "react";
import { cn } from "../lib/utils.js";
import type { Skill, SkillSummary, SkillsAdapter } from "../types/index.js";
import { useSkill, useSkills } from "../hooks/use-skills.js";
import { SkillList } from "./skill-list.js";
import { SkillFilterBar } from "./skill-filter-bar.js";
import { SkillDetail } from "./skill-detail.js";
import { SkillActionButton } from "./skill-action-button.js";

export interface SkillsManagerProps {
  adapter: SkillsAdapter;
  className?: string;
  title?: React.ReactNode;
  headerExtras?: React.ReactNode;
  /** Plug your markdown renderer for the body (defaults to <pre>). */
  renderBody?: (body: string) => React.ReactNode;
  /** Custom confirmation copy. Set `null` to disable the prompt. */
  revokeConfirm?: string | null;
  /** Default: 10000ms. Set 0 to disable. */
  refreshIntervalMs?: number;
  onSelect?: (skill: SkillSummary | null) => void;
}

// Opinionated end-to-end skills manager. For full control compose
// `<SkillList>` + `<SkillFilterBar>` + `<SkillDetail>` with the
// `useSkills` / `useSkill` hooks.
export function SkillsManager(props: SkillsManagerProps): React.ReactElement {
  const {
    adapter,
    className,
    title = "Skills",
    headerExtras,
    renderBody,
    revokeConfirm = "Revoke approval for this bundle hash? Future runs will skip this skill until it is re-approved.",
    refreshIntervalMs = 10_000,
    onSelect,
  } = props;

  const {
    visible,
    skills,
    loading,
    error,
    filter,
    setFilter,
    search,
    setSearch,
    approve,
    revoke,
    reload,
  } = useSkills({ adapter, refreshIntervalMs });

  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const detail = useSkill({ adapter, id: selectedId });

  // Auto-select the first visible skill (and revise when filters drop it).
  React.useEffect(() => {
    if (!visible.length) {
      if (selectedId !== null) {
        setSelectedId(null);
        onSelect?.(null);
      }
      return;
    }
    const stillVisible = selectedId && visible.some((s) => s.id === selectedId);
    if (!stillVisible) {
      const first = visible[0]!;
      setSelectedId(first.id);
      onSelect?.(first);
    }
    // intentional: avoid re-firing when onSelect identity changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible, selectedId]);

  const counts = React.useMemo(
    () => ({
      all: skills.length,
      approved: skills.filter((s) => s.trust === "approved").length,
      quarantined: skills.filter((s) => s.trust === "quarantined").length,
    }),
    [skills],
  );

  const [busyId, setBusyId] = React.useState<string | null>(null);
  const doApprove = async (s: Skill) => {
    setBusyId(s.id);
    try {
      await approve(s.id, s.bundleHash);
      await detail.refresh();
    } finally {
      setBusyId(null);
    }
  };
  const doRevoke = async (s: Skill) => {
    setBusyId(s.id);
    try {
      await revoke(s.id, s.bundleHash);
      await detail.refresh();
    } finally {
      setBusyId(null);
    }
  };

  return (
    <div
      data-slot="skills-manager"
      className={cn(
        "flex h-full min-h-0 w-full flex-col bg-gradient-to-b from-background to-muted/30 text-foreground",
        className,
      )}
    >
      <header className="flex flex-col gap-3 border-b border-border/60 bg-background/70 px-4 py-3 backdrop-blur">
        <div className="flex items-center gap-2">
          <div className="text-sm font-semibold">{title}</div>
          <div className="ml-auto flex items-center gap-2">
            {headerExtras}
            <SkillActionButton
              variant="ghost"
              loading={loading}
              onClick={() => void reload()}
              aria-label="Reload from disk"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="h-3.5 w-3.5"
                aria-hidden
              >
                <path d="M21 12a9 9 0 11-3-6.7L21 8" />
                <path d="M21 3v5h-5" />
              </svg>
              Reload
            </SkillActionButton>
          </div>
        </div>
        <SkillFilterBar
          filter={filter}
          onFilterChange={setFilter}
          search={search}
          onSearchChange={setSearch}
          counts={counts}
        />
        {error ? (
          <div
            role="alert"
            className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-1.5 text-xs text-destructive"
          >
            {error}
          </div>
        ) : null}
      </header>

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-0 md:grid-cols-[minmax(18rem,22rem)_1fr]">
        <aside className="min-h-0 overflow-y-auto border-border/40 p-3 md:border-r">
          {loading && skills.length === 0 ? (
            <ListSkeleton />
          ) : (
            <SkillList
              skills={visible}
              selectedId={selectedId}
              onSelect={(s) => {
                setSelectedId(s.id);
                onSelect?.(s);
              }}
              emptyMessage={
                skills.length === 0
                  ? "No skills are registered."
                  : "No skills match the current filter."
              }
            />
          )}
        </aside>

        <main className="min-h-0 overflow-hidden">
          {detail.loading && !detail.skill ? (
            <DetailSkeleton />
          ) : detail.skill ? (
            <SkillDetail
              skill={detail.skill}
              renderBody={renderBody}
              actions={
                <DetailActions
                  skill={detail.skill}
                  busy={busyId === detail.skill.id}
                  onApprove={() => void doApprove(detail.skill!)}
                  onRevoke={() => {
                    if (
                      revokeConfirm &&
                      typeof window !== "undefined" &&
                      !window.confirm(revokeConfirm)
                    ) {
                      return;
                    }
                    void doRevoke(detail.skill!);
                  }}
                />
              }
            />
          ) : (
            <div className="flex h-full items-center justify-center p-8 text-sm text-muted-foreground">
              {skills.length === 0
                ? "Load a skills directory to get started."
                : "Select a skill to inspect."}
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

function DetailActions({
  skill,
  busy,
  onApprove,
  onRevoke,
}: {
  skill: Skill;
  busy: boolean;
  onApprove: () => void;
  onRevoke: () => void;
}) {
  if (skill.trust === "approved") {
    return (
      <SkillActionButton
        variant="destructive"
        loading={busy}
        onClick={onRevoke}
      >
        Revoke
      </SkillActionButton>
    );
  }
  return (
    <SkillActionButton variant="primary" loading={busy} onClick={onApprove}>
      Approve
    </SkillActionButton>
  );
}

function ListSkeleton() {
  return (
    <div className="flex flex-col gap-2">
      {Array.from({ length: 4 }).map((_, i) => (
        <div
          key={i}
          className="h-20 animate-pulse rounded-lg border border-border/40 bg-muted/40"
        />
      ))}
    </div>
  );
}

function DetailSkeleton() {
  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <div className="h-6 w-1/3 animate-pulse rounded bg-muted/50" />
      <div className="h-4 w-2/3 animate-pulse rounded bg-muted/40" />
      <div className="mt-4 h-64 animate-pulse rounded bg-muted/30" />
    </div>
  );
}
