/**
 * IoT Dashboard Builder Fixture
 *
 * Comprehensive IoT dashboard UI tree with:
 * - Title and header information
 * - Key Performance Indicators (devices online, total data points, active alerts, uptime %)
 * - Time-series charts for temperature, humidity, and power consumption
 * - Device status table with name, status, and last reading information
 * - Clean, organized layout with proper spacing and visual hierarchy
 *
 * Integration:
 * 1. Import this file's IoT functions into builder-fixture.ts
 * 2. Add `iot: iotDashboardScript()` to the scripts object
 * 3. Users can now type "iot" or "iot dashboard" to trigger the template
 */

import type { BuilderEvent, UiComponent } from "@nube/starter-ui-ai-builder";
import { fixtureTree } from "@nube/starter-ui-ai-builder";

/**
 * Creates a KPI card component for dashboard metrics
 */
function createKpiCard(
  id: string,
  label: string,
  value: string,
  subtitle?: string,
  trend?: "up" | "down" | "stable"
): UiComponent {
  const trendIndicator =
    trend === "up" ? "↑" : trend === "down" ? "↓" : "→";
  const trendColor =
    trend === "up"
      ? "text-emerald-600"
      : trend === "down"
        ? "text-red-600"
        : "text-slate-400";

  return {
    type: "stack",
    id,
    gap: "xs",
    style: {
      className:
        "rounded-lg border border-border/40 bg-card p-4 shadow-sm hover:shadow-md transition-shadow",
    },
    children: [
      {
        type: "text",
        value: label,
        tone: "muted",
        style: { className: "text-xs font-medium uppercase tracking-wider" },
      },
      {
        type: "stack",
        gap: "xs",
        style: { className: "flex items-baseline justify-between" },
        children: [
          {
            type: "text",
            value,
            style: { className: "text-3xl font-bold" },
          },
          {
            type: "text",
            value: trendIndicator,
            style: { className: `text-lg font-semibold ${trendColor}` },
          },
        ],
      } as UiComponent,
      ...(subtitle
        ? [
            {
              type: "text",
              value: subtitle,
              tone: "muted",
              style: { className: "text-xs" },
            } as UiComponent,
          ]
        : []),
    ],
  } as UiComponent;
}

/**
 * Creates a sparkline card for trend visualization
 */
function createSparklineCard(
  id: string,
  title: string,
  data: string,
  unit: string
): UiComponent {
  return {
    type: "stack",
    id,
    gap: "sm",
    style: {
      className:
        "rounded-lg border border-border/40 bg-card p-4 shadow-sm",
    },
    children: [
      {
        type: "text",
        value: title,
        style: { className: "text-sm font-semibold" },
      },
      {
        type: "text",
        value: data,
        style: {
          className:
            "font-mono text-lg tracking-tight text-primary/80",
        },
      },
      {
        type: "text",
        value: `Unit: ${unit} · Last 24 hours`,
        tone: "muted",
        style: { className: "text-xs" },
      },
    ],
  } as UiComponent;
}

/**
 * Creates the KPI grid section with devices online, data points, alerts, and uptime
 */
function iotKpiGrid(): UiComponent {
  return {
    type: "grid",
    id: "root.kpis",
    cols: 4,
    gap: "sm",
    children: [
      createKpiCard(
        "root.kpis.online",
        "Devices Online",
        "23",
        "of 24 connected",
        "up"
      ),
      createKpiCard(
        "root.kpis.datapoints",
        "Total Data Points",
        "1.2M",
        "Last 24 hours",
        "up"
      ),
      createKpiCard(
        "root.kpis.alerts",
        "Active Alerts",
        "2",
        "1 critical, 1 warning",
        "down"
      ),
      createKpiCard(
        "root.kpis.uptime",
        "System Uptime",
        "99.8%",
        "Next maintenance: 3d",
        "stable"
      ),
    ],
  } as UiComponent;
}

/**
 * Creates the trends section with temperature, humidity, and power charts
 */
