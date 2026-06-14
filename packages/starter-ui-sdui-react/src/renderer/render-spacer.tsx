// `spacer` — pure vertical breathing room between bands. A `size` token
// drives the height; no content. Lets authors separate sections without
// abusing empty containers.
import { registerRenderer } from "../headless/registry.js";

const SIZE_REM: Record<string, string> = {
  xs: "0.5rem",
  sm: "1rem",
  md: "2rem",
  lg: "3rem",
  xl: "4.5rem",
  "2xl": "6rem",
};

export function RenderSpacer({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const height = SIZE_REM[node.size as string] ?? SIZE_REM.md;
  return <div className="sdui-spacer" aria-hidden style={{ height }} />;
}

registerRenderer("spacer", RenderSpacer);
