import { describe, expect, it } from "vitest";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "./tabs.js";
import { mount } from "./test-utils.js";

describe("<Tabs>", () => {
  it("marks the active trigger as selected and hides inactive panel", () => {
    const root = mount(
      <Tabs defaultValue="a">
        <TabsList>
          <TabsTrigger value="a">A</TabsTrigger>
          <TabsTrigger value="b">B</TabsTrigger>
        </TabsList>
        <TabsContent value="a">content-a</TabsContent>
        <TabsContent value="b">content-b</TabsContent>
      </Tabs>,
    );
    const triggers = root.querySelectorAll('[accessibilityrole="tab"]');
    expect(triggers).toHaveLength(2);
    expect(root.textContent).toContain("content-a");
    expect(root.textContent).not.toContain("content-b");
    expect(root.querySelector('[accessibilityrole="tablist"]')).not.toBeNull();
    expect(root.querySelector('[accessibilityrole="tabpanel"]')).not.toBeNull();
    expect(root).toMatchSnapshot();
  });
});
