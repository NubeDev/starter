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

export function FlowsList() {
  const qc = useQueryClient();
  const flows = useQuery({ queryKey: ["flows"], queryFn: api.flows.list });
  const [name, setName] = useState("");

  const create = useMutation({
    mutationFn: () => api.flows.create({ name: name.trim() }),
    onSuccess: () => {
      setName("");
      qc.invalidateQueries({ queryKey: ["flows"] });
    },
  });

  const del = useMutation({
    mutationFn: (id: string) => api.flows.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["flows"] }),
  });

  return (
    <div className="mx-auto w-full max-w-3xl p-6">
      <header className="mb-4 flex items-baseline justify-between">
        <h1 className="text-2xl font-semibold tracking-tight">Flows</h1>
        <span className="text-xs text-muted-foreground">
          {flows.data?.length ?? 0} total
        </span>
      </header>

      <Card className="mb-6 rounded-xl border border-border/60 shadow-sm ring-0">
        <CardHeader>
          <CardTitle className="text-base">New flow</CardTitle>
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
              placeholder="Customer onboarding"
              className="flex-1"
            />
            <Button type="submit" disabled={!name.trim() || create.isPending}>
              Create
            </Button>
          </form>
        </CardContent>
      </Card>

      <div className="flex flex-col gap-2">
        {flows.data?.map((f) => (
          <Card
            key={f.id}
            className="rounded-xl border border-border/60 shadow-sm ring-0 transition-colors hover:bg-accent/30"
          >
            <CardContent className="flex items-center justify-between p-4">
              <Link
                to={`/flows/${f.id}`}
                className="flex flex-col text-sm"
              >
                <span className="font-medium">{f.name}</span>
                <span className="text-xs text-muted-foreground">
                  v{f.version} · {new Date(f.updated_at).toLocaleString()}
                </span>
              </Link>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => del.mutate(f.id)}
                className="text-muted-foreground hover:text-destructive"
              >
                Delete
              </Button>
            </CardContent>
          </Card>
        ))}
        {flows.data?.length === 0 && (
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
                  <path d="M4 7h16M4 12h10M4 17h7" strokeLinecap="round" />
                </svg>
              </EmptyMedia>
              <EmptyTitle>No flows yet</EmptyTitle>
              <EmptyDescription>
                Create a flow to start wiring nodes together.
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button
                onClick={() => {
                  const el = document.querySelector<HTMLInputElement>(
                    'input[placeholder="Customer onboarding"]',
                  );
                  el?.focus();
                }}
              >
                New flow
              </Button>
            </EmptyContent>
          </Empty>
        )}
      </div>
    </div>
  );
}
