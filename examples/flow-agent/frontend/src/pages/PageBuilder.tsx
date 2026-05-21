// `/pages/new` and `/pages/:id/edit` — the headline screen.
//
// Composes `useBuilder` + `<BuilderTranscript>` + `<AiBuilderCanvas>`
// directly (does NOT wrap the opinionated `<AiBuilder>`) so this host
// owns the current `tree` and can wire its own save action. The
// scripted fixture adapter from `lib/builder-fixture.ts` streams the
// `BuilderEvent`s; saving writes `{ id, name, tree, createdAt }` into
// `localStorage` via `pages-store.savePage` and navigates to the new
// `/pages/:id`.

import { useCallback, useEffect, useMemo, useState } from "react"
import { useNavigate, useParams } from "react-router-dom"
import { IconDeviceFloppy, IconArrowLeft } from "@tabler/icons-react"

import {
  AiBuilderCanvas,
  BuilderTranscript,
  createHttpBuilderAdapter,
  useBuilder,
} from "@nube/starter-ui-ai-builder"
import type { UiComponentTree } from "@nube/starter-sdui-react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { createFlowAgentBuilderFixture } from "@/lib/builder-fixture"
import { getPage, savePage } from "@/lib/pages-store"
import {
  asTree,
  builderSurfaceKey,
  clearSessionId,
  createSession,
  getArtifactVersion,
  getLatestArtifact,
  listArtifactVersions,
  loadSessionId,
  saveSessionId,
  type ArtifactVersionMeta,
} from "@/lib/sessions"

/** URL of the live builder SSE route (proxied by Vite to the
 *  backend in dev; same-origin in prod). Kept relative so the
 *  surface works under any host. */
const BUILDER_STREAM_URL = "/api/builder/stream"

export interface PageBuilderProps {
  /** When set, seeds the builder for an edit-in-place flow. */
  initialTree?: UiComponentTree | null
  /** When editing, the existing record id so the save round-trips. */
  pageId?: string
  /** When editing, the existing record name to pre-fill. */
  initialName?: string
}

