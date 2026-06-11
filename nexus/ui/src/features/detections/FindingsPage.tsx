import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";

import { FindingsTab } from "@/features/detections/FindingsTab";
import { DetectionsTab } from "@/features/detections/DetectionsTab";

// Findings & detections (WS-15): the browse/ack surface for the sparks an
// analytic rule emits, plus the editor for the scheduled detections that
// produce them. "Findings" lands first — it's the workflow an analyst lives in;
// "Detections" is the authoring tab behind it.
export function FindingsPage() {
  return (
    <Tabs defaultValue="findings" className="flex h-full flex-col">
      <TabsList>
        <TabsTrigger value="findings">Findings</TabsTrigger>
        <TabsTrigger value="detections">Detections</TabsTrigger>
      </TabsList>
      <div className="mt-4 min-h-0 flex-1">
        <TabsContent value="findings" className="h-full">
          <FindingsTab />
        </TabsContent>
        <TabsContent value="detections" className="h-full">
          <DetectionsTab />
        </TabsContent>
      </div>
    </Tabs>
  );
}
