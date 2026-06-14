// `rich_text` — editorial prose. Renders `value` as a SAFE markdown
// subset: paragraphs, headings (#..###), bold (**), italic (*), inline
// `code`, links ([text](url)), and bullet lists (- ). No raw HTML is
// ever interpreted — we build React elements from a tiny hand parser, so
// there is no `dangerouslySetInnerHTML` and thus no XSS surface. This
// avoids pulling a markdown dependency into the renderer bundle.
import { cn } from "@nube/starter-ui-kit";
import type { ReactNode } from "react";
import { registerRenderer } from "../headless/registry.js";
import { nodeStyleAttrs } from "./node-style.js";

// Inline spans: **bold**, *italic*, `code`, [text](url). Parsed in a
// single left-to-right pass so the first matching marker wins.
function parseInline(text: string, keyPrefix: string): ReactNode[] {
  const out: ReactNode[] = [];
  let rest = text;
  let k = 0;
  // Ordered: link first (longest distinctive syntax), then bold (**)
  // before italic (*) so `**x**` isn't mis-split.
  const RX =
    /(\[[^\]]+\]\([^)]+\))|(\*\*[^*]+\*\*)|(\*[^*]+\*)|(`[^`]+`)/;
  while (rest.length > 0) {
    const m = RX.exec(rest);
    if (!m || m.index === undefined) {
      out.push(rest);
      break;
    }
    if (m.index > 0) out.push(rest.slice(0, m.index));
    const tok = m[0];
    const key = `${keyPrefix}-i${k++}`;
    if (tok.startsWith("[")) {
      const inner = /\[([^\]]+)\]\(([^)]+)\)/.exec(tok)!;
      out.push(
        <a
          key={key}
          href={inner[2]}
          className="text-[color:var(--color-leaf)] underline underline-offset-2"
        >
          {inner[1]}
        </a>,
      );
    } else if (tok.startsWith("**")) {
      out.push(<strong key={key}>{tok.slice(2, -2)}</strong>);
    } else if (tok.startsWith("*")) {
      out.push(<em key={key}>{tok.slice(1, -1)}</em>);
    } else {
      out.push(
        <code
          key={key}
          className="rounded bg-[color:var(--color-muted)] px-1 py-0.5 text-[0.85em]"
        >
          {tok.slice(1, -1)}
        </code>,
      );
    }
    rest = rest.slice(m.index + tok.length);
  }
  return out;
}

// Block parser: splits on blank lines; recognises headings and bullet
// lists, everything else is a paragraph.
function parseBlocks(value: string): ReactNode[] {
  const lines = value.replace(/\r\n/g, "\n").split("\n");
  const blocks: ReactNode[] = [];
  let para: string[] = [];
  let list: string[] = [];
  let bi = 0;

  const flushPara = () => {
    if (para.length === 0) return;
    blocks.push(
      <p key={`p${bi++}`} className="leading-relaxed">
        {parseInline(para.join(" "), `p${bi}`)}
      </p>,
    );
    para = [];
  };
  const flushList = () => {
    if (list.length === 0) return;
    blocks.push(
      <ul key={`u${bi++}`} className="list-disc space-y-1 pl-5">
        {list.map((li, i) => (
          <li key={i}>{parseInline(li, `u${bi}-${i}`)}</li>
        ))}
      </ul>,
    );
    list = [];
  };

  for (const raw of lines) {
    const line = raw.trimEnd();
    if (line.trim() === "") {
      flushPara();
      flushList();
      continue;
    }
    const heading = /^(#{1,3})\s+(.*)$/.exec(line);
    if (heading) {
      flushPara();
      flushList();
      const level = heading[1]!.length;
      const Tag = `h${level + 1}` as "h2";
      const sizes = ["text-2xl", "text-xl", "text-lg"];
      blocks.push(
        <Tag
          key={`h${bi++}`}
          className={cn("font-semibold leading-tight", sizes[level - 1])}
        >
          {parseInline(heading[2] ?? "", `h${bi}`)}
        </Tag>,
      );
      continue;
    }
    const bullet = /^[-*]\s+(.*)$/.exec(line);
    if (bullet) {
      flushPara();
      list.push(bullet[1] ?? "");
      continue;
    }
    para.push(line.trim());
  }
  flushPara();
  flushList();
  return blocks;
}

export function RenderRichText({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const value = typeof node.value === "string" ? node.value : "";
  const placeholder =
    typeof node.placeholder === "string" ? node.placeholder : undefined;
  if (value.trim() === "") {
    return placeholder ? (
      <p className="sdui-rich-text text-[color:var(--color-muted-foreground)] italic">
        {placeholder}
      </p>
    ) : null;
  }
  return (
    <div
      {...nodeStyleAttrs(node.style)}
      className={cn(
        "sdui-rich-text flex flex-col gap-3 text-[color:var(--color-text)]",
        node.style?.className,
      )}
    >
      {parseBlocks(value)}
    </div>
  );
}

registerRenderer("rich_text", RenderRichText);
