import { describe, it, expect } from "vitest";
import { RenderButton } from "./render-button.js";
import { RenderRichText } from "./render-rich-text.js";
import { renderHarness } from "./test-utils.js";

describe("button (CTA)", () => {
  it("renders an anchor when href is set", () => {
    const html = renderHarness(
      <RenderButton node={{ type: "button", label: "Get started", href: "/signup", variant: "solid", size: "lg" }} />,
    );
    expect(html).toContain("Get started");
    expect(html).toContain('href="/signup"');
    expect(html).toContain("<a");
  });

  it("renders a button (not a link) when no href", () => {
    const html = renderHarness(
      <RenderButton node={{ type: "button", label: "Click", variant: "outline" }} />,
    );
    expect(html).toContain("Click");
    expect(html).not.toContain('href="');
  });
});

describe("rich_text", () => {
  it("renders a safe markdown subset (no raw html injection)", () => {
    const html = renderHarness(
      <RenderRichText
        node={{
          type: "rich_text",
          value: "# Title\n\nSome **bold** and *italic* and [link](https://x.dev).\n\n- one\n- two",
        }}
      />,
    );
    expect(html).toContain("sdui-rich-text");
    expect(html).toContain("Title");
    expect(html).toContain("<strong>bold</strong>");
    expect(html).toContain("<em>italic</em>");
    expect(html).toContain('href="https://x.dev"');
    expect(html).toContain("<li>");
  });

  it("does not interpret embedded HTML tags", () => {
    const html = renderHarness(
      <RenderRichText node={{ type: "rich_text", value: "hello <script>alert(1)</script>" }} />,
    );
    // The angle brackets are escaped by React text rendering, not executed.
    expect(html).not.toContain("<script>alert(1)</script>");
    expect(html).toContain("&lt;script&gt;");
  });
});
