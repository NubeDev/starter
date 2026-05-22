import { useState } from "react"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { Link } from "react-router-dom"
import { IconPlus, IconRobot, IconTrash } from "@tabler/icons-react"

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
import { useDateFormatters } from "@/hooks/use-date-formatters"
import { useTranslate } from "@nube/starter-ui-core/i18n"

export function AgentsList() {
  const qc = useQueryClient()
  const agents = useQuery({ queryKey: ["agents"], queryFn: api.agents.list })
  const [name, setName] = useState("")
  const dates = useDateFormatters()
  const t = useTranslate()

  const create = useMutation({
    mutationFn: () =>
      api.agents.create({
        name: name.trim(),
        provider: "anthropic.claude",
        model: "claude-sonnet-4-6",
      }),
    onSuccess: () => {
      setName("")
      qc.invalidateQueries({ queryKey: ["agents"] })
    },
  })

  const del = useMutation({
    mutationFn: (id: string) => api.agents.delete(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["agents"] }),
  })

  const total = agents.data?.length ?? 0
  const isEmpty = total === 0 && !agents.isLoading

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <PageHero
        icon={IconRobot}
        accent="var(--accent-agents)"
        title={t("flow_agent.page.agents.title")}
        description={t("flow_agent.page.agents.description")}
        actions={<Badge variant="secondary">{total} total</Badge>}
      />

      <Card className="card-lift">
        <CardHeader>
          <CardTitle className="text-base">
            {t("flow_agent.page.agents.new.title")}
          </CardTitle>
          <CardDescription>
            {t("flow_agent.page.agents.new.description")}
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
              placeholder={t("flow_agent.page.agents.new.placeholder")}
              className="flex-1"
            />
            <Button type="submit" disabled={!name.trim() || create.isPending}>
              <IconPlus className="size-4" />
              {t("flow_agent.action.create")}
            </Button>
          </form>
        </CardContent>
      </Card>

      {isEmpty ? (
        <Empty className="border border-dashed bg-card/30">
          <EmptyHeader>
            <EmptyMedia variant="icon" aria-hidden>
              <IconRobot className="size-5" />
            </EmptyMedia>
            <EmptyTitle>{t("flow_agent.page.agents.empty.title")}</EmptyTitle>
            <EmptyDescription>
              {t("flow_agent.page.agents.empty.description")}
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent />
        </Empty>
      ) : (
        <Card className="overflow-hidden p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("flow_agent.table.name")}</TableHead>
                <TableHead>{t("flow_agent.table.provider")}</TableHead>
                <TableHead>{t("flow_agent.table.model")}</TableHead>
                <TableHead className="w-56">{t("flow_agent.table.updated")}</TableHead>
                <TableHead className="w-16" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {agents.data?.map((a) => (
                <TableRow key={a.id}>
                  <TableCell>
                    <Link
                      to={`/agents/${a.id}`}
                      className="font-medium hover:underline"
                    >
                      {a.name}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline">{a.provider}</Badge>
                  </TableCell>
                  <TableCell className="font-mono text-xs text-muted-foreground">
                    {a.model}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {dates.dateTime(a.updated_at)}
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => del.mutate(a.id)}
                      aria-label={t("flow_agent.action.delete", { name: a.name })}
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
