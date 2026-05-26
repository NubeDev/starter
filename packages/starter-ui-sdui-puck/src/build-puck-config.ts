// Pure function: (schema, slots, overrides, bindable, taxonomy) →
// PuckConfig. Walks `definitions.Component.oneOf` arm-by-arm and
// emits one Puck ComponentConfig per author-time variant. The
// transform is deterministic and dependency-free — the test suite
// snapshots its output.
//
// See rubix/docs/scope/dashboards/10-puck-builder.md §B1 for the
// authoritative field-mapping table; this file implements it.

import { BINDABLE, type BindableTuple } from "./curation/bindable.js";
import {
  DATA_SOURCES,
  dataSourceKindOf,
  type DataSourceTuple,
} from "./curation/data-sources.js";
import {
  OVERRIDES,
  RESOLVER_ONLY_VARIANTS,
} from "./curation/overrides.js";
import {
  PALETTE_TAXONOMY,
  type PaletteCategory,
} from "./curation/palette-taxonomy.js";
import { SLOTS, type SlotTuple } from "./curation/slots.js";
import { makeDataSourceField } from "./data-source-field.js";
import { makePlaceholderRenderer } from "./placeholder-renderer.js";
import type {
  PuckComponentConfigStub,
  PuckConfigStub,
  PuckFieldStub,
} from "./puck-types.js";
import {
  flatten,
  refName,
  resolveRef,
  variantTypeOf,
  type JsonSchema,
} from "./schema-walker.js";

/** Inputs to the pure generator. Defaults wire the curated tables. */
export interface BuildPuckConfigOpts {
  /** The committed IR JSON Schema artifact. */
  schema: JsonSchema;
  /** Slot tuples — defaults to {@link SLOTS}. */
  slots?: readonly SlotTuple[];
  /** Override map — defaults to {@link OVERRIDES}. */
  overrides?: Readonly<Record<string, PuckComponentConfigStub | null>>;
  /** Bindable tuples — defaults to {@link BINDABLE}. */
  bindable?: readonly BindableTuple[];
  /** Catalogue-backed data-source tuples — defaults to
   *  {@link DATA_SOURCES}. Per scope §B3. */
  dataSources?: readonly DataSourceTuple[];
  /** Palette taxonomy — defaults to {@link PALETTE_TAXONOMY}. */
  taxonomy?: Readonly<Record<string, PaletteCategory>>;
}

/** Property keys we drop wholesale from every variant in v1. */
const SKIPPED_PROPERTIES: ReadonlySet<string> = new Set([
  "type", // discriminator — Puck supplies it from the component key.
  "style", // NodeStyle skipped in v1 per scope §B1 / Q2.
]);

/**
 * Build a Puck `Config` from the IR schema + curated tables.
 *
 * The function is pure: same inputs ⇒ same output, byte-stable
 * (modulo the renderer ComponentType references, which are
 * referentially stable per variant within a single call).
 */
export function buildPuckConfig(opts: BuildPuckConfigOpts): PuckConfigStub {
  const schema = opts.schema;
  const slots = opts.slots ?? SLOTS;
  const overrides = opts.overrides ?? OVERRIDES;
  const bindable = opts.bindable ?? BINDABLE;
  const dataSources = opts.dataSources ?? DATA_SOURCES;
  const taxonomy = opts.taxonomy ?? PALETTE_TAXONOMY;

  const componentDef = schema.definitions?.["Component"];
  if (!componentDef || !componentDef.oneOf) {
    throw new Error(
      "[build-puck-config] schema is missing definitions.Component.oneOf",
    );
  }

  const components: Record<string, PuckComponentConfigStub> = {};
  const buckets: Record<PaletteCategory | "uncategorised", string[]> = {
    layout: [],
    display: [],
    interactive: [],
    custom: [],
    uncategorised: [],
  };

  for (const arm of componentDef.oneOf) {
    const variant = variantTypeOf(arm);
    if (!variant) continue;

    // Drop resolver-only variants — scope §B2. Curated list lives
    // in overrides.ts; the assertion there guarantees they cannot
    // sneak back in via an override.
    if (RESOLVER_ONLY_VARIANTS.includes(variant)) continue;

    const override = overrides[variant];
    const config: PuckComponentConfigStub = override
      ? override
      : armToComponentConfig({
          schema,
          arm,
          variant,
          slots,
          bindable,
          dataSources,
          refStack: new Set<string>(),
        });

    components[variant] = config;
    const cat = taxonomy[variant] ?? "uncategorised";
    buckets[cat].push(variant);
  }

  // Build the `categories` map only for non-empty buckets so the
  // Puck palette UI doesn't render dead headers. "uncategorised"
  // intentionally lands at the bottom — scope §B2 wants missing
  // classifications visible.
  const categories: NonNullable<PuckConfigStub["categories"]> = {};
  for (const cat of [
    "layout",
    "display",
    "interactive",
    "custom",
    "uncategorised",
  ] as const) {
    if (buckets[cat].length > 0) {
      categories[cat] = {
        title: titleCase(cat),
        components: [...buckets[cat]].sort(),
      };
    }
  }

  return { components, categories };
}

