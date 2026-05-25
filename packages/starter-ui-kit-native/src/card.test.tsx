import { describe, expect, it } from "vitest";

import { Card, CardContent, CardHeader, CardTitle } from "./card.js";
import { mount } from "./test-utils.js";

describe("<Card>", () => {
  it("renders the composed structure with summary a11y role", () => {
    const root = mount(
      <Card accessibilityLabel="Sensor card">
        <CardHeader>
          <CardTitle>Sensor</CardTitle>
        </CardHeader>
        <CardContent />
      </Card>,
    );
    const card = root.querySelector('[accessibilityrole="summary"]');
    expect(card).not.toBeNull();
    expect(card?.getAttribute("accessibilitylabel")).toBe("Sensor card");
    expect(root.querySelector('[accessibilityrole="header"]')).not.toBeNull();
    expect(root).toMatchSnapshot();
  });
});
