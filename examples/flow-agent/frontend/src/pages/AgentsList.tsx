import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
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

      <Card className="mb-6 rounded-xl border border-border/60 shadow-sm ring-0">
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
            className="rounded-xl border border-border/60 shadow-sm ring-0 transition-colors hover:bg-accent/30"
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
          <Empty className="border border-dashed border-border/60 bg-card/30">
            <EmptyHeader>
              <EmptyMedia variant="icon" aria-hidden>
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  className="size-5"
                >
                  <circle cx="12" cy="8" r="3.25" />
                  <path d="M5 20c0-3.5 3.13-6 7-6s7 2.5 7 6" strokeLinecap="round" />
                </svg>
              </EmptyMedia>
              <EmptyTitle>No agents yet</EmptyTitle>
              <EmptyDescription>
                Create an agent to chat with a model and call flows as tools.
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button
                onClick={() => {
                  const el = document.querySelector<HTMLInputElement>(
                    'input[placeholder="Assistant"]',
                  );
                  el?.focus();
                }}
              >
                New agent
              </Button>
            </EmptyContent>
          </Empty>
        )}
      </div>
    </div>
  );
}
