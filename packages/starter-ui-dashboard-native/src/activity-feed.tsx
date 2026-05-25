// `<ActivityFeed>` — RN port of
// `starter-ui-dashboard/src/activity-feed.tsx`. Prop API mirrors the
// web version one-to-one.
//
// `ActivityItem.icon` is typed against lucide-react's `LucideIcon` for
// strict parity with the web `ActivityItem`. On mobile, callers may
// pass the corresponding lucide-react-native icon — the structural
// shape (a React component accepting `size`/`color`) is compatible,
// and `lucide-react` is declared as an optional peer dependency. The
// widget itself never imports a concrete icon library.

import * as React from "react";
import type { LucideIcon } from "lucide-react";
import { AnimatePresence, MotiView } from "moti";

import {
  Box,
  Card,
  Column,
  Row,
  Text,
  useTheme,
} from "@nube/starter-ui-kit-native";

export interface ActivityItem {
  /** Stable key for animation reconciliation. */
  id: string;
  /** Lucide icon component (or any React component with a
   * compatible prop signature) shown in the left badge. */
  icon: LucideIcon;
  /** Already-localized headline. */
  title: string;
  /** Already-localized secondary line. */
  meta: string;
  /** Already-localized timestamp (e.g. "2m", "1h"). The first visible
   * item in the rotation falls back to `nowLabel` if provided. */
  time: string;
  /** CSS colour (hex/hsl/var) for the icon tint. Defaults to current
   * card foreground. */
  accent?: string;
}

export interface ActivityFeedProps {
  /** Source data. The component cycles through them on a fixed timer
   * to feel "live"; pass a longer list for slower-feeling rotation. */
  items: ActivityItem[];
  /** Section heading (already localized). */
  title: string;
  /** Right-aligned status caption (e.g. "Streaming"). */
  streamingLabel?: string;
  /** Override the timestamp on the first visible row (e.g. "now"). */
  nowLabel?: string;
  /** Number of rows visible at once. Default 5. */
  visibleCount?: number;
  /** Rotation interval in milliseconds. Default 4500. Pass `0` to
   * disable rotation (useful for tests / harnesses). */
  intervalMs?: number;
  /** Reserved for parity with the web component. Ignored on RN. */
  className?: string;
}

export function ActivityFeed({
  items,
  title,
  streamingLabel,
  nowLabel,
  visibleCount = 5,
  intervalMs = 4500,
}: ActivityFeedProps): React.ReactElement | null {
  const t = useTheme();
  const [start, setStart] = React.useState(0);

  React.useEffect(() => {
    if (!items.length || intervalMs <= 0) return;
    const handle = setInterval(
      () => setStart((s) => (s + 1) % items.length),
      intervalMs,
    );
    return () => clearInterval(handle);
  }, [items.length, intervalMs]);

  if (!items.length) return null;

  const take = Math.min(visibleCount, items.length);
  const visible = Array.from({ length: take }, (_, i) => {
    const item = items[(start + i) % items.length];
    if (!item) throw new Error("unreachable: items.length asserted above");
    return item;
  });

  return (
    <Card accessibilityRole="summary" accessibilityLabel={title}>
      <Row style={{ justifyContent: "space-between", alignItems: "center" }}>
        <Text variant="caption" color="muted">
          {title}
        </Text>
        {streamingLabel ? (
          <Text variant="caption" color="primary">
            {streamingLabel}
          </Text>
        ) : null}
      </Row>

      <Column gap={t.space(1)}>
        <AnimatePresence>
          {visible.map((item, i) => {
            const Icon = item.icon;
            const tint = item.accent ?? t.color("foreground");
            return (
              <MotiView
                key={item.id + i}
                from={{ opacity: 0, translateY: -12 }}
                animate={{ opacity: 1, translateY: 0 }}
                exit={{ opacity: 0, translateX: 30 }}
                transition={{ type: "timing", duration: t.duration("slow") || 550 }}
              >
                <Row
                  gap={t.space(4)}
                  style={{
                    alignItems: "center",
                    paddingHorizontal: t.space(3),
                    paddingVertical: t.space(3),
                    borderRadius: t.radius("2xl"),
                  }}
                >
                  <Box
                    style={{
                      width: 36,
                      height: 36,
                      alignItems: "center",
                      justifyContent: "center",
                      borderRadius: 12,
                      borderWidth: 1,
                      borderColor: t.color("border"),
                    }}
                  >
                    <Icon size={16} color={tint} />
                  </Box>
                  <Column flex={1}>
                    <Text variant="body" weight="medium" numberOfLines={1}>
                      {item.title}
                    </Text>
                    <Text variant="caption" color="muted" numberOfLines={1}>
                      {item.meta}
                    </Text>
                  </Column>
                  <Text variant="caption" color="muted">
                    {i === 0 && nowLabel ? nowLabel : item.time}
                  </Text>
                </Row>
              </MotiView>
            );
          })}
        </AnimatePresence>
      </Column>
    </Card>
  );
}
