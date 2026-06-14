// Curated "Style" field for the Puck editor.
//
// The IR `NodeStyle` carries a mix of *visual decoration* tokens
// (surface, radius, spacing, background, gradient, shadow, typography)
// and *non-visual* machinery (show_when predicates, flex/min_width
// layout constraints, binding-ish bits). Auto-generating a field for the
// whole object would dump that machinery into the editor as raw text
// inputs — confusing, and not what an author reaching for "make this look
// nicer" wants.
//
// Instead we expose a hand-curated subset: just the closed-set decoration
// tokens, each as a dropdown. This is the editor half of the "extend
// tokens only" decision — authors pick from theme tokens, never raw hex,
// so light/dark stays consistent. The renderer half lives in
// `@nube/starter-ui-sdui-react` (`renderer/node-style.ts` +
// `styles/node-style.css`).
//
// Kept as a standalone `object` PuckFieldStub so `build-puck-config.ts`
// can splice it in wherever a variant declares a `style` property,
// replacing the old wholesale skip.

import type { PuckFieldStub } from "../puck-types.js";

function select(
  values: readonly string[],
): Extract<PuckFieldStub, { type: "select" }> {
  return {
    type: "select",
    options: [
      { label: "—", value: "" },
      ...values.map((v) => ({ label: v, value: v })),
    ],
  };
}

/**
 * The curated visual-style field. Spliced in for every variant that has a
 * `style` property in the IR schema. Only decoration tokens are exposed;
 * layout/visibility machinery (show_when, flex, min_width, …) is
 * intentionally omitted from author-facing editing in this pass.
 */
export const STYLE_FIELD: Extract<PuckFieldStub, { type: "object" }> = {
  type: "object",
  objectFields: {
    background: select([
      "none",
      "surface",
      "muted",
      "subtle",
      "leaf",
      "aqua",
      "sun",
      "sky",
      "warn",
      "ink",
    ]),
    gradient: select(["none", "leaf", "aqua", "sun", "sky", "dusk", "ink"]),
    surface: select(["default", "raised", "subtle", "transparent"]),
    radius: select(["none", "sm", "md", "lg", "xl", "full"]),
    spacing: select(["none", "xs", "sm", "md", "lg", "xl", "2xl"]),
    shadow: select(["none", "sm", "md", "lg", "xl", "glow"]),
    intent: select(["info", "success", "warning", "danger", "muted"]),
    text_align: select(["start", "center", "end"]),
    font_size: select([
      "xs",
      "sm",
      "md",
      "lg",
      "xl",
      "2xl",
      "3xl",
      "4xl",
    ]),
    font_weight: select(["normal", "medium", "semibold", "bold"]),
  },
};