export function PageBuilder(props: PageBuilderProps = {}) {
  // Adapter selection (PAGE-BUILDER-LIVE-FRONTEND.md §4.4):
  //
  //   ?fixture=1 → deterministic scripted fixture (offline e2e).
  //   ?demo=1    → real HTTP adapter, but silently fall back to
  //                the fixture on HTTP 503 (live demo where the
  //                backend may be down).
  //   default    → real HTTP adapter; 503 surfaces inline as an
  //                error frame in the transcript.
  //
  // Use `.get(…) === "1"` rather than `.has(…)` — `?fixture=0` must
  // NOT enable the fixture.
  const { useFixture, demoMode } = useMemo(() => {
    if (typeof window === "undefined") {
      return { useFixture: false, demoMode: false }
    }
    const params = new URLSearchParams(window.location.search)
    return {
      useFixture: params.get("fixture") === "1",
      demoMode: params.get("demo") === "1",
    }
  }, [])

  const adapter = useMemo(() => {
    if (useFixture) return createFlowAgentBuilderFixture()
    return createHttpBuilderAdapter({
      url: BUILDER_STREAM_URL,
      onUnavailable: demoMode
        ? () => createFlowAgentBuilderFixture()
        : undefined,
    })
  }, [useFixture, demoMode])

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

  // MEMORY.md Phase M-D — agent-session persistence.
  //
  //   - `sessionId` is stable per surface (new draft vs. existing
  //     page id); persisted in `localStorage` via `lib/sessions`.
  //   - On mount we either create a fresh session (`POST
  //     /api/sessions`) or rehydrate the canvas from the existing
  //     session's `tree` artifact (`GET .../artifacts/tree`) —
  //     **zero model tokens** for page reload (MEMORY.md M4).
  //   - The version count is surfaced as a small badge so the user
  //     can tell undo is available.
  const surface = useMemo(
    () => builderSurfaceKey(editingId),
    [editingId],
  )
  const [sessionId, setSessionId] = useState<string | null>(() =>
    loadSessionId(surface),
  )
  // Full version metadata (newest first), so the picker can render
  // labels without an additional round-trip per item. `versions[0]`
  // is the latest snapshot.
  const [versions, setVersions] = useState<ArtifactVersionMeta[]>([])
  const versionCount = versions.length
  const [sessionWarning, setSessionWarning] = useState<string | null>(null)

  const builder = useBuilder({
    adapter,
    initialTree: seeded.tree,
    onSessionArtifact: ({ key }) => {
      // Persist failures (`session_error`) cleared on success.
      setSessionWarning(null)
      // Refresh the version count so the badge reflects the new
      // snapshot. Doing this in a fire-and-forget effect would
      // also work; inline is simpler and the endpoint is cheap.
      if (key === "tree" && sessionId) {
        void listArtifactVersions(sessionId, "tree")
          .then((vs) => setVersions(vs))
          .catch(() => {
            /* non-fatal */
          })
      }
    },
    onSessionError: (msg) => {
      // Backend kept the response; only persistence failed. Keep
      // the canvas usable and tell the user we're degraded.
      setSessionWarning(msg)
    },
  })

  // Mount-time hydration: create-or-resume the session and pull
  // the latest `tree` artifact into the canvas. Uses an AbortController
  // so route changes during the round-trip don't write into a
  // stale builder.
  useEffect(() => {
    const ctrl = new AbortController()
    let cancelled = false

    void (async () => {
      try {
        let id = sessionId
        if (id) {
          // Resume: try to hydrate. If the server has dropped this
          // session (404 on artifact), fall through to creating a
          // new one rather than wedging the surface.
          try {
            const art = await getLatestArtifact(id, "tree", ctrl.signal)
            if (cancelled) return
            const tree = asTree(art)
            if (tree) {
              builder.setTree(tree)
              // Refresh version list for the badge / picker.
              try {
                const vs = await listArtifactVersions(id, "tree", ctrl.signal)
                if (!cancelled) setVersions(vs)
              } catch {
                /* non-fatal */
              }
              return
            }
          } catch (e) {
            // 4xx/5xx (likely 400 "invalid session_id" or backend
            // restart that lost in-memory state): forget the id
            // and start fresh.
            if (cancelled) return
            if (ctrl.signal.aborted) return
            clearSessionId(surface)
            id = null
            setSessionId(null)
            // eslint-disable-next-line no-console
            console.warn(`[page-builder] dropping stale session: ${(e as Error).message}`)
          }
        }
        if (!id) {
          const created = await createSession(ctrl.signal)
          if (cancelled) return
          saveSessionId(surface, created)
          setSessionId(created)
        }
      } catch (e) {
        if (cancelled || ctrl.signal.aborted) return
        // Backend unreachable / 503 — stay stateless (MEMORY.md M13).
        // eslint-disable-next-line no-console
        console.warn(
          `[page-builder] session bootstrap failed, continuing stateless: ${(e as Error).message}`,
        )
        setSessionWarning("persistence unavailable — staying stateless")
      }
    })()

    return () => {
      cancelled = true
      ctrl.abort()
    }
    // We deliberately depend on `surface` only; `sessionId` changes
    // are driven from inside this effect, and `builder.setTree` is
    // stable across the hook's lifetime.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [surface])

  // Wire client-side abort so retyping mid-stream doesn't race the
  // canvas with the previous turn (PAGE-BUILDER-LIVE-FRONTEND.md
  // §4.4). The transcript's Stop button already calls `cancel`;
  // here we also cancel on submit-while-busy and on unmount.
  const handleSend = useCallback(
    (text: string) => {
      if (builder.phase !== "idle" && builder.phase !== "done") {
        builder.cancel()
      }
      // MEMORY.md Phase M-D — pass `sessionId` (when we have one)
      // and ask the backend to seed the prompt with the latest
      // `tree` artifact. Omitted sessionId stays ephemeral (M13).
      void builder.send({
        text,
        sessionId: sessionId ?? undefined,
        includeArtifact: sessionId ? "tree" : undefined,
      })
    },
    [builder, sessionId],
  )

  useEffect(() => {
    return () => {
      builder.cancel()
    }
    // We intentionally re-bind on `builder.cancel` identity changes
    // (stable per mount); see useBuilder hook.
  }, [builder.cancel])

  // MEMORY.md Phase M-D — version picker (undo). Loads the chosen
  // historical artifact and replaces the canvas in-place. Pure
  // storage read; the model is NOT invoked (§5 step 6).
  const handlePickVersion = useCallback(
    async (version: number) => {
      if (!sessionId) return
      try {
        const art = await getArtifactVersion(sessionId, "tree", version)
        const tree = asTree(art)
        if (tree) builder.setTree(tree)
      } catch (e) {
        // eslint-disable-next-line no-console
        console.warn(`[page-builder] load v${version} failed: ${(e as Error).message}`)
      }
    },
    [builder, sessionId],
  )

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
          {versionCount > 0 ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="outline"
                  size="sm"
                  className="h-6 gap-1 border-border/60 bg-muted/40 px-2 text-[10px]"
                  title={`Session ${sessionId ?? "(none)"} — ${versionCount} saved version(s)`}
                  aria-label="Version history"
                >
                  v{versionCount}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="min-w-[14rem]">
                <DropdownMenuLabel>Version history</DropdownMenuLabel>
                <DropdownMenuSeparator />
                {versions.map((v, idx) => (
                  <DropdownMenuItem
                    key={v.version}
                    onSelect={() => void handlePickVersion(v.version)}
                  >
                    <span className="font-mono text-xs">v{v.version}</span>
                    <span className="ml-2 truncate text-muted-foreground">
                      {new Date(v.updated_at).toLocaleString()}
                    </span>
                    {idx === 0 ? (
                      <span className="ml-auto text-[10px] uppercase text-muted-foreground">
                        latest
                      </span>
                    ) : null}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          ) : null}
          {sessionWarning ? (
            <Badge
              variant="outline"
              className="border-destructive/40 bg-destructive/10 text-destructive"
              title={sessionWarning}
            >
              offline
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
            onSend={(text) => handleSend(text)}
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
