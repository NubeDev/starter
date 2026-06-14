import { describe, expect, it } from "vitest";

import { parseFlowSection, toCreateFlow } from "@/features/flows/flowDraft";

// A flow's input/pipeline/output are opaque JSON the backend stores
// verbatim. The editor authors them as text, so the only real logic is
// parsing + validating that text into JSON before it's sent. Pinned here
// (F10) so a malformed config is caught in the UI, not at the API.
describe("parseFlowSection", () => {
  it("parses valid JSON to a value", () => {
    expect(parseFlowSection('{"type":"sql"}')).toEqual({
      ok: true,
      value: { type: "sql" },
    });
  });

  it("treats blank text as an empty object (an omitted section)", () => {
    expect(parseFlowSection("   ")).toEqual({ ok: true, value: {} });
  });

  it("reports a parse error for malformed JSON", () => {
    const r = parseFlowSection("{not json}");
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toMatch(/./);
  });

  it("accepts an array (a pipeline is a list of processors)", () => {
    expect(parseFlowSection("[]")).toEqual({ ok: true, value: [] });
  });

  it("rejects a scalar top level (a flow section is an object or array)", () => {
    expect(parseFlowSection("42").ok).toBe(false);
    expect(parseFlowSection('"x"').ok).toBe(false);
    expect(parseFlowSection("null").ok).toBe(false);
  });
});

describe("toCreateFlow", () => {
  it("assembles a create request from a valid draft", () => {
    const r = toCreateFlow({
      name: "weather",
      enabled: true,
      input: '{"type":"http"}',
      pipeline: "[]",
      output: '{"type":"postgres"}',
    });
    expect(r.ok).toBe(true);
    if (r.ok) {
      expect(r.value.name).toBe("weather");
      expect(r.value.enabled).toBe(true);
      expect(r.value.input).toEqual({ type: "http" });
      expect(r.value.output).toEqual({ type: "postgres" });
    }
  });

  it("surfaces which section failed to parse", () => {
    const r = toCreateFlow({
      name: "x",
      enabled: false,
      input: "{bad}",
      pipeline: "",
      output: "",
    });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.field).toBe("input");
  });
});
