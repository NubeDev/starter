import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
} from "@nube/starter-ui-kit";

import { api } from "../lib/api";

export function AgentsList() {
  const qc = useQueryClient();
  const agents = useQuery({ queryKey: ["agents"], queryFn: api.agents.list });
  const [name, setName] = useState("");

  const create = useMutation({
    mutationFn: () =>
      api.agents.create({
        name: name.trim(),
        provider: "anthropic.claude",
        model: "claude-sonnet-4-6",
      }),
    onSuccess: () => {
      setName("");
      qc.invalidateQueries({ queryKey: ["agents"] });
    },
  });

  const del = useMutation({
    mutationFn: (id: string) => api.agents.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["agents"] }),
  });

  return (
    <div className="mx-auto w-full max-w-3xl p-6">
      <header className="mb-4 flex items-baseline justify-between">
        <h1 className="text-2xl font-semibold tracking-tight">Agents</h1>
        <span className="text-xs text-muted-foreground">
          {agents.data?.length ?? 0} total
        </span>
      </header>

      <Card className="mb-6 border-border/60 shadow-sm">
        <CardHeader>
          <CardTitle className="text-base">New agent</CardTitle>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (!name.trim()) return;
              create.mutate();
            }}
            className="flex gap-2"
          >
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Assistant"
              className="flex-1"
            />
            <Button type="submit" disabled={!name.trim() || create.isPending}>
              Create
            </Button>
          </form>
        </CardContent>
      </Card>

      <div className="flex flex-col gap-2">
        {agents.data?.map((a) => (
          <Card
            key={a.id}
            className="border-border/60 shadow-sm transition-colors hover:bg-accent/30"
          >
            <CardContent className="flex items-center justify-between p-4">
              <Link to={`/agents/${a.id}`} className="flex flex-col text-sm">
                <span className="font-medium">{a.name}</span>
                <span className="text-xs text-muted-foreground">
                  {a.provider} · {a.model}
                </span>
              </Link>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => del.mutate(a.id)}
                className="text-muted-foreground hover:text-destructive"
              >
                Delete
              </Button>
            </CardContent>
          </Card>
        ))}
        {agents.data?.length === 0 && (
          <p className="py-16 text-center text-sm text-muted-foreground">
            No agents yet. Create one above.
          </p>
        )}
      </div>
    </div>
  );
}
