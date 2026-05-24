// `/extensions` — frontend host-provider page for Phase D.2.
//
// Wires `ExtensionHostProvider` from `@nube/starter-ext-ui` against
// the rubix-agent base URL (overridable via the `VITE_RUBIX_AGENT_BASE_URL`
// env var; defaults to `http://localhost:8080` to match the rubix-agent
// dev config). The page renders `<ExtensionSlot id="main"/>` visibly so a
// human running `pnpm --filter @nube/test-ui-5 dev` against a live agent
// sees every extension that contributes to the `main` slot mount.
//
// Per SCOPE R11 the page does not issue raw `fetch`: every host call goes
// through `StarterClient`, which the manager exposes to extensions via
// `useHostClient()`.

import * as React from "react";
import * as ReactDOM from "react-dom";

import { StarterClient } from "@nube/starter-client-ts";
import {
  ExtensionHostManager,
  ExtensionHostProvider,
  ExtensionSlot,
} from "@nube/starter-ext-ui";

/**
 * Resolve the rubix-agent base URL. Build-time `VITE_RUBIX_AGENT_BASE_URL`
 * wins; otherwise we fall back to the dev port the agent listens on per
 * `rubix/dev/agent.toml`.
 */
function resolveAgentBaseUrl(): string {
  const env = (import.meta as ImportMeta & {
    env?: Record<string, string | undefined>;
  }).env;
  return env?.VITE_RUBIX_AGENT_BASE_URL ?? "http://localhost:8080";
}

/**
 * Build the singleton table the host advertises. Matches the four
 * SCOPE-R11 baseline keys; consumers wiring richer hosts add more here.
 * `instance` holds the host's actual module reference so extensions
 * negotiated against this table bind to *this* React, not a duplicate.
 */
function buildHostSingletons() {
  return {
    react: { version: React.version, instance: React },
    "react-dom": { version: ReactDOM.version, instance: ReactDOM },
  };
}

export interface ExtensionsPageProps {
  /**
   * Optional pre-built manager — tests inject one with pre-registered
   * remotes; the dev shell omits it and the page constructs its own.
   */
  host?: ExtensionHostManager;
  /** Optional override of the agent base URL, only used when `host` is omitted. */
  agentBaseUrl?: string;
}

export default function ExtensionsPage(
  props: ExtensionsPageProps = {},
): React.ReactElement {
  const host = React.useMemo(() => {
    if (props.host) return props.host;
    const baseUrl = props.agentBaseUrl ?? resolveAgentBaseUrl();
    const client = new StarterClient({ baseUrl });
    return new ExtensionHostManager({
      client,
      singletons: buildHostSingletons(),
    });
  }, [props.host, props.agentBaseUrl]);

  return (
    <ExtensionHostProvider host={host}>
      <main
        data-page="extensions"
        style={{
          padding: "1.5rem",
          fontFamily: "system-ui, sans-serif",
          display: "flex",
          flexDirection: "column",
          gap: "1rem",
        }}
      >
        <header>
          <h1 style={{ margin: 0, fontSize: "1.25rem" }}>
            Rubix extensions — <code>main</code> slot
          </h1>
          <p style={{ margin: 0, opacity: 0.7 }}>
            Every enabled extension contributing to <code>slot=main</code> mounts below.
          </p>
        </header>
        <section
          data-region="ext-main"
          style={{
            border: "1px dashed rgba(0,0,0,0.15)",
            borderRadius: "0.5rem",
            padding: "1rem",
            minHeight: "6rem",
          }}
        >
          <ExtensionSlot id="main" />
        </section>
      </main>
    </ExtensionHostProvider>
  );
}
