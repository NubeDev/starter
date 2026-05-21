# @nube/starter-ui-ai-builder

React surface for the **AI Builder** — a chat transcript paired with a
live SDUI canvas that streams updates from a model-driven backend.

This package owns the page-builder slice described in
[DOCS/frontend/ai-builder/SCOPE.md](../../DOCS/frontend/ai-builder/SCOPE.md):
**R1** patch buffering, the `BuilderEvent` discriminated union,
streaming phase tracking, and the opinionated split-pane composition.
The theme-slice variant is deliberately deferred to a future
`@nube/starter-ui-ai-builder-theme` package.

## Status

- Fixture adapter (`createFixtureBuilderAdapter`) only — the Rust
  `starter-flow-node-ai-builder` crate that will power SSE streaming
  does **not exist yet**. Wire your own adapter once it lands.
- `<AiBuilderCanvas>` mounts its own minimal `SduiProvider` with
  permissive no-op defaults. Actions dispatched from inside the
  rendered tree will not round-trip to a real backend; intentional for
  the builder preview.

## Install

```bash
pnpm add @nube/starter-ui-ai-builder \
  @nube/starter-ui-chat @nube/starter-sdui-react @nube/starter-ui-kit \
  react react-dom @tanstack/react-query
```

## Opinionated quick start

```tsx
import {
  AiBuilder,
  createFixtureBuilderAdapter,
  fixtureTree,
} from "@nube/starter-ui-ai-builder";
import "@nube/starter-ui-kit/styles.css";

const adapter = createFixtureBuilderAdapter({
  scripts: [
    {
      matchPrefix: "dashboard",
      events: [
        { type: "status", phase: "thinking", message: "Planning…" },
        { type: "full-render", tree: fixtureTree("dashboard") },
        { type: "status", phase: "done" },
      ],
    },
  ],
});

export default function Page() {
  return <AiBuilder adapter={adapter} title="AI Builder" />;
}
```

## Headless composition

```tsx
import {
  AiBuilderCanvas,
  BuilderTranscript,
  useBuilder,
} from "@nube/starter-ui-ai-builder";

function MyBuilder({ adapter }) {
  const b = useBuilder({ adapter });
  return (
    <div className="grid h-full grid-cols-[24rem_1fr]">
      <BuilderTranscript
        entries={b.transcript}
        phase={b.phase}
        onSend={(text) => void b.send(text)}
        onCancel={b.cancel}
        onRetry={() => void b.retry()}
        canRetry={b.transcript.length > 0}
      />
      <AiBuilderCanvas
        tree={b.tree}
        bufferedPatches={b.bufferedPatches}
      />
    </div>
  );
}
```

## `BuilderAdapter`

```ts
interface BuilderAdapter {
  send(
    input: BuilderSendInput,
    signal: AbortSignal,
  ): AsyncIterable<BuilderEvent>;
}
```

Yield `BuilderEvent` frames in any order. The hook enforces the
SCOPE-mandated patch buffer (R1): a `patch` whose
`targetComponentId` is not present in the current tree is held for
`patchBufferMs` (default `2000`) and replayed once a parent lands.
Stale buffered patches are dropped silently after the window expires.

### Event shape (summary)

| `type`         | Meaning                                              |
| -------------- | ---------------------------------------------------- |
| `full-render`  | Replace the entire `UiComponentTree`.                |
| `patch`        | Replace the subtree at `targetComponentId`.          |
| `token-patch`  | Theme-slice token diff (page slice ignores).         |
| `shell-patch`  | Theme-slice shell diff (page slice ignores).         |
| `status`       | Move the phase machine; optional human message.      |
| `error`        | Terminate the stream with an error message.          |

### Sketching a real SSE adapter

```ts
import type { BuilderAdapter, BuilderEvent } from "@nube/starter-ui-ai-builder";

export function createSseBuilderAdapter(url: string): BuilderAdapter {
  return {
    async *send(input, signal) {
      const res = await fetch(url, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
        signal,
      });
      if (!res.ok || !res.body) {
        throw new Error(`builder ${res.status}`);
      }
      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buf = "";
      for (;;) {
        const { value, done } = await reader.read();
        if (done) return;
        buf += decoder.decode(value, { stream: true });
        // …split on \n\n, parse SSE frames into BuilderEvent…
        // yield each parsed event
      }
    },
  };
}
```

## Phases

`idle → thinking → writing → done | error | cancelled`

`<AiBuilder>` maps each phase onto `ChatComposer`'s `ChatStatus`, so
the send/cancel button toggles automatically.

## Layout

`<AiBuilder>` uses CSS grid with a default split of
`minmax(20rem, 28rem) 1fr` on `md+` viewports, single-column below.
Override via `splitClassName`, or pass `canvasOnly` / `transcriptOnly`
to render just one pane.
