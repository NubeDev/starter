// `custom` — delegates to a host-supplied renderer keyed by
// `node.renderer_id`. If the host hasn't registered a renderer for
// this id, fall back to the same "dangling" placeholder the central
// walker uses for unknown variants.
import { registerRenderer } from "./registry.js";
import { useSduiContext } from "../provider/sdui-provider.js";

export function RenderCustom({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const id = typeof node.renderer_id === "string" ? node.renderer_id : "";
  const { customRenderers } = useSduiContext();
  const Impl = customRenderers?.[id];
  if (Impl) return <Impl node={node} />;
  return (
    <div
      data-sdui-custom-missing={id}
      style={{
        padding: 8,
        border: "1px dashed #c93",
        color: "#c93",
        fontSize: 12,
      }}
    >
      Missing custom renderer: <code>{id || "(unset)"}</code>
    </div>
  );
}

registerRenderer("custom", RenderCustom);
