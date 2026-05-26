import { describe, expect, it } from "vitest";

import { buildPuckConfig } from "../build-puck-config.js";
import { OVERRIDES, RESOLVER_ONLY_VARIANTS } from "../curation/overrides.js";
import { PALETTE_TAXONOMY } from "../curation/palette-taxonomy.js";
import { SLOTS } from "../curation/slots.js";
import { IR_SCHEMA } from "../schema-loader.js";
import type { JsonSchema } from "../schema-walker.js";
import { variantTypeOf } from "../schema-walker.js";

describe("buildPuckConfig", () => {
  const config = buildPuckConfig({ schema: IR_SCHEMA });

  it("emits one ComponentConfig for every author-time variant in the IR", () => {
    const componentDef = (IR_SCHEMA as JsonSchema).definitions?.["Component"];
    expect(componentDef?.oneOf).toBeDefined();

    const authorTimeVariants: string[] = [];
    for (const arm of componentDef!.oneOf!) {
      const v = variantTypeOf(arm);
      if (!v) continue;
      if (RESOLVER_ONLY_VARIANTS.includes(v)) continue;
      authorTimeVariants.push(v);
    }

    // Sanity — the schema MUST carry at least the core layout
    // primitives. A regression here means we changed how we
    // enumerate variants, not the IR.
    expect(authorTimeVariants).toEqual(
      expect.arrayContaining(["page", "row", "col", "grid", "chart", "kpi"]),
    );

    for (const v of authorTimeVariants) {
      expect(
        config.components[v],
        `expected generated config to include "${v}"`,
      ).toBeDefined();
    }
  });

  it("excludes resolver-only variants (forbidden / dangling / unknown)", () => {
    for (const banned of RESOLVER_ONLY_VARIANTS) {
      expect(config.components[banned]).toBeUndefined();
    }
  });

  it("treats `row.children` as a slot, not an authored array", () => {
    const row = config.components["row"];
    expect(row).toBeDefined();
    const childrenField = row!.fields["children"];
    expect(childrenField).toBeDefined();
    expect(childrenField).toEqual({ type: "slot" });
  });

  it("places curated slot tuples as slot fields on their parent variant", () => {
    for (const slot of SLOTS) {
      // The nested `tabs[].children` tuple is documented in PR1
      // but slot-inside-array Puck wiring lands later — skip the
      // direct-property check for it.
      if (slot.propertyPath.includes("[]")) continue;
      const c = config.components[slot.variant];
      expect(c, `missing component config for ${slot.variant}`).toBeDefined();
      const f = c!.fields[slot.propertyPath];
      expect(
        f,
        `expected ${slot.variant}.${slot.propertyPath} to be a slot field`,
      ).toEqual({ type: "slot" });
    }
  });

  it("buckets every author-time variant under a palette category present in the taxonomy", () => {
    // Every key in components should either be classified or surface
    // under the "uncategorised" bucket in `categories`.
    const allCatComponents = new Set<string>();
    for (const cat of Object.values(config.categories ?? {})) {
      for (const c of cat.components) allCatComponents.add(c);
    }
    for (const v of Object.keys(config.components)) {
      expect(allCatComponents.has(v)).toBe(true);
      // Anything in the taxonomy must not appear under
      // "uncategorised" (that bucket is a tripwire for missing
      // entries, not a catch-all).
      if (PALETTE_TAXONOMY[v]) {
        expect(config.categories?.["uncategorised"]?.components ?? []).not.toContain(v);
      }
    }
  });

  it("renders a stable snapshot of the generated config", () => {
    // We snapshot a *serialisable projection*: field shapes per
    // variant, category buckets. The render function references are
    // intentionally stripped — they're React components and break
    // structural snapshotting.
    const projected = {
      components: Object.fromEntries(
        Object.entries(config.components).map(([k, v]) => [
          k,
          {
            label: v.label,
            fields: v.fields,
            defaultProps: v.defaultProps,
          },
        ]),
      ),
      categories: config.categories,
    };
    expect(projected).toMatchSnapshot();
  });

  it("does not register any resolver-only variant as an override", () => {
    for (const banned of RESOLVER_ONLY_VARIANTS) {
      expect(Object.prototype.hasOwnProperty.call(OVERRIDES, banned)).toBe(false);
    }
  });
});
