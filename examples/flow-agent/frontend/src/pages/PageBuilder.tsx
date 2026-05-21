// `/pages/new` and `/pages/:id/edit` — the headline screen.
//
// Composes `useBuilder` + `<BuilderTranscript>` + `<AiBuilderCanvas>`
// directly (does NOT wrap the opinionated `<AiBuilder>`) so this host
// owns the current `tree` and can wire its own save action. The
// scripted fixture adapter from `lib/builder-fixture.ts` streams the
// `BuilderEvent`s; saving writes `{ id, name, tree, createdAt }` into
// `localStorage` via `pages-store.savePage` and navigates to the new
// `/pages/:id`.

import { useMemo, useState } from "react"
import { useNavigate, useParams } from "react-router-dom"
import { IconDeviceFloppy, IconArrowLeft } from "@tabler/icons-react"

import {
  AiBuilderCanvas,
  BuilderTranscript,
  useBuilder,
} from "@nube/starter-ui-ai-builder"
import type { UiComponentTree } from "@nube/starter-sdui-react"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { createFlowAgentBuilderFixture } from "@/lib/builder-fixture"
import { getPage, savePage } from "@/lib/pages-store"

export interface PageBuilderProps {
  /** When set, seeds the builder for an edit-in-place flow. */
  initialTree?: UiComponentTree | null
  /** When editing, the existing record id so the save round-trips. */
  pageId?: string
  /** When editing, the existing record name to pre-fill. */
  initialName?: string
}

export function PageBuilder(props: PageBuilderProps = {}) {
  // The fixture adapter is deterministic but holds a small amount of
  // internal state (script cursor per turn); memoise it for the
  // lifetime of this mount so retries reset cleanly.
  const adapter = useMemo(() => createFlowAgentBuilderFixture(), [])
  const navigate = useNavigate()
  const params = useParams<{ id?: string }>()

  // Resolve "edit-in-place" mode when invoked from `/pages/:id/edit`.
  // Props win so a parent can always seed explicitly; otherwise we
  // hydrate from `localStorage` on first mount.
  const editingId = props.pageId ?? params.id
  const seeded = useMemo(() => {
    if (props.initialTree !== undefined) {
      return { tree: props.initialTree, name: props.initialName ?? "" }
    }
    if (editingId) {
      const rec = getPage(editingId)
      if (rec) return { tree: rec.tree, name: rec.name }
    }
    return { tree: null as UiComponentTree | null, name: "" }
  }, [editingId, props.initialName, props.initialTree])

  const [name, setName] = useState(seeded.name)
  const builder = useBuilder({
    adapter,
    initialTree: seeded.tree,
  })

  const canSave = Boolean(builder.tree) && name.trim().length > 0

  function handleSave() {
    if (!builder.tree || !name.trim()) return
    const rec = savePage({
      id: editingId,
      name: name.trim(),
      tree: builder.tree,
    })
    navigate(`/pages/${rec.id}`)
  }

  return (
    <div
      data-slot="page-builder"
      className="flex h-[calc(100dvh-3.5rem)] min-h-0 w-full flex-col bg-gradient-to-b from-background to-muted/30"
    >
      <header className="flex shrink-0 items-center gap-3 border-b border-border/60 bg-background/70 px-4 py-2.5 backdrop-blur">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => navigate("/pages")}
          aria-label="Back to pages"
        >
          <IconArrowLeft className="size-4" />
        </Button>
        <div className="text-sm font-semibold">
          {editingId ? "Edit page" : "Page Builder"}
        </div>
        <Input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Page name"
          className="ml-2 h-8 max-w-xs"
        />
        <div className="ml-auto flex items-center gap-2 text-xs text-muted-foreground">
          <PhaseBadge phase={builder.phase} />
          {builder.bufferedPatches > 0 ? (
            <Badge
              variant="outline"
              className="border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300"
            >
              {builder.bufferedPatches} buffered
            </Badge>
          ) : null}
          <Button
            size="sm"
            onClick={handleSave}
            disabled={!canSave}
            aria-label="Save page"
          >
            <IconDeviceFloppy className="size-4" />
            Save
          </Button>
        </div>
      </header>

      <div className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[minmax(20rem,28rem)_1fr]">
        <div className="min-h-0 border-border/40 md:border-r">
          <BuilderTranscript
            entries={builder.transcript}
            phase={builder.phase}
            placeholder="Describe the UI… try: sales · dashboard · onboard · report"
            onSend={(text) => void builder.send(text)}
            onCancel={builder.cancel}
            onRetry={() => void builder.retry()}
            canRetry={builder.transcript.some((e) => e.kind === "user")}
            className="bg-transparent"
          />
        </div>
        <AiBuilderCanvas
          tree={builder.tree}
          bufferedPatches={builder.bufferedPatches}
        />
      </div>
    </div>
  )
}

function PhaseBadge({
  phase,
}: {
  phase: ReturnType<typeof useBuilder>["phase"]
}) {
  if (phase === "idle") return null
  const tones: Record<string, string> = {
    thinking: "bg-primary/15 text-primary",
    writing: "bg-primary/15 text-primary animate-pulse",
    done: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
    error: "bg-destructive/15 text-destructive",
    cancelled: "bg-muted text-muted-foreground",
  }
  return (
    <span
      className={`rounded-full px-2 py-0.5 text-[10px] uppercase tracking-wide ${tones[phase] ?? "bg-muted text-muted-foreground"}`}
    >
      {phase}
    </span>
  )
}
