import { useState } from "react"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { Link } from "react-router-dom"
import { IconPlus, IconTrash, IconSitemap } from "@tabler/icons-react"

import { Button } from "@/components/ui/button"
import { PageHero } from "@/components/page-hero"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { api } from "@/lib/api"

export function FlowsList() {
  const qc = useQueryClient()
  const flows = useQuery({ queryKey: ["flows"], queryFn: api.flows.list })
  const [name, setName] = useState("")

  const create = useMutation({
    mutationFn: () => api.flows.create({ name: name.trim() }),
    onSuccess: () => {
      setName("")
      qc.invalidateQueries({ queryKey: ["flows"] })
    },
  })

  const del = useMutation({
    mutationFn: (id: string) => api.flows.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["flows"] }),
  })

  const total = flows.data?.length ?? 0
  const isEmpty = total === 0 && !flows.isLoading

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <PageHero
        icon={IconSitemap}
        accent="var(--accent-flows)"
        title="Flows"
        description="Compose node graphs and fire them on demand."
        actions={<Badge variant="secondary">{total} total</Badge>}
      />

      <Card className="card-lift">
        <CardHeader>
          <CardTitle className="text-base">New flow</CardTitle>
          <CardDescription>
            Give the flow a short, descriptive name.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              if (!name.trim()) return
              create.mutate()
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
              <IconPlus className="size-4" />
              Create
            </Button>
          </form>
        </CardContent>
      </Card>

      {isEmpty ? (
        <Empty className="border border-dashed bg-card/30">
          <EmptyHeader>
            <EmptyMedia variant="icon" aria-hidden>
              <IconSitemap className="size-5" />
            </EmptyMedia>
            <EmptyTitle>No flows yet</EmptyTitle>
            <EmptyDescription>
              Create a flow above to start wiring nodes together.
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent />
        </Empty>
      ) : (
        <Card className="overflow-hidden p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead className="w-24">Version</TableHead>
                <TableHead className="w-56">Updated</TableHead>
                <TableHead className="w-16" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {flows.data?.map((f) => (
                <TableRow key={f.id}>
                  <TableCell>
                    <Link
                      to={`/flows/${f.id}`}
                      className="font-medium hover:underline"
                    >
                      {f.name}
                    </Link>
                    {f.description ? (
                      <div className="text-xs text-muted-foreground">
                        {f.description}
                      </div>
                    ) : null}
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline">v{f.version}</Badge>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {new Date(f.updated_at).toLocaleString()}
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => del.mutate(f.id)}
                      aria-label={`Delete ${f.name}`}
                      className="text-muted-foreground hover:text-destructive"
                    >
                      <IconTrash className="size-4" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      )}
    </div>
  )
}