function iotTrends(): UiComponent {
  return {
    type: "grid",
    id: "root.trends",
    cols: 3,
    gap: "sm",
    children: [
      createSparklineCard(
        "root.trends.temp",
        "Temperature Over Time",
        "▁▂▃▄▅▆▇█▆▇▆▅▄▃▂▁▂▃▄▅",
        "°F (68-78°F range)"
      ),
      createSparklineCard(
        "root.trends.humidity",
        "Humidity Over Time",
        "▄▅▆▇█▆▅▄▃▂▃▄▅▆▇█▇▆▅▄",
        "% (30-65% range)"
      ),
      createSparklineCard(
        "root.trends.power",
        "Power Consumption",
        "▂▃▄▆█▆▅▄▃▂▃▄▅▆▇█▆▇▆▅",
        "kW (2-8 kW range)"
      ),
    ],
  } as UiComponent;
}

/**
 * Creates the device status table with name, status, and last reading information
 */
function iotDeviceTable(): UiComponent {
  return {
    type: "stack",
    id: "root.table-section",
    gap: "sm",
    style: {
      className:
        "rounded-lg border border-border/40 bg-card p-4 shadow-sm",
    },
    children: [
      {
        type: "heading",
        value: "Device Status Monitor",
        level: 3,
        style: { className: "text-lg font-semibold" },
      },
      {
        type: "text",
        value: "Real-time status and readings from all connected IoT devices",
        tone: "muted",
        style: { className: "text-sm mb-2" },
      },
      {
        type: "table",
        id: "root.devices-table",
        columns: [
          { key: "name", label: "Device Name" },
          { key: "status", label: "Status" },
          { key: "temperature", label: "Last Temperature" },
          { key: "humidity", label: "Last Humidity" },
          { key: "power", label: "Power Draw" },
          { key: "lastUpdated", label: "Last Reading" },
        ],
        rows: [
          {
            id: "device-001",
            slots: {
              name: "Office Climate - Zone A",
              status: "● Online",
              temperature: "72°F",
              humidity: "42%",
              power: "1.2 kW",
              lastUpdated: "2 mins ago",
            },
          },
          {
            id: "device-002",
            slots: {
              name: "Server Room Monitor",
              status: "● Online",
              temperature: "68°F",
              humidity: "35%",
              power: "3.8 kW",
              lastUpdated: "1 min ago",
            },
          },
          {
            id: "device-003",
            slots: {
              name: "Warehouse Temperature",
              status: "● Online",
              temperature: "75°F",
              humidity: "55%",
              power: "2.1 kW",
              lastUpdated: "3 mins ago",
            },
          },
          {
            id: "device-004",
            slots: {
              name: "Office Climate - Zone B",
              status: "● Online",
              temperature: "71°F",
              humidity: "48%",
              power: "1.1 kW",
              lastUpdated: "2 mins ago",
            },
          },
          {
            id: "device-005",
            slots: {
              name: "Plant Storage Room",
              status: "● Online",
              temperature: "70°F",
              humidity: "60%",
              power: "0.8 kW",
              lastUpdated: "1 min ago",
            },
          },
          {
            id: "device-006",
            slots: {
              name: "Parking Lot Sensor",
              status: "● Offline",
              temperature: "N/A",
              humidity: "N/A",
              power: "0.0 kW",
              lastUpdated: "47 mins ago",
            },
          },
        ],
      } as UiComponent,
    ],
  } as UiComponent;
}

/**
 * Creates the dashboard skeleton with complete layout
 *
 * Structure:
 * 1. Header (title + subtitle)
 * 2. KPI Grid (devices online, data points, alerts, uptime %)
 * 3. Trends Section (temperature, humidity, power charts)
 * 4. Device Status Table (name, status, readings)
 * 5. Footer (refresh info)
 */
