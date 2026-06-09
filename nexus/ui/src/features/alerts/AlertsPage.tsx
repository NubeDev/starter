import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";

import { RulesTab } from "@/features/alerts/RulesTab";
import { ChannelsTab } from "@/features/alerts/ChannelsTab";
import { SilencesTab } from "@/features/alerts/SilencesTab";
import { EventsTab } from "@/features/alerts/EventsTab";

// Alerting management over the real `/alerts/*` endpoints: threshold rules,
// notification channels, silence windows, and the fired-event history.
// Each tab owns its own data subscription; all render loading/empty/error
// (F0). Replaces the earlier "coming soon" placeholder now that the
// alerts contract is published.
export function AlertsPage() {
  return (
    <Tabs defaultValue="rules" className="flex h-full flex-col">
      <TabsList>
        <TabsTrigger value="rules">Rules</TabsTrigger>
        <TabsTrigger value="channels">Channels</TabsTrigger>
        <TabsTrigger value="silences">Silences</TabsTrigger>
        <TabsTrigger value="events">Events</TabsTrigger>
      </TabsList>
      <div className="mt-4 min-h-0 flex-1">
        <TabsContent value="rules" className="h-full">
          <RulesTab />
        </TabsContent>
        <TabsContent value="channels" className="h-full">
          <ChannelsTab />
        </TabsContent>
        <TabsContent value="silences" className="h-full">
          <SilencesTab />
        </TabsContent>
        <TabsContent value="events" className="h-full">
          <EventsTab />
        </TabsContent>
      </div>
    </Tabs>
  );
}
