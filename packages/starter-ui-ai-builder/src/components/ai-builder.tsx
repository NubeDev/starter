import * as React from "react";
import type { UiComponentTree } from "@nube/starter-sdui-react";
import { cn } from "../lib/utils.js";
import { useBuilder } from "../hooks/use-builder.js";
import type {
  BuilderAdapter,
  BuilderMode,
  ShellPatch,
  TokenPatch,
} from "../types/index.js";
import { AiBuilderCanvas } from "./ai-builder-canvas.js";
import { BuilderTranscript } from "./builder-transcript.js";

export interface AiBuilderProps {
  adapter: BuilderAdapter;
  initialTree?: UiComponentTree | null;
  /** Initial conversation lane. Defaults to `"build"`. */
  defaultMode?: BuilderMode;
  /** Hide the Build/Ask toggle. The conversation stays locked on
   *  `defaultMode`. Use this for surfaces that don't want to expose
   *  the Ask lane (e.g. embedded "create page" wizards). */
  hideModeToggle?: boolean;
  title?: React.ReactNode;
  headerExtras?: React.ReactNode;
  placeholder?: string;
  allowAttachments?: boolean;
  /** Sinks for the theme slice; ignored by the page-builder slice. */
  onTokenPatch?: (patch: TokenPatch) => void;
  onShellPatch?: (patch: ShellPatch) => void;
  /** Canvas-only mode (no transcript pane). Useful when the prompt
   *  surface lives elsewhere in the host UI. */
  canvasOnly?: boolean;
  /** Transcript-only mode (no canvas). Pair with your own renderer. */
  transcriptOnly?: boolean;
  className?: string;
  /** Default: `1fr_1fr`. Tweak the split ratio. */
  splitClassName?: string;
}

// Opinionated end-to-end ai-builder surface: chat transcript on the
// left, live SDUI canvas on the right. For full control compose the
// pieces directly: `useBuilder` + `<BuilderTranscript>` +
// `<AiBuilderCanvas>`.
export function AiBuilder(props: AiBuilderProps): React.ReactElement {
  const {
    adapter,
    initialTree,
    defaultMode,
    hideModeToggle,
    title,
    headerExtras,
    placeholder,
    allowAttachments,
    onTokenPatch,
    onShellPatch,
    canvasOnly,
    transcriptOnly,
    className,
    splitClassName = "md:grid-cols-[minmax(20rem,28rem)_1fr]",
  } = props;

  const builder = useBuilder({
    adapter,
    initialTree,
    defaultMode,
    onTokenPatch,
    onShellPatch,
  });

  const showTranscript = !canvasOnly;
  const showCanvas = !transcriptOnly;

  return (
    <div
      data-slot="ai-builder"
      className={cn(
        "flex h-full min-h-0 w-full flex-col bg-gradient-to-b from-background to-muted/30 text-foreground",
        className,
      )}
    >
      {(title || headerExtras) && (
        <header className="flex shrink-0 items-center gap-2 border-b border-border/60 bg-background/70 px-4 py-2.5 backdrop-blur">
          {title ? <div className="text-sm font-semibold">{title}</div> : null}
          <div className="ml-auto flex items-center gap-2 text-xs text-muted-foreground">
            <PhaseBadge phase={builder.phase} />
            {builder.bufferedPatches > 0 ? (
              <span className="rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-amber-700 dark:text-amber-300">
                {builder.bufferedPatches} buffered
              </span>
            ) : null}
            {headerExtras}
          </div>
        </header>
      )}

      <div
        className={cn(
          "grid min-h-0 flex-1 grid-cols-1 gap-0",
          showTranscript && showCanvas && splitClassName,
        )}
      >
        {showTranscript && (
          <div className="min-h-0 border-border/40 md:border-r">
            <BuilderTranscript
              entries={builder.transcript}
              phase={builder.phase}
              mode={hideModeToggle ? undefined : builder.mode}
              onModeChange={hideModeToggle ? undefined : builder.setMode}
              placeholder={placeholder}
              allowAttachments={allowAttachments}
              onSend={(text) => void builder.send(text)}
              onCancel={builder.cancel}
              onRetry={() => void builder.retry()}
              canRetry={builder.transcript.some((e) => e.kind === "user")}
              className="bg-transparent"
            />
          </div>
        )}
        {showCanvas && (
          <AiBuilderCanvas
            tree={builder.tree}
            bufferedPatches={builder.bufferedPatches}
          />
        )}
      </div>
    </div>
  );
}

function PhaseBadge({ phase }: { phase: ReturnType<typeof useBuilder>["phase"] }) {
  if (phase === "idle") return null;
  const styles: Record<string, string> = {
    thinking: "bg-primary/15 text-primary",
    writing: "bg-primary/15 text-primary animate-pulse",
    done: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300",
    error: "bg-destructive/15 text-destructive",
    cancelled: "bg-muted text-muted-foreground",
  };
  return (
    <span
      className={cn(
        "rounded-full px-2 py-0.5 text-[10px] uppercase tracking-wide",
        styles[phase] ?? "bg-muted text-muted-foreground",
      )}
    >
      {phase}
    </span>
  );
}
