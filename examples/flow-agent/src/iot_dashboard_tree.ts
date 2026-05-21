/**
 * SDUI Tree for IoT Dashboard
 *
 * A professional IoT dashboard showcasing:
 * - Dashboard title and description
 * - KPI grid with key metrics (Total Devices, Active Devices, Avg Temperature, Avg Humidity)
 * - Device status table with real-time updates
 * - Temperature and humidity trend sparklines
 * - Professional layout with proper spacing and visual hierarchy
 */

import type { UiComponent } from "@nube/starter-sdui-react";

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
 * Creates a simple sparkline chart placeholder
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
 * Creates the KPI grid section skeleton
 */
function createKpiGridSkeleton(): UiComponent {
  return {
    type: "grid",
    id: "root.kpis",
    cols: 4,
    gap: "sm",
    children: [
      createKpiCard("root.kpis.total", "Total Devices", "24", "Online", "stable"),
      createKpiCard(
        "root.kpis.active",
        "Active Devices",
        "18",
        "75% utilization",
        "up"
      ),
      createKpiCard(
        "root.kpis.temp",
        "Avg Temperature",
        "72°F",
        "21.1°C",
        "stable"
      ),
      createKpiCard("root.kpis.humidity", "Avg Humidity", "45%", "Optimal", "down"),
    ],
  } as UiComponent;
}

/**
 * Creates the trends section with sparklines
 */
function createTrendsSkeleton(): UiComponent {
  return {
    type: "grid",
    id: "root.trends",
    cols: 2,
    gap: "sm",
    children: [
      createSparklineCard(
        "root.trends.temp",
        "Temperature Trend",
        "▁▂▃▄▅▆▇█▆▇▆▅▄▃▂▁",
        "°F"
      ),
      createSparklineCard(
        "root.trends.humidity",
        "Humidity Trend",
        "▄▅▆▇█▆▅▄▃▂▃▄▅▆▇█",
        "%"
      ),
    ],
  } as UiComponent;
}

/**
 * Creates the device table section
 */
function createDeviceTableSkeleton(): UiComponent {
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
        value: "Connected Devices",
        level: 3,
        style: { className: "text-lg font-semibold" },
      },
      {
        type: "text",
        value: "Real-time status and environmental readings from all active IoT devices",
        tone: "muted",
        style: { className: "text-sm mb-2" },
      },
      {
        type: "table",
        id: "root.devices-table",
        columns: [
          { key: "name", label: "Device Name" },
          { key: "status", label: "Status" },
          { key: "temperature", label: "Temperature" },
          { key: "humidity", label: "Humidity" },
          { key: "lastUpdated", label: "Last Updated" },
        ],
        rows: [
          {
            id: "device-001",
            slots: {
              name: "Office Climate - Zone A",
              status: "● Online",
              temperature: "72°F / 22.2°C",
              humidity: "42%",
              lastUpdated: "2 mins ago",
            },
          },
          {
            id: "device-002",
            slots: {
              name: "Server Room Monitor",
              status: "● Online",
              temperature: "68°F / 20.0°C",
              humidity: "35%",
              lastUpdated: "1 min ago",
            },
          },
          {
            id: "device-003",
            slots: {
              name: "Warehouse Temperature",
              status: "● Online",
              temperature: "75°F / 23.9°C",
              humidity: "55%",
              lastUpdated: "3 mins ago",
            },
          },
          {
            id: "device-004",
            slots: {
              name: "Office Climate - Zone B",
              status: "● Online",
              temperature: "71°F / 21.7°C",
              humidity: "48%",
              lastUpdated: "2 mins ago",
            },
          },
          {
            id: "device-005",
            slots: {
              name: "Plant Storage Room",
              status: "● Online",
              temperature: "70°F / 21.1°C",
              humidity: "60%",
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
              lastUpdated: "47 mins ago",
            },
          },
        ],
      } as UiComponent,
    ],
  } as UiComponent;
}

/**
 * Creates the main dashboard root skeleton with title and layout
 */
function createDashboardSkeleton(): UiComponent {
  return {
    type: "stack",
    id: "root",
    gap: "lg",
    style: {
      className: "p-6 bg-background min-h-screen",
    },
    children: [
      // Header section
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

      // KPI Grid section
      {
        type: "stack",
        id: "root.kpi-section",
        gap: "sm",
        children: [
          {
            type: "text",
            value: "Key Metrics",
            style: { className: "text-sm font-semibold uppercase tracking-wider text-muted-foreground" },
          },
          createKpiGridSkeleton(),
        ],
      } as UiComponent,

      // Trends section
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
          createTrendsSkeleton(),
        ],
      } as UiComponent,

      // Devices table section
      createDeviceTableSkeleton(),

      // Footer
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
            value: "Last dashboard refresh: Just now · Data updates every 30 seconds",
          },
        ],
      } as UiComponent,
    ],
  } as UiComponent;
}

/**
 * Exports the IoT Dashboard tree for use in the page builder
 */
export function getIoTDashboardTree(): UiComponent {
  return createDashboardSkeleton();
}

/**
 * Example BuilderEvent[] script that streams the IoT Dashboard
 * following the builder pattern (status → patches → full-render)
 */
export function getIoTDashboardScript() {
  return [
    { type: "status", phase: "thinking" },
    // Stream the table first as a patch
    {
      type: "patch",
      targetComponentId: "root.devices-table",
      subtree: createDeviceTableSkeleton(),
    },
    // Render the full skeleton which will be filled by patches
    { type: "full-render", tree: { root: createDashboardSkeleton() } },
    { type: "status", phase: "writing", message: "Rendering dashboard…" },
    // Refine with trends
    {
      type: "patch",
      targetComponentId: "root.trends-section",
      subtree: createTrendsSkeleton(),
    },
    { type: "status", phase: "done", message: "Dashboard ready" },
  ];
}