function iotDashboardSkeleton(): UiComponent {
  return {
    type: "stack",
    id: "root",
    gap: "lg",
    style: {
      className: "p-6 bg-background min-h-screen",
    },
    children: [
      {
        type: "stack",
        id: "root.header",
        gap: "xs",
        style: { className: "mb-2" },
        children: [
          {
            type: "heading",
            value: "IoT Dashboard",
            level: 1,
            style: {
              className:
                "text-3xl font-bold tracking-tight text-foreground",
            },
          },
          {
            type: "text",
            value: "Real-time monitoring and analytics for connected IoT devices",
            tone: "muted",
            style: { className: "text-sm" },
          },
        ],
      } as UiComponent,
      {
        type: "stack",
        id: "root.kpi-section",
        gap: "sm",
        children: [
          {
            type: "text",
            value: "Key Performance Indicators",
            style: { className: "text-sm font-semibold uppercase tracking-wider text-muted-foreground" },
          },
          iotKpiGrid(),
        ],
      } as UiComponent,
      {
        type: "stack",
        id: "root.trends-section",
        gap: "sm",
        children: [
          {
            type: "text",
            value: "24-Hour Trends",
            style: { className: "text-sm font-semibold uppercase tracking-wider text-muted-foreground" },
          },
          {
            type: "text",
            value: "Temperature, Humidity, and Power Consumption Charts",
            tone: "muted",
            style: { className: "text-xs mb-2" },
          },
          iotTrends(),
        ],
      } as UiComponent,
      iotDeviceTable(),
      {
        type: "stack",
        id: "root.footer",
        gap: "xs",
        style: {
          className:
            "rounded-lg border border-border/40 bg-muted/20 p-4 mt-4 text-xs text-muted-foreground",
        },
        children: [
          {
            type: "text",
            value: "Last dashboard refresh: Just now · Data updates every 30 seconds · 23 of 24 devices connected",
          },
        ],
      } as UiComponent,
    ],
  } as UiComponent;
}

/**
 * IoT Dashboard builder event script
 *
 * Follows the buffered-patch pattern for optimal streaming:
 * 1. Emit status "thinking" — start processing
 * 2. Emit patches for device table (pre-parent) — buffered in R1
 * 3. Emit patches for KPIs (pre-parent) — second buffered patch
 * 4. Emit full skeleton render — drains buffered patches in one tick
 * 5. Emit status "writing" — transition to refinement
 * 6. Emit patches for trends (post-parent) — applied immediately
 * 7. Emit status "done" — complete
 *
 * This pattern ensures the dashboard appears quickly with data flowing in,
 * demonstrating the streaming capability while staying within 2s budget.
 */
export function iotDashboardScript(): BuilderEvent[] {
  return [
    { type: "status", phase: "thinking" },
    // Pre-parent patches (buffered in R1 buffer — up to 2 will be held)
    {
      type: "patch",
      targetComponentId: "root.devices-table",
      subtree: iotDeviceTable(),
    },
    {
      type: "patch",
      targetComponentId: "root.kpis",
      subtree: iotKpiGrid(),
    },
    // Parent skeleton — drains both buffered patches in one tick
    { type: "full-render", tree: fixtureTree(iotDashboardSkeleton()) },
    { type: "status", phase: "writing", message: "Loading live data…" },
    // Post-parent patches (applied immediately — not buffered)
    {
      type: "patch",
      targetComponentId: "root.trends",
      subtree: iotTrends(),
    },
    { type: "status", phase: "done", message: "Dashboard ready" },
  ];
}

/**
 * Export helper to get just the dashboard tree (without streaming events)
 * Useful for direct rendering without the builder/streaming flow
 */
export function iotDashboardTree(): UiComponent {
  return iotDashboardSkeleton();
}

/**
 * Export all helper functions for custom dashboard variants
 * Allows downstream integrations to remix and extend the dashboard components
 */
export {
  createKpiCard,
  createSparklineCard,
  iotKpiGrid,
  iotTrends,
  iotDeviceTable,
  iotDashboardSkeleton,
};
