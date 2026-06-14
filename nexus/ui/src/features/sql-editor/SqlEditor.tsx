import { useMemo } from "react";
import CodeMirror from "@uiw/react-codemirror";
import { sql, PostgreSQL } from "@codemirror/lang-sql";
import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import { RefreshCw } from "lucide-react";

import { useDatasourceSchema } from "@/features/sql-editor/useDatasourceSchema";
import { toSqlNamespace } from "@/features/sql-editor/schemaCompletion";

// A reusable Postgres SQL editor: CodeMirror with syntax highlighting and
// schema-aware autocomplete. Pass the selected datasource id and the editor
// learns that datasource's tables/columns (cached 5 min) and completes them
// after FROM/JOIN and as `table.column`. With no datasource it still gives
// SQL keyword completion. Shared by Explore and the panel editors so SQL
// authoring is identical everywhere.
//
// Visuals inherit the app's theme via its runtime CSS tokens, so the editor
// is correct in both light and dark without detecting the mode. The key is to
// override CodeMirror's *own* default light background on every surface it
// paints (.cm-editor / .cm-scroller / .cm-content / .cm-gutters), not just the
// outer wrapper — otherwise it shows a white strip on a dark page.
const baseTheme = EditorView.theme({
  // `&` is the `.cm-editor` root itself — this is the element that paints the
  // background, so transparent here is what removes the white box.
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
  },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
    backgroundColor: "var(--primary)",
    color: "var(--primary-foreground)",
  },
  ".cm-tooltip-autocomplete > ul > li": {
    color: "var(--popover-foreground, var(--foreground))",
  },
});

// Syntax colors drawn from the runtime palette, so highlighting re-tints with
// the brand and works in both modes (the tokens already flip on `.dark`). Kept
// to a few semantic groups — keywords, strings, numbers, identifiers, comments
// — which is all SQL needs.
const highlight = HighlightStyle.define([
  { tag: [t.keyword, t.operatorKeyword, t.modifier], color: "var(--primary)" },
  { tag: [t.string, t.special(t.string)], color: "var(--chart-2)" },
  { tag: [t.number, t.bool, t.null], color: "var(--chart-4)" },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: "var(--chart-1)" },
  { tag: [t.comment, t.lineComment, t.blockComment], color: "var(--muted-foreground)", fontStyle: "italic" },
  { tag: [t.variableName, t.propertyName], color: "var(--foreground)" },
  { tag: t.punctuation, color: "var(--muted-foreground)" },
]);

export function SqlEditor({
  value,
  onChange,
  datasourceId,
  placeholder = "select … from … where … limit 100",
  minHeight = "8rem",
  ariaLabel = "SQL query",
  id,
}: {
  value: string;
  onChange: (sql: string) => void;
  /** When set, autocomplete learns this datasource's tables and columns. */
  datasourceId?: string;
  placeholder?: string;
  minHeight?: string;
  ariaLabel?: string;
  id?: string;
}) {
  const schemaQuery = useDatasourceSchema(datasourceId);

  // Rebuild the language extension only when the learned schema changes;
  // CodeMirror reconfigures completion sources from it.
  const extensions = useMemo(() => {
    const { schema, tables } = toSqlNamespace(schemaQuery.data);
    return [
      sql({
        dialect: PostgreSQL,
        schema,
        tables: tables.map((label) => ({ label })),
        upperCaseKeywords: false,
      }),
      EditorView.lineWrapping,
      baseTheme,
      syntaxHighlighting(highlight),
    ];
  }, [schemaQuery.data]);

  return (
    <div className="relative" id={id}>
      <div className="overflow-hidden rounded-md border border-border/60 bg-background/40">
        <CodeMirror
          value={value}
          onChange={onChange}
          extensions={extensions}
          // `none` stops @uiw/react-codemirror from injecting its own light
          // theme (the default), which otherwise outranks our baseTheme and
          // repaints the editor white. Our baseTheme is then the only theme.
          theme="none"
          placeholder={placeholder}
          basicSetup={{
            lineNumbers: false,
            foldGutter: false,
            highlightActiveLine: false,
            highlightActiveLineGutter: false,
            autocompletion: true,
            // Disable basicSetup's built-in (light) highlight style so our
            // palette-driven one — added as an extension — wins instead.
            syntaxHighlighting: false,
          }}
          style={{ minHeight }}
          aria-label={ariaLabel}
        />
      </div>
      {datasourceId ? (
        <SchemaStatus
          state={
            schemaQuery.isPending
              ? "loading"
              : schemaQuery.isError
                ? "error"
                : "ready"
          }
          tableCount={schemaQuery.data?.tables.length ?? 0}
          onRefresh={() => void schemaQuery.refetch()}
          refreshing={schemaQuery.isFetching}
        />
      ) : null}
    </div>
  );
}

// Tiny line under the editor telling the user what autocomplete knows, with a
// manual refresh for when the database shape changed mid-session.
function SchemaStatus({
  state,
  tableCount,
  onRefresh,
  refreshing,
}: {
  state: "loading" | "error" | "ready";
  tableCount: number;
  onRefresh: () => void;
  refreshing: boolean;
}) {
  return (
    <div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
      {state === "loading" ? (
        <span>Learning schema…</span>
      ) : state === "error" ? (
        <span>Schema unavailable — keyword completion only</span>
      ) : (
        <span>
          {tableCount} {tableCount === 1 ? "table" : "tables"} for autocomplete
        </span>
      )}
      <button
        type="button"
        onClick={onRefresh}
        disabled={refreshing}
        className="inline-flex items-center gap-1 rounded-sm hover:text-foreground disabled:opacity-50"
        aria-label="Refresh schema"
      >
        <RefreshCw className={`size-3 ${refreshing ? "animate-spin" : ""}`} />
        Refresh
      </button>
    </div>
  );
}