interface ArmCtx {
  schema: JsonSchema;
  arm: JsonSchema;
  variant: string;
  slots: readonly SlotTuple[];
  bindable: readonly BindableTuple[];
  dataSources: readonly DataSourceTuple[];
  /** $ref names currently being expanded on this branch — used to
   *  break cycles in self-referencing definitions (e.g. TreeItem,
   *  ShowWhen). When a ref reappears we collapse it to a text field
   *  rather than recursing forever. */
  refStack: ReadonlySet<string>;
}

function armToComponentConfig(ctx: ArmCtx): PuckComponentConfigStub {
  const fields: Record<string, PuckFieldStub> = {};
  const defaultProps: Record<string, unknown> = {};
  const properties = ctx.arm.properties ?? {};

  for (const [propName, propSchema] of Object.entries(properties)) {
    if (SKIPPED_PROPERTIES.has(propName)) continue;
    const field = propertyToField({
      ctx,
      propName,
      propSchema,
    });
    if (!field) continue;
    fields[propName] = field;
    if (propSchema.default !== undefined) {
      defaultProps[propName] = propSchema.default;
    }
  }

  return {
    fields,
    defaultProps: Object.keys(defaultProps).length > 0 ? defaultProps : undefined,
    render: makePlaceholderRenderer(ctx.variant),
    label: ctx.variant,
  };
}

interface PropCtx {
  ctx: ArmCtx;
  propName: string;
  propSchema: JsonSchema;
}

