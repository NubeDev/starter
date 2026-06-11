import { useMemo } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { EditorView } from "@codemirror/view";
import {
  autocompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

import type { InsightFunctionDoc } from "@/api/types";

// The Workbench transform editor: a CodeMirror surface for the Rhai script
// with function-name autocomplete sourced from the curated catalogue. We mirror
// SqlEditor's transparent, token-driven theme so it reads identically in light
// and dark, but the language is plain text (Rhai has no shipped CM grammar) and
// the completion source is the insight functions, not a datasource schema.
const baseTheme = EditorView.theme({
  "&": {
    backgroundColor: "transparent",
    fontSize: "0.8125rem",
    color: "var(--foreground)",
  },
  ".cm-scroller": {
    backgroundColor: "transparent",
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-content": {
    backgroundColor: "transparent",
    caretColor: "var(--foreground)",
    padding: "0.5rem 0",
  },
  ".cm-gutters": {
    backgroundColor: "transparent",
    color: "var(--muted-foreground)",
    border: "none",
  },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "transparent" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--foreground)" },
  ".cm-placeholder": { color: "var(--muted-foreground)" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
    {
      backgroundColor: "color-mix(in oklab, var(--primary) 28%, transparent)",
    },
  ".cm-tooltip": {
    backgroundColor: "var(--popover, var(--card))",
    color: "var(--popover-foreground, var(--foreground))",
    border: "1px solid var(--border)",
    borderRadius: "0.5rem",
    maxWidth: "28rem",
  },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
    backgroundColor: "var(--primary)",
    color: "var(--primary-foreground)",
  },
  ".cm-tooltip-autocomplete > ul > li": {
    color: "var(--popover-foreground, var(--foreground))",
  },
  ".cm-completionDetail": {
    color: "var(--muted-foreground)",
    fontStyle: "italic",
  },
});

const highlight = HighlightStyle.define([
  { tag: [t.string, t.special(t.string)], color: "var(--chart-2)" },
  { tag: [t.number, t.bool, t.null], color: "var(--chart-4)" },
  {
    tag: [t.comment, t.lineComment, t.blockComment],
    color: "var(--muted-foreground)",
    fontStyle: "italic",
  },
]);

// Build a completion source over the catalogue: completing a bare word offers
// every function by name, with its signature as the detail and its summary as
// the info popover. Selecting one inserts its runnable example so the user gets
// a working call, not just a name.
function functionCompletions(
  functions: InsightFunctionDoc[],
): (ctx: CompletionContext) => CompletionResult | null {
  const options: Completion[] = functions.map((fn) => ({
    label: fn.name,
    type: "function",
    detail: fn.signature,
    info: fn.summary,
    apply: fn.example,
  }));
  return (ctx) => {
    const word = ctx.matchBefore(/[A-Za-z_][A-Za-z0-9_]*/);
    if (!word || (word.from === word.to && !ctx.explicit)) return null;
    return { from: word.from, options, validFor: /^[A-Za-z0-9_]*$/ };
  };
}

export function TransformEditor({
  value,
  onChange,
  functions,
  onEditorReady,
  minHeight = "12rem",
  ariaLabel = "Rhai transform script",
}: {
  value: string;
  onChange: (script: string) => void;
  functions: InsightFunctionDoc[];
  /** Receives the EditorView so the cheatsheet can insert at the cursor. */
  onEditorReady?: (view: EditorView | null) => void;
  minHeight?: string;
  ariaLabel?: string;
}) {
  const extensions = useMemo(
    () => [
      EditorView.lineWrapping,
      baseTheme,
      syntaxHighlighting(highlight),
      autocompletion({ override: [functionCompletions(functions)] }),
    ],
    [functions],
  );

  return (
    <div className="overflow-hidden rounded-md border border-border/60 bg-background/40">
      <CodeMirror
        value={value}
        onChange={onChange}
        extensions={extensions}
        theme="none"
        placeholder={'zscore("value")'}
        onCreateEditor={(view) => onEditorReady?.(view)}
        basicSetup={{
          lineNumbers: false,
          foldGutter: false,
          highlightActiveLine: false,
          highlightActiveLineGutter: false,
          autocompletion: true,
          syntaxHighlighting: false,
        }}
        style={{ minHeight }}
        aria-label={ariaLabel}
      />
    </div>
  );
}

// Insert text at the editor's current selection (used by the cheatsheet's
// "insert example" affordance). Falls back to end-of-doc if there is no view.
export function insertAtCursor(view: EditorView | null, text: string) {
  if (!view) return;
  const { from, to } = view.state.selection.main;
  view.dispatch({
    changes: { from, to, insert: text },
    selection: { anchor: from + text.length },
  });
  view.focus();
}
