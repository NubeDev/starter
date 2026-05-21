# IoT Dashboard SDUI Tree Example

This document demonstrates how to use the IoT Dashboard SDUI tree for building professional IoT monitoring interfaces.

## Overview

The IoT Dashboard provides a complete, production-ready SDUI tree for displaying:
- Real-time IoT device metrics
- Key Performance Indicators (KPIs)
- Device status and environmental readings
- Temperature and humidity trends
- Professional visual hierarchy and spacing

## Tree Structure

### Location
```
src/iot_dashboard_tree.ts
```

### Components

#### 1. KPI Grid (4-column layout)
```
├── Total Devices: 24
├── Active Devices: 18  
├── Avg Temperature: 72°F
└── Avg Humidity: 45%
```

Each KPI card includes:
- Clean, readable label (uppercase, muted tone)
- Large, bold metric value
- Optional trend indicator (↑/↓/→)
- Subtle subtitle with additional context
- Hover shadow effect for interactivity

#### 2. 24-Hour Trends (2-column layout)
```
├── Temperature Trend: ▁▂▃▄▅▆▇█▆▇▆▅▄▃▂▁
└── Humidity Trend: ▄▅▆▇█▆▅▄▃▂▃▄▅▆▇█
```

Sparkline cards showing 24-hour historical data with:
- Section title and unit
- ASCII sparkline visualization
- Time range indicator

#### 3. Device Status Table
```
Columns:
├── Device Name
├── Status (● Online / ● Offline)
├── Temperature (°F / °C)
├── Humidity (%)
└── Last Updated
```

Sample data includes 6 devices:
- 5 online devices with active readings
- 1 offline device showing historical state

### Layout Hierarchy

```
root (main container)
├── header
│   ├── h1: "IoT Dashboard"
│   └── subtitle: "Real-time monitoring..."
├── kpi-section
│   ├── "Key Metrics" label
│   └── KPI Grid (4 columns)
├── trends-section
│   ├── "24-Hour Trends" label
│   └── Trends Grid (2 columns)
├── table-section
│   ├── "Connected Devices" heading
│   ├── description text
│   └── Device Status Table
└── footer
    └── "Last dashboard refresh: Just now..."
```

## Usage

### Basic Integration

```typescript
import { getIoTDashboardTree } from './src/iot_dashboard_tree';

// Get the complete dashboard tree
const dashboardTree = getIoTDashboardTree();

// Use in page builder
const pageData = {
  id: 'iot-dashboard-001',
  name: 'IoT Facility Monitor',
  tree: { root: dashboardTree },
  createdAt: new Date().toISOString(),
};
```

### With Builder Streaming

```typescript
import { getIoTDashboardScript } from './src/iot_dashboard_tree';

// Get the event sequence for streaming
const builderEvents = getIoTDashboardScript();

// Events sequence:
// 1. status: "thinking"
// 2. patch: Device table
// 3. full-render: Complete dashboard skeleton
// 4. status: "writing"
// 5. patch: Trends section refinement
// 6. status: "done"
```

### In Builder Fixture

To add the IoT Dashboard to the page builder fixture (in `frontend/src/lib/builder-fixture.ts`):

```typescript
import { getIoTDashboardTree, getIoTDashboardScript } from './iot_dashboard_tree';

export function createFlowAgentBuilderFixture(): BuilderAdapter {
  const iotScript = getIoTDashboardScript();
  
  return createFixtureBuilderAdapter({
    delayMs: FIXTURE_DELAY_MS,
    scripts: {
      sales,
      dashboard: sales,
      iot: iotScript,           // Add IoT Dashboard
      onboard: onboardScript(),
      report: reportScript(),
      default: defaultScript(),
    },
  });
}
```

Then prompt the builder with: "iot" or "iot dashboard"

## Design Details

### Styling Classes

- **Card styling**: `rounded-lg border border-border/40 bg-card p-4 shadow-sm hover:shadow-md`
- **Heading**: `text-3xl font-bold tracking-tight text-foreground`
- **Labels**: `text-xs font-medium uppercase tracking-wider`
- **KPI values**: `text-3xl font-bold`
- **Trend indicators**: `text-lg font-semibold` (with color variants)

