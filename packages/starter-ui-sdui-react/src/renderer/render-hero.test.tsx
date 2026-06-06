import { describe, it, expect } from "vitest";
import { RenderHero } from "./render-hero.js";
import { RenderImage } from "./render-image.js";
import { RenderSpacer } from "./render-spacer.js";
import { RenderSection } from "./render-section.js";
import { renderHarness } from "./test-utils.js";

describe("content blocks", () => {
  it("hero renders title + eyebrow and applies gradient style token", () => {
    const html = renderHarness(
      <RenderHero
        node={{
          type: "hero",
          eyebrow: "NEW",
          title: "Welcome",
          subtitle: "sub",
          style: { gradient: "dusk", spacing: "xl" },
        }}
      />,
    );
    expect(html).toContain("sdui-hero");
    expect(html).toContain("Welcome");
    expect(html).toContain("NEW");
    expect(html).toContain('data-sdui-gradient="dusk"');
    expect(html).toContain('data-sdui-spacing="xl"');
  });

  it("image honours aspect/fit tokens and decorative alt", () => {
    const html = renderHarness(
      <RenderImage node={{ type: "image", src: "/a.png", alt: "", aspect: "video", fit: "contain" }} />,
    );
    expect(html).toContain("aspect-video");
    expect(html).toContain("object-contain");
    expect(html).toContain('alt=""');
  });

  it("spacer emits a sized aria-hidden gap", () => {
    const html = renderHarness(<RenderSpacer node={{ type: "spacer", size: "lg" }} />);
    expect(html).toContain("sdui-spacer");
    expect(html).toContain("aria-hidden");
  });

  it("section renders a landmark element with title", () => {
    const html = renderHarness(
      <RenderSection node={{ type: "section", title: "Stats", landmark: "region", children: [] }} />,
    );
    expect(html).toContain("sdui-section");
    expect(html).toContain("Stats");
  });
});
