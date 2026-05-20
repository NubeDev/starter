// Stage 4: read-only provider detection so a user can see why an
// agent might fail. The backend probes the Claude CLI binary and the
// `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` env vars; this page just
// renders the rows it returns.

import { useQuery } from "@tanstack/react-query";
import {
  Badge,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@nube/starter-ui-kit";

import { api } from "../lib/api";

export function Settings() {
  const providers = useQuery({
    queryKey: ["providers"],
    queryFn: api.providers.list,
    // Re-probe when the user comes back to the tab — env var changes
    // require a server restart, but Claude CLI install status can
    // change at runtime.
    refetchOnWindowFocus: true,
  });

  return (
    <div className="mx-auto w-full max-w-3xl p-6">
      <h1 className="mb-4 text-2xl font-semibold tracking-tight">Settings</h1>
      <Card className="rounded-xl border border-border/60 shadow-sm ring-0">
        <CardHeader>
          <CardTitle className="text-base">Providers</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-3 text-sm">
          {providers.isLoading && (
            <p className="text-muted-foreground">Probing providers…</p>
          )}
          {providers.error && (
            <p className="text-destructive">Could not load providers.</p>
          )}
          {providers.data?.map((p) => (
            <div
              key={p.id}
              className="flex items-start justify-between gap-4 rounded-lg border border-border/60 p-3"
            >
              <div className="flex flex-col">
                <span className="font-medium">{p.label}</span>
                <span className="text-xs text-muted-foreground">{p.hint}</span>
              </div>
              <Badge variant={p.available ? "default" : "outline"}>
                {p.available ? "Detected" : "Missing"}
              </Badge>
            </div>
          ))}
          {providers.data?.length === 0 && (
            <p className="text-muted-foreground">
              No providers detected. Install the Claude CLI or export an API
              key, then refresh.
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
