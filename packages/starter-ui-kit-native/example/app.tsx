// Story-style harness — renders every primitive in light, dark, and
// two named palettes ("modern-minimal", "violet-bloom") so a reviewer
// can eyeball the kit in isolation. Mount this as the root of a fresh
// Expo app; no `rubix/mobile/` glue is required.
//
// This file is intentionally `.tsx` and ships in `src` so the
// workspace `tsc --noEmit` validates it alongside the primitives —
// the example is the kit's first consumer and must keep typechecking.

import * as React from "react";
import { ScrollView, Text, View } from "react-native";
import { useLayoutPreferences } from "@nube/starter-ui-core/theme-editor";

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
  Skeleton,
  Slider,
  Spinner,
  Switch,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  useTheme,
} from "../src/index.js";

/** A single "story" — one primitive per row, labelled. */
function Stories() {
  const t = useTheme();
  const [chk, setChk] = React.useState(false);
  const [sld, setSld] = React.useState(40);

  return (
    <ScrollView
      contentContainerStyle={{ padding: t.space(4), gap: t.space(6) }}
      style={{ flex: 1, backgroundColor: t.color("background") }}
    >
      <Text style={{ color: t.color("foreground"), fontSize: t.fontSize("lg") }}>
        starter-ui-kit-native — {t.preferences.mode} / {t.paletteId}
      </Text>

      <Row label="Button">
        <Button onPress={() => undefined}>Primary</Button>
        <Button variant="outline" size="sm">Outline sm</Button>
        <Button variant="destructive">Delete</Button>
        <Button variant="ghost">Ghost</Button>
        <Button disabled accessibilityLabel="Disabled action">…</Button>
      </Row>

      <Row label="Badge">
        <Badge>New</Badge>
        <Badge variant="secondary">v0.1</Badge>
        <Badge variant="outline">draft</Badge>
        <Badge variant="destructive">error</Badge>
      </Row>

      <Row label="Input">
        <Input
          placeholder="email"
          accessibilityLabel="Email"
          keyboardType="email-address"
        />
        <Input placeholder="invalid" invalid accessibilityLabel="Invalid field" />
      </Row>

      <Row label="Switch">
        <Switch
          checked={chk}
          onCheckedChange={setChk}
          accessibilityLabel="Notifications"
        />
        <Switch size="sm" defaultChecked accessibilityLabel="Compact mode" />
      </Row>

      <Row label="Slider">
        <Slider
          value={sld}
          onValueChange={setSld}
          accessibilityLabel="Brightness"
        />
      </Row>

      <Row label="Spinner / Skeleton">
        <Spinner />
        <Skeleton width={120} height={16} />
      </Row>

      <Tabs defaultValue="a">
        <TabsList>
          <TabsTrigger value="a">A</TabsTrigger>
          <TabsTrigger value="b">B</TabsTrigger>
        </TabsList>
        <TabsContent value="a">
          <Text style={{ color: t.color("foreground") }}>Tab A body</Text>
        </TabsContent>
        <TabsContent value="b">
          <Text style={{ color: t.color("foreground") }}>Tab B body</Text>
        </TabsContent>
      </Tabs>

      <Card>
        <CardHeader>
          <CardTitle>Card title</CardTitle>
          <CardDescription>Card description goes here.</CardDescription>
        </CardHeader>
        <CardContent>
          <Text style={{ color: t.color("card-foreground") }}>Body content.</Text>
        </CardContent>
        <CardFooter>
          <Button size="sm">OK</Button>
        </CardFooter>
      </Card>

      <Select defaultValue="one">
        <SelectTrigger placeholder="Pick…" accessibilityLabel="Select fruit" />
        <SelectContent>
          <SelectItem value="one">One</SelectItem>
          <SelectItem value="two">Two</SelectItem>
          <SelectItem value="three">Three</SelectItem>
        </SelectContent>
      </Select>

      <Sheet>
        <SheetTrigger>
          <Button variant="outline">Open sheet</Button>
        </SheetTrigger>
        <SheetContent>
          <SheetHeader>
            <SheetTitle>Sheet title</SheetTitle>
            <SheetDescription>Sheet description body.</SheetDescription>
          </SheetHeader>
        </SheetContent>
      </Sheet>

      <Dialog>
        <DialogTrigger>
          <Button>Open dialog</Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Dialog title</DialogTitle>
            <DialogDescription>Are you sure?</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline">Cancel</Button>
            <Button variant="destructive">Confirm</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Tooltip>
        <TooltipTrigger accessibilityLabel="Info">
          <Badge>?</Badge>
        </TooltipTrigger>
        <TooltipContent>Long-press hint.</TooltipContent>
      </Tooltip>
    </ScrollView>
  );
}

function Row({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  const t = useTheme();
  return (
    <View style={{ gap: t.space(2) }}>
      <Text style={{ color: t.color("muted-foreground"), fontSize: t.fontSize("xs") }}>
        {label}
      </Text>
      <View style={{ flexDirection: "row", flexWrap: "wrap", gap: t.space(2) }}>
        {children}
      </View>
    </View>
  );
}

/** Mount this as the root of a fresh Expo app. Drives the global
 * `useLayoutPreferences` store so toggling the mode / palette in
 * dev tools (or a Settings screen) re-themes the harness live. */
export default function ExampleApp() {
  const prefs = useLayoutPreferences();
  // Default to platform-default if the consumer hasn't picked one;
  // the harness rotates through {light/platform, dark/platform,
  // light/modern-minimal, dark/violet-bloom} by re-rendering with
  // these calls below.
  React.useEffect(() => {
    if (!prefs.palette) prefs.setPalette("platform-default");
  }, [prefs]);
  return <Stories />;
}

/** Reviewer convenience — switch between named palettes from a REPL
 * or a dev menu. Not exported from the kit itself. */
export const PALETTE_CYCLE = [
  { mode: "light" as const, palette: "platform-default" },
  { mode: "dark" as const, palette: "platform-default" },
  { mode: "light" as const, palette: "modern-minimal" },
  { mode: "dark" as const, palette: "violet-bloom" },
];
