import { useMemo } from "react"
import { useQuery } from "@tanstack/react-query"
import { IconCheck, IconX, IconSettings } from "@tabler/icons-react"
import { ThemeEditorPage } from "@kit/theme-editor"
import { localStorageThemeTransport } from "@nube/starter-ui-core/theme-editor"
import { SettingsPage as PreferencesSettingsPage } from "@nube/starter-ui-core/preferences"
import { useTranslate } from "@nube/starter-ui-core/i18n"

import { Badge } from "@/components/ui/badge"
import { PageHero } from "@/components/page-hero"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs"
import { api } from "@/lib/api"

export function Settings() {
  const transport = useMemo(
    () => localStorageThemeTransport({ key: "fa-theme" }),
    [],
  )
  const t = useTranslate()

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <PageHero
        icon={IconSettings}
        accent="var(--accent-settings)"
        title={t("flow_agent.page.settings.title")}
        description={t("flow_agent.page.settings.description")}
      />

      <Tabs defaultValue="preferences">
        <TabsList>
          <TabsTrigger value="preferences">
            {t("flow_agent.page.settings.tab.preferences")}
          </TabsTrigger>
          <TabsTrigger value="providers">
            {t("flow_agent.page.settings.tab.providers")}
          </TabsTrigger>
          <TabsTrigger value="theme">
            {t("flow_agent.page.settings.tab.theme")}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="preferences" className="pt-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">
                {t("flow_agent.page.settings.preferences.title")}
              </CardTitle>
              <CardDescription>
                {t("flow_agent.page.settings.preferences.description")}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <PreferencesSettingsPage
                onToast={({ kind, message }) => {
                  if (typeof console !== "undefined") {
                    console.info(`[prefs] ${kind}: ${message}`)
                  }
                }}
              />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="providers" className="pt-4">
          <ProvidersPanel />
        </TabsContent>

        <TabsContent value="theme" className="pt-4">
          <Card className="overflow-hidden p-0">
            <ThemeEditorPage
              transport={transport}
              onNotify={(kind, message) => {
                if (typeof console !== "undefined") {
                  console.info(`[theme] ${kind}${message ? `: ${message}` : ""}`)
                }
              }}
              className="min-h-[640px]"
            />
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}

function ProvidersPanel() {
  const providers = useQuery({
    queryKey: ["providers"],
    queryFn: api.providers.list,
    refetchOnWindowFocus: true,
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Providers</CardTitle>
        <CardDescription>
          The backend probes the Claude CLI binary and the
          ANTHROPIC_API_KEY / OPENAI_API_KEY env vars.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {providers.isLoading && (
          <p className="text-sm text-muted-foreground">Probing providers…</p>
        )}
        {providers.error && (
          <Alert variant="destructive">
            <AlertTitle>Could not load providers</AlertTitle>
            <AlertDescription>
              Check that the flow-agent backend is reachable.
            </AlertDescription>
          </Alert>
        )}
        {providers.data?.map((p) => (
          <div
            key={p.id}
            className="flex items-start justify-between gap-4 rounded-lg border p-3"
          >
            <div className="flex flex-col">
              <span className="text-sm font-medium">{p.label}</span>
              <span className="text-xs text-muted-foreground">{p.hint}</span>
            </div>
            {p.available ? (
              <Badge className="gap-1 bg-(--accent-success)/15 text-(--accent-success) hover:bg-(--accent-success)/15">
                <IconCheck className="size-3" />
                Detected
              </Badge>
            ) : (
              <Badge variant="outline" className="gap-1 text-muted-foreground">
                <IconX className="size-3" />
                Missing
              </Badge>
            )}
          </div>
        ))}
        {providers.data && providers.data.length === 0 && (
          <p className="text-sm text-muted-foreground">
            No providers detected. Install the Claude CLI or export an API
            key, then refresh.
          </p>
        )}
      </CardContent>
    </Card>
  )
}