function propertyToField(p: PropCtx): PuckFieldStub | undefined {
  const { ctx, propName, propSchema } = p;
  const flat = flatten(ctx.schema, propSchema);

  // §B3 catalogue-backed picker — checked FIRST so curation wins
  // over the default type dispatch. The actual picker UI lives in
  // `data-source-field.tsx`; here we only emit a `custom` field
  // bound to the right `CatalogueKind`. Consulted via the curation
  // tuples in `data-sources.ts`; defaults are wired by the caller.
  for (const ds of ctx.dataSources) {
    if (ds.variant === ctx.variant && ds.propertyPath === propName) {
      return makeDataSourceField(ds.kind);
    }
  }
  // Also consult the helper so tests can patch DATA_SOURCES via the
  // default and still see the same outcome — costs nothing at runtime.
  void dataSourceKindOf;

  // Slot — children-array of a layout container, per the curated
  // SLOTS table. Scope §B1: this is a drag-target, NOT an array
  // field. (The nested `tabs[].children` tuple is not handled in
  // PR1 — slot-inside-array Puck wiring lands later. We fall
  // through to the array branch for that case.)
  for (const slot of ctx.slots) {
    if (slot.variant === ctx.variant && slot.propertyPath === propName) {
      return { type: "slot" };
    }
  }

  // Enum string → select.
  if (flat.type === "string" && Array.isArray(flat.enum)) {
    return {
      type: "select",
      options: flat.enum.map((v) => ({
        label: String(v),
        value: String(v),
      })),
    };
  }

  // String → text. Bindable strings *also* render as text in PR1
  // (BindingAwareField is deferred — see bindable.ts header).
  if (isTypeOrIncludes(flat.type, "string")) {
    void isBindablePath(ctx, propName); // referenced for future wiring
    return { type: "text" };
  }

  // Numbers (integer or number) → number.
  if (
    isTypeOrIncludes(flat.type, "number") ||
    isTypeOrIncludes(flat.type, "integer")
  ) {
    return { type: "number" };
  }

  // Booleans → yes/no radio per scope §B1.
  if (isTypeOrIncludes(flat.type, "boolean")) {
    return {
      type: "radio",
      options: [
        { label: "Yes", value: true },
        { label: "No", value: false },
      ],
    };
  }

  // Arrays.
  if (flat.type === "array" && flat.items && !Array.isArray(flat.items)) {
    const rawItems = flat.items as JsonSchema;
    const items = flatten(ctx.schema, rawItems);
    // Detect "items is the Component union" via the original $ref
    // before flatten (flatten resolves it; we need the name).
    const componentItem = rawItems.$ref === "#/definitions/Component";

    if (componentItem) {
      // Data-bearing variants that AUTHOR arrays of Component
      // (chart.sources, kpi_grid.kpis): per scope §B1 these are
      // *not* slots. Render as an authored array of nested objects.
      // The slot tuples in SLOTS already short-circuited any
      // layout-container case before we got here.
      return {
        type: "array",
        arrayFields: { type: { type: "text" } },
      };
    }

    // Array of $ref (non-Component): authored array of objects.
    // Guard against cycles — TreeItem and similar self-recursive
    // definitions would otherwise blow the stack.
    const itemsRef = (flat.items as JsonSchema).$ref;
    if (itemsRef) {
      const name = refName(itemsRef);
      if (ctx.refStack.has(name)) {
        return { type: "text" };
      }
      const refTarget = resolveRef(ctx.schema, itemsRef);
      return objectArrayField(ctx, refTarget, name);
    }

    // Array of primitives: leaf array field.
    if (items.type === "string") {
      return { type: "array", arrayFields: { value: { type: "text" } } };
    }
    if (items.type === "number" || items.type === "integer") {
      return { type: "array", arrayFields: { value: { type: "number" } } };
    }
    if (items.type === "boolean") {
      return {
        type: "array",
        arrayFields: {
          value: {
            type: "radio",
            options: [
              { label: "Yes", value: true },
              { label: "No", value: false },
            ],
          },
        },
      };
    }
    // Fallback: opaque array.
    return { type: "array", arrayFields: { value: { type: "text" } } };
  }

  // Object — nested `$ref` or inline object.
  if (flat.$ref) {
    // Per scope §B1: `$ref` (non-Component) → external/custom
    // selector field once B3 lands. PR1 renders as plain text —
    // the operator can still type a value; the selector lands in
    // PR2.
    return { type: "text" };
  }

  if (flat.type === "object" && flat.properties) {
    const objectFields: Record<string, PuckFieldStub> = {};
    for (const [k, v] of Object.entries(flat.properties)) {
      const f = propertyToField({ ctx, propName: `${propName}.${k}`, propSchema: v });
      if (f) objectFields[k] = f;
    }
    if (Object.keys(objectFields).length > 0) {
      return { type: "object", objectFields };
    }
    // Open object (no declared properties) — fall through to text.
    return { type: "text" };
  }

  // oneOf at a property — render as text in PR1; a real adapter
  // (union picker) lands when overrides for the parent variant do.
  if (flat.oneOf) {
    return { type: "text" };
  }

  // Unknown shape — text fallback so the field is at least
  // present and editable. Avoids silently dropping data.
  return { type: "text" };
}

function isTypeOrIncludes(t: unknown, name: string): boolean {
  if (t === name) return true;
  if (Array.isArray(t) && t.includes(name)) return true;
  return false;
}

function isBindablePath(ctx: ArmCtx, propPath: string): boolean {
  return ctx.bindable.some(
    (b) => b.variant === ctx.variant && b.propertyPath === propPath,
  );
}

function objectArrayField(
  ctx: ArmCtx,
  def: JsonSchema,
  defName: string,
): PuckFieldStub {
  const childCtx: ArmCtx = {
    ...ctx,
    refStack: new Set<string>([...ctx.refStack, defName]),
  };
  const objectFields: Record<string, PuckFieldStub> = {};
  for (const [k, v] of Object.entries(def.properties ?? {})) {
    const f = propertyToField({ ctx: childCtx, propName: k, propSchema: v });
    if (f) objectFields[k] = f;
  }
  // Puck `array.arrayFields` is the per-item schema — same shape
  // as objectFields.
  return {
    type: "array",
    arrayFields:
      Object.keys(objectFields).length > 0
        ? objectFields
        : { value: { type: "text" } },
  };
}

function titleCase(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
