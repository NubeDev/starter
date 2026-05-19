// Smoke test: NotesClient round-trips through a stub fetch. Proves
// the consumer's client composes with StarterClient without needing
// any starter-side changes.

import { describe, expect, it } from "vitest";
import { StarterClient } from "@nube/starter-client-ts";

import { NotesClient, type Note } from "./notes-client.js";

const sample: Note = {
  id: "n-1",
  body: "hi",
  created_at: new Date().toISOString(),
  created_by: "u-1",
};

function stubFetch(routes: Record<string, (init?: RequestInit) => Response>): typeof fetch {
  return (async (input, init) => {
    const url = typeof input === "string" ? input : (input as URL | Request).toString();
    const path = new URL(url, "http://t").pathname;
    const key = `${(init?.method ?? "GET").toUpperCase()} ${path}`;
    const handler = routes[key];
    if (!handler) throw new Error(`unstubbed: ${key}`);
    return handler(init);
  }) as typeof fetch;
}

describe("NotesClient", () => {
  it("list() GETs /notes and parses the array", async () => {
    const fetch = stubFetch({
      "GET /notes": () =>
        new Response(JSON.stringify([sample]), {
          status: 200,
          headers: { "content-type": "application/json" },
        }),
    });
    const notes = new NotesClient(new StarterClient({ baseUrl: "http://t", fetch }));
    expect(await notes.list()).toEqual([sample]);
  });

  it("create() POSTs JSON to /notes and returns the created note", async () => {
    let captured: string | undefined;
    const fetch = stubFetch({
      "POST /notes": (init) => {
        captured = typeof init?.body === "string" ? init.body : undefined;
        return new Response(JSON.stringify(sample), { status: 201 });
      },
    });
    const notes = new NotesClient(new StarterClient({ baseUrl: "http://t", fetch }));
    const out = await notes.create("hi");
    expect(out.id).toBe("n-1");
    expect(JSON.parse(captured!)).toEqual({ body: "hi" });
  });

  it("propagates StarterError on non-2xx", async () => {
    const fetch = stubFetch({
      "GET /notes": () =>
        new Response(JSON.stringify({ type: "about:blank", title: "Unauthorized", status: 401 }), {
          status: 401,
          headers: { "content-type": "application/problem+json" },
        }),
    });
    const notes = new NotesClient(new StarterClient({ baseUrl: "http://t", fetch }));
    await expect(notes.list()).rejects.toMatchObject({ name: "StarterError", status: 401 });
  });
});
