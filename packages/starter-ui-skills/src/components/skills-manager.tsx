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
  description?: React.ReactNode;
  headerExtras?: React.ReactNode;
  renderBody?: (body: string) => React.ReactNode;
  revokeConfirm?: string | null;
  refreshIntervalMs?: number;
  onSelect?: (skill: SkillSummary | null) => void;
}

export function SkillsManager(props: SkillsManagerProps): React.ReactElement {
  const {
    adapter,
    className,
    title = "Skills",
    description = "Bundles the AI can load. Approve trusted ones; quarantine the rest.",
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

  const [inspectId, setInspectId] = React.useState<string | null>(null);
  const detail = useSkill({ adapter, id: inspectId });

  const counts = React.useMemo(
    () => ({
      all: skills.length,
      approved: skills.filter((s) => s.trust === "approved").length,
      quarantined: skills.filter((s) => s.trust === "quarantined").length,
    }),
    [skills],
  );

  const [busyId, setBusyId] = React.useState<string | null>(null);
  const doApprove = async (s: { id: string; bundleHash: string }) => {
    setBusyId(s.id);
    try {
      await approve(s.id, s.bundleHash);
      if (inspectId === s.id) await detail.refresh();
    } finally {
      setBusyId(null);
    }
  };
  const doRevoke = async (s: { id: string; bundleHash: string }) => {
    if (
      revokeConfirm &&
      typeof window !== "undefined" &&
      !window.confirm(revokeConfirm)
    ) {
      return;
    }
    setBusyId(s.id);
    try {
      await revoke(s.id, s.bundleHash);
      if (inspectId === s.id) await detail.refresh();
    } finally {
      setBusyId(null);
    }
  };

  const handleInspect = (s: SkillSummary) => {
    setInspectId(s.id);
    onSelect?.(s);
  };

  return (
    <div
      data-slot="skills-manager"
      className={cn(
        "h-full min-h-0 w-full overflow-y-auto bg-background text-foreground",
        className,
      )}
    >
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-7 px-6 py-7">
        <header className="flex flex-col gap-1">
          <h2 className="text-[15px] font-semibold tracking-tight">{title}</h2>
          {description ? (
            <p className="text-[11.5px] text-muted-foreground">{description}</p>
          ) : null}
        </header>

        <section className="flex flex-col gap-2">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span className="text-[11.5px] font-medium text-foreground/90">
              Skills
            </span>
            <div className="flex items-center gap-1.5">
              {headerExtras}
              <SkillActionButton
                variant="outline"
                size="xs"
                loading={loading}
                onClick={() => void reload()}
                aria-label="Reload from disk"
              >
                <svg
                  viewBox="0 0 24 24"
                  width="12"
                  height="12"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.75"
                  strokeLinecap="round"
                  strokeLinejoin="round"
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
              className="rounded-md border border-destructive/40 bg-destructive/10 px-2.5 py-1.5 text-[11px] text-destructive"
            >
              {error}
            </div>
          ) : null}

          {loading && skills.length === 0 ? (
            <GridSkeleton />
          ) : (
            <SkillList
              skills={visible}
              busyId={busyId}
              onSelect={handleInspect}
              onInspect={handleInspect}
              onApprove={(s) => void doApprove(s)}
              onRevoke={(s) => void doRevoke(s)}
              emptyMessage={
                skills.length === 0
                  ? "No skills are registered yet."
                  : "No skills match the current filter."
              }
            />
          )}
        </section>
      </div>

      <SkillInspectorModal
        open={inspectId !== null}
        onClose={() => setInspectId(null)}
        loading={detail.loading}
        skill={detail.skill}
        renderBody={renderBody}
        busy={detail.skill ? busyId === detail.skill.id : false}
        onApprove={(s) => void doApprove(s)}
        onRevoke={(s) => void doRevoke(s)}
      />
    </div>
  );
}

function SkillInspectorModal({
  open,
  onClose,
  loading,
  skill,
  renderBody,
  busy,
  onApprove,
  onRevoke,
}: {
  open: boolean;
  onClose: () => void;
  loading: boolean;
  skill: Skill | null;
  renderBody?: (body: string) => React.ReactNode;
  busy: boolean;
  onApprove: (s: Skill) => void;
  onRevoke: (s: Skill) => void;
}) {
  React.useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      window.removeEventListener("keydown", onKey);
      document.body.style.overflow = prev;
    };
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div
      role="dialog"
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
    >
      <div
        aria-hidden
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />
      <div className="relative flex max-h-[85vh] w-full max-w-2xl flex-col overflow-hidden rounded-xl border bg-background shadow-2xl">
        <button
          type="button"
          onClick={onClose}
          aria-label="Close"
          className="absolute right-2 top-2 z-10 inline-flex size-7 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          <svg
            viewBox="0 0 24 24"
            width="14"
            height="14"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden
          >
            <path d="M6 6l12 12" />
            <path d="M18 6L6 18" />
          </svg>
        </button>
        {loading && !skill ? (
          <DetailSkeleton />
        ) : skill ? (
          <SkillDetail
            skill={skill}
            renderBody={renderBody}
            actions={
              skill.trust === "approved" ? (
                <SkillActionButton
                  variant="destructive"
                  size="xs"
                  loading={busy}
                  onClick={() => onRevoke(skill)}
                >
                  Revoke
                </SkillActionButton>
              ) : (
                <SkillActionButton
                  variant="default"
                  size="xs"
                  loading={busy}
                  onClick={() => onApprove(skill)}
                >
                  Approve
                </SkillActionButton>
              )
            }
          />
        ) : (
          <div className="flex h-40 items-center justify-center text-[11.5px] text-muted-foreground">
            Skill not found.
          </div>
        )}
      </div>
    </div>
  );
}

function GridSkeleton() {
  return (
    <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
      {Array.from({ length: 4 }).map((_, i) => (
        <div
          key={i}
          className="h-[78px] animate-pulse rounded-lg border border-border/40 bg-muted/30"
        />
      ))}
    </div>
  );
}

function DetailSkeleton() {
  return (
    <div className="flex flex-col gap-3 p-4">
      <div className="h-5 w-1/3 animate-pulse rounded bg-muted/40" />
      <div className="h-3.5 w-2/3 animate-pulse rounded bg-muted/30" />
      <div className="mt-4 h-64 animate-pulse rounded bg-muted/20" />
    </div>
  );
}
