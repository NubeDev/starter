/**
 * `diff` — old / new text diff. The real diff surface delegates to
 * monaco-diff (per SCOPE "Size targets"); this wrapper is the IR
 * adapter that maps `old_text` / `new_text` / `language` /
 * `annotations` / `line_action` into a renderable shape.
 *
 * Only the wrapper counts toward the LoC budget. Hosts that want
 * the full monaco surface register a custom renderer with the same
 * `diff` kind.
 *
 * Per SCOPE "Diff interactions": `annotations` carry side-channel
 * marks (comments, lint markers) keyed to line numbers; clicking
 * an annotated line fires `line_action` with `$line` substituted
 * from the click context.
 */
import type { ComponentSpec } from "../registry/types.js";
import type { UiComponent } from "../types.js";
import { useSdui } from "../context.js";

export interface DiffAnnotation {
  line: number;
  side: "old" | "new";
  severity?: "info" | "warning" | "error";
  message?: string;
}
export interface DiffNode extends UiComponent {
  type: "diff";
  old_text: string;
  new_text: string;
  language?: string;
  annotations?: DiffAnnotation[];
  line_action?: { handler: string; args?: Record<string, unknown> };
}

function substituteLine(
  args: Record<string, unknown> | undefined,
  line: number,
  side: "old" | "new",
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(args ?? {})) {
    if (v === "$line") out[k] = line;
    else if (v === "$side") out[k] = side;
    else out[k] = v;
  }
  return out;
}

export const diffSpec: ComponentSpec<DiffNode> = {
  kind: "diff" as never,
  Component: ({ node }) => {
    const { dispatchAction } = useSdui();
    const oldLines = node.old_text.split("\n");
    const newLines = node.new_text.split("\n");
    const ann = node.annotations ?? [];

    const annotationFor = (line: number, side: "old" | "new") =>
      ann.find((a) => a.line === line && a.side === side);

    const onLineClick = (line: number, side: "old" | "new") => {
      if (!node.line_action) return;
      void dispatchAction(
        node.line_action.handler,
        substituteLine(node.line_action.args, line, side),
      );
    };

    const renderColumn = (lines: string[], side: "old" | "new") => (
      <pre className="overflow-x-auto rounded border bg-muted/50 p-2 text-xs">
        {lines.map((ln, i) => {
          const a = annotationFor(i + 1, side);
          const sevClass =
            a?.severity === "error"
              ? "border-l-2 border-destructive pl-1"
              : a?.severity === "warning"
              ? "border-l-2 border-amber-500 pl-1"
              : a
              ? "border-l-2 border-muted-foreground pl-1"
              : "pl-1";
          return (
            <button
              key={i}
              type="button"
              className={`block w-full text-left ${sevClass} hover:bg-accent`}
              onClick={() => onLineClick(i + 1, side)}
              title={a?.message}
            >
              <span className="mr-2 inline-block w-6 text-right text-muted-foreground">
                {i + 1}
              </span>
              <code>{ln || " "}</code>
            </button>
          );
        })}
      </pre>
    );

    return (
      <div
        className={`grid grid-cols-2 gap-2 ${node.style?.className ?? ""}`}
        data-language={node.language}
      >
        {renderColumn(oldLines, "old")}
        {renderColumn(newLines, "new")}
      </div>
    );
  },
};
