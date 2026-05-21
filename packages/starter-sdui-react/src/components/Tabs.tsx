/**
 * `tabs` — shadcn Tabs primitive. The IR carries `tabs: [{ id, label,
 * children: [] }]` rather than `children: []`; `RendererList`
 * projects each tab's children independently. The active tab is
 * page-state-bound via the optional `active_key` field; when
 * absent, the first tab is the initial value.
 */
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import { RendererList } from "../Renderer.js";
import { useSdui } from "../context.js";
import type { UiComponent } from "../types.js";

export interface TabsNode extends UiComponent {
  type: "tabs";
  tabs: { id?: string; label: string; children: UiComponent[] }[];
  /** Optional page-state path the active tab id should write to. */
  state_key?: string;
}

export const tabsSpec: ComponentSpec<TabsNode> = {
  kind: "tabs",
  Component: ({ node }) => {
    const { pageState, setPageState } = useSdui();
    // Be tolerant: the model occasionally emits `children: []` (the
    // layout shape it uses everywhere else) instead of the required
    // `tabs: [{ label, children }]` array. Coerce both shapes into
    // one structure, and fall back to a single "Tab 1" if neither is
    // usable, so a malformed node degrades to a flat render instead
    // of crashing the whole canvas.
    const rawTabs = Array.isArray(node.tabs) ? node.tabs : null;
    const tabs: { id?: string; label: string; children: UiComponent[] }[] =
      rawTabs && rawTabs.length > 0
        ? rawTabs
        : Array.isArray((node as unknown as { children?: UiComponent[] }).children)
          ? [
              {
                label: "Tab 1",
                children:
                  (node as unknown as { children: UiComponent[] }).children ?? [],
              },
            ]
          : [{ label: "Tab 1", children: [] }];
    const ids = tabs.map((t, i) => t.id ?? `${node.id ?? "tabs"}-${i}`);
    const stateKey = node.state_key;
    const stateValue = stateKey
      ? (pageState[stateKey] as string | undefined)
      : undefined;
    const value = stateValue ?? ids[0] ?? "0";
    const onValueChange = (v: string) => {
      if (stateKey) setPageState({ [stateKey]: v });
    };
    return (
      <Tabs value={value} onValueChange={onValueChange} className="w-full">
        <TabsList>
          {tabs.map((t, i) => (
            <TabsTrigger key={ids[i]} value={ids[i]!}>
              {t.label}
            </TabsTrigger>
          ))}
        </TabsList>
        {tabs.map((t, i) => (
          <TabsContent key={ids[i]} value={ids[i]!}>
            <RendererList nodes={t.children ?? []} parentId={node.id} parentType="tabs" />
          </TabsContent>
        ))}
      </Tabs>
    );
  },
};
