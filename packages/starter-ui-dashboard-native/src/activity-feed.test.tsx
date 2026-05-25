import * as React from "react";
import { describe, expect, it } from "vitest";
import { ActivityFeed, type ActivityItem } from "./activity-feed.js";
import { allByKit, byKit, mount } from "./test-utils.js";

// Local icon component so the test doesn't depend on lucide.
const FakeIcon: ActivityItem["icon"] = (({
  size,
  color,
}: { size?: number; color?: string }) =>
  React.createElement("fake-icon", {
    "data-size": size,
    "data-color": color,
  })) as ActivityItem["icon"];

function makeItems(n: number): ActivityItem[] {
  return Array.from({ length: n }, (_, i) => ({
    id: `i-${i}`,
    icon: FakeIcon,
    title: `Event ${i}`,
    meta: `meta ${i}`,
    time: `${i + 1}m`,
  }));
}

describe("ActivityFeed", () => {
  it("returns null when items is empty", () => {
    const root = mount(<ActivityFeed items={[]} title="Activity" />);
    expect(root.children.length).toBe(0);
  });

  it("renders the title, streamingLabel, and up to visibleCount rows", () => {
    const root = mount(
      <ActivityFeed
        items={makeItems(8)}
        title="Activity"
        streamingLabel="Streaming"
        visibleCount={3}
        intervalMs={0}
      />,
    );
    const card = byKit(root, "card");
    expect(card?.getAttribute("data-accessibilitylabel")).toBe("Activity");
    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("Activity");
    expect(texts).toContain("Streaming");
    expect(texts).toContain("Event 0");
    expect(texts).toContain("Event 1");
    expect(texts).toContain("Event 2");
    expect(texts).not.toContain("Event 3");
  });

  it("substitutes nowLabel on the first row", () => {
    const root = mount(
      <ActivityFeed
        items={makeItems(2)}
        title="Activity"
        visibleCount={2}
        intervalMs={0}
        nowLabel="now"
      />,
    );
    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("now");
    expect(texts).toContain("2m"); // second row still uses item.time
  });

  it("caps visible rows to items.length when smaller than visibleCount", () => {
    const root = mount(
      <ActivityFeed
        items={makeItems(2)}
        title="A"
        visibleCount={5}
        intervalMs={0}
      />,
    );
    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("Event 0");
    expect(texts).toContain("Event 1");
    expect(texts).not.toContain("Event 2");
  });
});
