// `tabs` — uncontrolled `<Tabs>` from the kit; children live in
// `node.tabs`. Mirrors the web renderer one-for-one.
import type { UiComponent } from "@nube/starter-ui-ir";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@nube/starter-ui-kit-native";
import { RenderChildren, registerRenderer } from "@nube/starter-ui-sdui-react/headless";

interface TabSpec {
  id?: string;
  label: string;
  children: UiComponent[];
}

export function RenderTabs({ node }: { node: UiComponent }) {
  const tabs: TabSpec[] = Array.isArray(node.tabs) ? (node.tabs as TabSpec[]) : [];
  if (tabs.length === 0) return null;
  const defaultId = tabs[0]?.id ?? "tab-0";
  return (
    <Tabs defaultValue={defaultId}>
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
