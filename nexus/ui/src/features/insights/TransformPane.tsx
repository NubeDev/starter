import { useState } from "react";
import type { EditorView } from "@codemirror/view";
import { BookOpen, Code2 } from "lucide-react";

import type { InsightFunctionDoc } from "@/api/types";
import {
  TransformEditor,
  insertAtCursor,
} from "@/features/insights/TransformEditor";
import { FunctionCheatsheet } from "@/features/insights/FunctionCheatsheet";

// Center pane: author the Rhai transform with catalogue-fed autocomplete, an
// optional params JSON box, and a clickable cheatsheet of the curated function
// surface. The editor and cheatsheet share one EditorView so the cheatsheet can
// insert an example at the cursor.
export function TransformPane({
  script,
  onScriptChange,
  functions,
  functionsLoading,
  paramsText,
  onParamsChange,
  paramsError,
}: {
  script: string;
  onScriptChange: (script: string) => void;
  functions: InsightFunctionDoc[];
  functionsLoading: boolean;
  paramsText: string;
  onParamsChange: (text: string) => void;
  paramsError: string | null;
}) {
  const [view, setView] = useState<EditorView | null>(null);

  return (
    <div className="glass flex h-full min-h-0 flex-col gap-3 rounded-xl p-3">
      <div className="flex items-center gap-2">
        <Code2 className="size-4 text-muted-foreground" />
        <h3 className="text-sm font-semibold">Transform</h3>
        <span className="ms-auto text-xs text-muted-foreground">
          Rhai · ⌃Space for functions
        </span>
      </div>

      <TransformEditor
        value={script}
        onChange={onScriptChange}
        functions={functions}
        onEditorReady={setView}
        minHeight="9rem"
      />

      <details className="rounded-md border border-border/60">
        <summary className="cursor-pointer select-none px-2 py-1.5 text-xs font-medium text-muted-foreground">
          Params (JSON) — bound as <code className="font-mono">params</code>
        </summary>
        <div className="border-t border-border/60 p-2">
          <textarea
            value={paramsText}
            onChange={(e) => onParamsChange(e.target.value)}
            spellCheck={false}
            placeholder='{ "threshold": 3.0 }'
            className="scrollbar-thin h-16 w-full resize-y rounded-sm border border-border/60 bg-background/40 p-2 font-mono text-xs text-foreground outline-none focus:border-primary/60"
            aria-label="Transform params JSON"
          />
          {paramsError ? (
            <p role="alert" className="mt-1 text-xs text-destructive">
              {paramsError}
            </p>
          ) : null}
        </div>
      </details>

      <div className="flex min-h-0 flex-1 flex-col gap-1.5">
        <div className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
          <BookOpen className="size-3.5" />
          Functions
          <span className="text-muted-foreground/70">— click to insert</span>
        </div>
        <div className="min-h-0 flex-1">
          {functionsLoading ? (
            <p className="px-1 text-xs text-muted-foreground">
              Loading functions…
            </p>
          ) : (
            <FunctionCheatsheet
              functions={functions}
              onInsert={(example) => insertAtCursor(view, example)}
            />
          )}
        </div>
      </div>
    </div>
  );
}
