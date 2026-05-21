/**
 * `rich_text` — markdown-aware editor. The IR adapter delegates
 * the actual editor surface to the host's tiptap / milkdown
 * integration; this wrapper is the IR shape mapping (per SCOPE
 * "Size targets": only the wrapper counts toward the LoC budget,
 * not the underlying editor library).
 *
 * In v1 the wrapper renders a `<textarea>` fallback so the IR is
 * exerciseable without pulling a heavy editor; hosts replace this
 * spec via `registerCustomRenderer` when they want a real WYSIWYG.
 * The IR shape (value / placeholder) stays compatible.
 */
import { useState } from "react";
import { Textarea } from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import type { UiComponent } from "../types.js";

export interface RichTextNode extends UiComponent {
  type: "rich_text";
  value?: string;
  placeholder?: string;
}

export const richTextSpec: ComponentSpec<RichTextNode> = {
  kind: "rich_text" as never,
  Component: ({ node }) => {
    const [draft, setDraft] = useState(node.value ?? "");
    return (
      <Textarea
        value={draft}
        placeholder={node.placeholder}
        onChange={(e) => setDraft(e.target.value)}
        className={node.style?.className}
        rows={6}
      />
    );
  },
};
