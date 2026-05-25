// `tabs` — `<Tabs>` from the UI kit; children live in `node.tabs`.
import { Tabs, TabsContent, TabsList, TabsTrigger, cn } from "@nube/starter-ui-kit";
import { RenderChildren } from "./render.js";
import { registerRenderer } from "./registry.js";

interface TabSpec {
  id?: string;
  label: string;
  children: import("@nube/starter-ui-ir").UiComponent[];
}

export function RenderTabs({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const tabs: TabSpec[] = Array.isArray(node.tabs) ? (node.tabs as TabSpec[]) : [];
  if (tabs.length === 0) return null;
  const first = tabs[0];
  const defaultId = first?.id ?? `tab-0`;
  return (
    <Tabs defaultValue={defaultId} className={cn("sdui-tabs", node.style?.className)}>
      <TabsList>
        {tabs.map((t, i) => (
          <TabsTrigger key={t.id ?? `tab-${i}`} value={t.id ?? `tab-${i}`}>
            {t.label}
          </TabsTrigger>
        ))}
      </TabsList>
      {tabs.map((t, i) => (
        <TabsContent key={t.id ?? `tab-${i}`} value={t.id ?? `tab-${i}`}>
          <RenderChildren nodes={t.children} />
        </TabsContent>
      ))}
    </Tabs>
  );
}

registerRenderer("tabs", RenderTabs);
