import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";

import { FindingsTab } from "@/features/detections/FindingsTab";
import { DetectionsTab } from "@/features/detections/DetectionsTab";
import { ChannelsTab } from "@/features/detections/ChannelsTab";
import { SilencesTab } from "@/features/detections/SilencesTab";
import { NotificationsTab } from "@/features/detections/NotificationsTab";

// Findings, detections & their notification delivery. "Findings" lands first —
// the workflow an analyst lives in; "Detections" is the authoring tab behind it.
// The remaining tabs are the delivery surface of an "alert-type" detection
// (channels it pages, silence windows, and the delivery history) — what the old
// standalone Alerts page owned, now folded in beside the detections that drive it.
export function FindingsPage() {
  return (
    <Tabs defaultValue="findings" className="flex h-full flex-col">
      <TabsList>
        <TabsTrigger value="findings">Findings</TabsTrigger>
        <TabsTrigger value="detections">Detections</TabsTrigger>
        <TabsTrigger value="channels">Channels</TabsTrigger>
        <TabsTrigger value="silences">Silences</TabsTrigger>
        <TabsTrigger value="notifications">Notifications</TabsTrigger>
      </TabsList>
      <div className="mt-4 min-h-0 flex-1">
        <TabsContent value="findings" className="h-full">
          <FindingsTab />
        </TabsContent>
        <TabsContent value="detections" className="h-full">
          <DetectionsTab />
        </TabsContent>
        <TabsContent value="channels" className="h-full">
          <ChannelsTab />
        </TabsContent>
        <TabsContent value="silences" className="h-full">
          <SilencesTab />
        </TabsContent>
        <TabsContent value="notifications" className="h-full">
          <NotificationsTab />
        </TabsContent>
      </div>
    </Tabs>
  );
}
