# IoT Dashboard SDUI Implementation

Professional IoT device monitoring dashboard built with SDUI (Structured Data UI) pattern.

## Overview

This implementation provides a complete, production-ready SDUI tree for an IoT dashboard with:

- **KPI Grid**: 4-column layout showing Total Devices, Active Devices, Average Temperature, and Average Humidity
- **Trend Charts**: 24-hour sparkline visualizations for temperature and humidity trends
- **Device Table**: Real-time device status with 5 columns and 6 sample devices
- **Professional Layout**: Proper spacing, visual hierarchy, and responsive design
- **Streaming Support**: Works with builder event streaming (buffered patches pattern)

## Files

| File | Purpose | Type |
|------|---------|------|
| `src/iot_dashboard_tree.ts` | Core tree definitions (backend-compatible) | TypeScript |
| `frontend/src/lib/iot-dashboard-fixture.ts` | Frontend builder fixture integration | TypeScript (React) |
| `IoT_DASHBOARD_EXAMPLE.md` | Comprehensive usage guide and examples | Documentation |
| `IOT_DASHBOARD_INTEGRATION.md` | Step-by-step integration instructions | Documentation |
| `IOT_DASHBOARD_README.md` | This file | Documentation |

## Quick Start

### Option 1: Frontend Integration (Fastest)

1. Open `frontend/src/lib/builder-fixture.ts`

2. Add import:
```typescript
import { iotDashboardScript } from './iot-dashboard-fixture';
```

3. Add to scripts object:
```typescript
scripts: {
  sales,
  dashboard: sales,
  iot: iotDashboardScript(),    // Add this line
  onboard: onboardScript(),
  report: reportScript(),
  default: defaultScript(),
}
```

4. Access via page builder with prompt: `"iot"` or `"iot dashboard"`

### Option 2: Backend Use

```typescript
// In src/builder_stream.rs or equivalent
use crate::iot_dashboard_tree::getIoTDashboardTree;

let tree = getIoTDashboardTree();
```

## Component Breakdown

### 1. Header Section
```
Title: "IoT Dashboard"
Subtitle: "Real-time monitoring and analytics for connected IoT devices"
```

### 2. KPI Grid (4 Cards in Single Row)
```
┌──────────────┬──────────────┬──────────────┬──────────────┐
│ Total        │ Active       │ Avg          │ Avg          │
│ Devices      │ Devices      │ Temperature  │ Humidity     │
├──────────────┼──────────────┼──────────────┼──────────────┤
│ 24           │ 18           │ 72°F         │ 45%          │
│ → Online     │ ↑ 75% util   │ → Stable     │ ↓ Optimal    │
└──────────────┴──────────────┴──────────────┴──────────────┘
```

**Features:**
- Large, readable values
- Trend indicators (↑↓→) with colors
- Subtle shadows and hover effects
- Professional card styling

### 3. Trends Section (Sparkline Charts)
```
┌──────────────────────────┬──────────────────────────┐
│ Temperature Trend        │ Humidity Trend           │
│                          │                          │
│ ▁▂▃▄▅▆▇█▆▇▆▅▄▃▂▁        │ ▄▅▆▇█▆▅▄▃▂▃▄▅▆▇█       │
│ Unit: °F · Last 24 hrs   │ Unit: % · Last 24 hrs    │
└──────────────────────────┴──────────────────────────┘
```

**Features:**
- ASCII sparklines for lightweight rendering
- Time range indicators
- Unit labels
- 2-column responsive layout

### 4. Device Status Table
```
Device Name              │ Status  │ Temperature    │ Humidity │ Last Updated
───────────────────────────────────────────────────────────────────────────
Office Climate - Zone A  │ ● Online│ 72°F / 22.2°C  │ 42%      │ 2 mins ago
Server Room Monitor      │ ● Online│ 68°F / 20.0°C  │ 35%      │ 1 min ago
Warehouse Temperature    │ ● Online│ 75°F / 23.9°C  │ 55%      │ 3 mins ago
Office Climate - Zone B  │ ● Online│ 71°F / 21.7°C  │ 48%      │ 2 mins ago
Plant Storage Room       │ ● Online│ 70°F / 21.1°C  │ 60%      │ 1 min ago
Parking Lot Sensor       │ ● Offline│ N/A           │ N/A      │ 47 mins ago
```

**Features:**
- 5 columns with proper header labels
- 6 sample devices (5 online, 1 offline)
- Real-time status indicators
- Temperature in Fahrenheit and Celsius
- Relative timestamps

### 5. Footer
```
Last dashboard refresh: Just now · Data updates every 30 seconds
```

## Visual Hierarchy

```
1. Page Background (min-h-screen)
   │
   ├─ H1 Title (text-3xl font-bold)
   │  └─ Subtitle (text-sm muted)
   │
   ├─ Section Label (uppercase, tracking-wider)
   │  └─ 4-Column KPI Grid
   │     └─ KPI Cards (3xl values)
   │
   ├─ Section Label (uppercase, tracking-wider)
   │  └─ 2-Column Trends Grid
   │     └─ Sparkline Cards (lg font-mono)
   │
   ├─ Table Section
   │  ├─ H3 Heading (text-lg font-semibold)
   │  ├─ Description (text-sm muted)
   │  └─ Device Table (standard table)
   │
   └─ Footer
      └─ Muted text (text-xs)
```

## Styling System

### Tailwind Classes

**Spacing**
- `gap-lg` — 24px (main sections)
- `gap-sm` — 8px (card internals, grid items)
- `gap-xs` — 4px (label/value pairs)

**Border & Shadow**
- `border border-border/40` — Subtle borders
- `shadow-sm hover:shadow-md` — Interactive shadows
- `rounded-lg` / `rounded-xl` — Border radius

