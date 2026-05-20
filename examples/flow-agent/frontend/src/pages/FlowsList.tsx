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

      <Card className="mb-6 border-border/60 shadow-sm">
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
            className="border-border/60 shadow-sm transition-colors hover:bg-accent/30"
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
          <p className="py-16 text-center text-sm text-muted-foreground">
            No flows yet. Create one above.
          </p>
        )}
      </div>
    </div>
  );
}