### Spacing Scale

- `gap: "lg"` — Main sections (header, KPIs, trends, table, footer)
- `gap: "sm"` — Card internals and grid spacing
- `gap: "xs"` — KPI card label/value spacing

### Color/Tone

- **Muted text**: Section labels, subtitles, footers
- **Primary**: KPI values, trends
- **Trend colors**: 
  - Green (`text-emerald-600`) for uptrends ↑
  - Red (`text-red-600`) for downtrends ↓
  - Gray (`text-slate-400`) for stable →

### Responsive Grid

- **KPI Grid**: 4 columns → naturally responsive
- **Trends Grid**: 2 columns → responsive on smaller screens
- All components use gap spacing for mobile-friendly layouts

## Sample Device Data

The table includes realistic IoT device examples:

| Device Name | Status | Temperature | Humidity | Last Updated |
|---|---|---|---|---|
| Office Climate - Zone A | ● Online | 72°F / 22.2°C | 42% | 2 mins ago |
| Server Room Monitor | ● Online | 68°F / 20.0°C | 35% | 1 min ago |
| Warehouse Temperature | ● Online | 75°F / 23.9°C | 55% | 3 mins ago |
| Office Climate - Zone B | ● Online | 71°F / 21.7°C | 48% | 2 mins ago |
| Plant Storage Room | ● Online | 70°F / 21.1°C | 60% | 1 min ago |
| Parking Lot Sensor | ● Offline | N/A | N/A | 47 mins ago |

## Customization

### Adding More KPIs

Edit the `createKpiGridSkeleton()` function:

```typescript
function createKpiGridSkeleton(): UiComponent {
  return {
    type: "grid",
    id: "root.kpis",
    cols: 4,  // Change to 6 for 6 columns
    gap: "sm",
    children: [
      // Add more KPI cards here
      createKpiCard("root.kpis.uptime", "System Uptime", "99.9%", "7 days", "up"),
    ],
  };
}
```

### Modifying KPI Card Styling

Update the `createKpiCard()` function for different card sizes, colors, or animations:

```typescript
// Example: Make cards larger with more prominent values
style: {
  className: "rounded-xl border border-border/40 bg-gradient-to-br from-card to-card/80 p-6 shadow-lg",
}
```

### Adding Real Data Binding

Replace hardcoded values with dynamic data:

```typescript
function createKpiCard(
  id: string,
  label: string,
  value: string,
  data: { current: number; max: number; unit: string }
): UiComponent {
  return {
    // ... card structure ...
    children: [
      // Use data.current, data.max for dynamic rendering
    ],
  };
}
```

## Integration Points

### With Page Builder

1. Save dashboard tree to localStorage or backend
2. Load tree in PageView.tsx with Renderer
3. Wire save button to persist {id, name, tree, createdAt}

### With Real Data

1. Replace sample device rows with API response data
2. Map API device objects to table row format
3. Add real-time WebSocket updates for "Last Updated" timestamps

### With Actions

Add interactive handlers:

```typescript
{
  type: "table",
  id: "root.devices-table",
  columns: [...],
  rows: [...],
  onRowClick: { handler: "select-device", args: { deviceId: "device-001" } }
}
```

## Performance Notes

- Grid layout automatically handles responsive breakpoints
- Sparkline ASCII art provides lightweight trend visualization
- Table pagination recommended for 100+ devices
- Consider virtual scrolling for large device lists

## Testing

The tree can be tested by:

1. Accessing `/pages/new` in the page builder
2. Typing "iot" or "iot dashboard" to trigger the script
3. Verifying all sections render with proper spacing
4. Checking mobile responsiveness (grid columns adjust)
5. Confirming table displays all sample device rows

## Future Enhancements

- [ ] Real-time WebSocket updates
- [ ] Device status filtering/search
- [ ] Threshold-based alerts on KPI cards
- [ ] Interactive trend charts (replace sparklines)
- [ ] Device detail drilldown modals
- [ ] Export dashboard as PDF/CSV
- [ ] Custom date range selection for trends