**Typography**
- H1: `text-3xl font-bold tracking-tight`
- H3: `text-lg font-semibold`
- Labels: `text-xs font-medium uppercase tracking-wider`
- Values: `text-3xl font-bold`
- Sparklines: `font-mono text-lg tracking-tight`

**Colors**
- Trend Up: `text-emerald-600` (↑)
- Trend Down: `text-red-600` (↓)
- Trend Stable: `text-slate-400` (→)
- Muted: `text-muted-foreground` (labels, footers)
- Primary: `text-primary/80` (sparkline data)

## Event Streaming

The builder emits events in this pattern:

```
1. status: "thinking"
2. patch: root.devices-table        (buffered)
3. patch: root.kpis                 (buffered)
4. full-render: root skeleton        (drains buffer)
5. status: "writing", message: "Loading live data…"
6. patch: root.trends               (post-parent, immediate)
7. status: "done", message: "Dashboard ready"
```

**Timing (with DELAY_MS = 80ms)**
- Total duration: <2 seconds
- Buffer visible: 0-200ms ("buffered patches" badge)
- No layout shift: Full skeleton before patches

## Tree Size & Performance

- **Complete tree**: ~15-18 KB (with sample data)
- **Streaming events**: 7 events
- **Render time**: <2s from prompt to fully rendered dashboard
- **Grid responsiveness**: Native CSS Grid (no media queries)
- **Sample data**: 6 devices embedded (replace with API calls)

## Customization

### Add More KPIs
Edit `createKpiGridSkeleton()` to add 5th/6th KPI:
```typescript
cols: 6,  // Change from 4 to 6
children: [
  // ... existing 4 ...
  createKpiCard("root.kpis.uptime", "System Uptime", "99.9%", "7 days", "up"),
]
```

### Change Trend Charts
Modify ASCII sparklines in `createSparklineCard()`:
```typescript
// Example: More volatile temperature
"▂▄▆█▇▅▄▃▂▃▅▇█▆▄▂"
```

### Update Device Data
Edit device rows in `iotDeviceTable()`:
```typescript
{
  id: "device-007",
  slots: {
    name: "New Device Name",
    status: "● Online",
    temperature: "70°F / 21.1°C",
    humidity: "50%",
    lastUpdated: "Just now",
  },
}
```

### Adjust Colors
Update trend color mapping in `createKpiCard()`:
```typescript
const trendColor =
  trend === "up"
    ? "text-green-500"     // Change color
    : trend === "down"
    ? "text-orange-600"    // Change color
    : "text-gray-500";     // Change color
```

## Real-World Integration

To connect actual IoT data:

```typescript
// 1. Fetch devices from API
const devices = await api.getDevices();

// 2. Calculate aggregates
const totalDevices = devices.length;
const activeDevices = devices.filter(d => d.online).length;
const avgTemp = devices.reduce((sum, d) => sum + d.temperature, 0) / devices.length;
const avgHumidity = devices.reduce((sum, d) => sum + d.humidity, 0) / devices.length;

// 3. Map to KPI cards
const kpiGrid = iotKpiGrid();
kpiGrid.children = [
  createKpiCard("root.kpis.total", "Total Devices", totalDevices.toString()),
  createKpiCard("root.kpis.active", "Active Devices", activeDevices.toString()),
  createKpiCard("root.kpis.temp", "Avg Temperature", `${Math.round(avgTemp)}°F`),
  createKpiCard("root.kpis.humidity", "Avg Humidity", `${Math.round(avgHumidity)}%`),
];

// 4. Map device list to table rows
const rows = devices.map(device => ({
  id: device.id,
  slots: {
    name: device.name,
    status: `${device.online ? "●" : "○"} ${device.online ? "Online" : "Offline"}`,
    temperature: `${device.temperature}°F / ${toC(device.temperature)}°C`,
    humidity: `${device.humidity}%`,
    lastUpdated: timeAgo(device.lastSeen),
  },
}));
```

## Browser Compatibility

- Modern browsers with CSS Grid support (Chrome 57+, Firefox 52+, Safari 10.1+, Edge 15+)
- Requires SDUI renderer with:
  - `stack` component (flex container)
  - `grid` component (CSS Grid)
  - `text` component (text rendering)
  - `heading` component (h1-h6)
  - `table` component (data table)

## Testing

Run the page builder and test:

```
1. Load `/pages/new`
2. Type "iot" as the prompt
3. Verify within 2 seconds:
   - Title and subtitle appear
   - 4 KPI cards render with values
   - 2 trend sparklines display
   - Device table shows 6 rows
   - Footer text appears
4. Check responsive design on mobile (grid adjusts)
5. Verify hover effects on cards
6. Confirm no console errors
7. Check that save/edit round-trips work
```

## Files Summary

```
examples/flow-agent/
├── src/
│   └── iot_dashboard_tree.ts          (Core tree definitions)
├── frontend/src/lib/
│   └── iot-dashboard-fixture.ts       (React/Frontend integration)
├── IoT_DASHBOARD_README.md             (This file)
├── IoT_DASHBOARD_EXAMPLE.md            (Usage examples)
└── IOT_DASHBOARD_INTEGRATION.md        (Integration guide)
```

## Next Steps

1. **Immediate**: Add to builder fixture for testing
2. **Short-term**: Connect real device API
3. **Medium-term**: Add device filtering/search
4. **Long-term**: Real-time WebSocket updates, alert thresholds

## Support

- See `IoT_DASHBOARD_EXAMPLE.md` for detailed examples
- See `IOT_DASHBOARD_INTEGRATION.md` for integration steps
- Check `builder-fixture.ts` for reference patterns
- Review `PageBuilder.tsx` for rendering context
