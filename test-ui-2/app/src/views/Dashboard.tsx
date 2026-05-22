import { useEffect, useMemo, useRef, useState } from 'react'
import { motion } from 'motion/react'
import {
  Activity,
  AlertTriangle,
  Cpu,
  Droplets,
  Gauge,
  Power,
  Thermometer,
  Wifi,
  Zap,
} from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge, StatusDot } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { KpiCard } from '@/components/kpi-card'
import { Sparkline } from '@/components/sparkline'
import { DeviceList, type Device } from '@/components/device-list'
import { AlertFeed, type Alert } from '@/components/alert-feed'
import { GradientBars } from '@/components/gradient-bars'
import { RadialGauge } from '@/components/radial-gauge'
import { Heatmap } from '@/components/heatmap'
import { SiteGrid } from '@/components/site-grid'
import { SidebarTrigger } from '@/components/sidebar'

const SEED_DEVICES: Device[] = [
  { id: 'NODE-001', name: 'Boiler Room Sensor', location: 'Plant A · Lvl 1', status: 'online', load: 42, battery: 92 },
  { id: 'NODE-002', name: 'HVAC Roof Unit', location: 'Plant A · Roof', status: 'online', load: 71, battery: 88 },
  { id: 'NODE-003', name: 'Coldroom Probe', location: 'Plant B · Lvl 0', status: 'degraded', load: 33, battery: 41 },
  { id: 'NODE-004', name: 'Gateway Edge-1', location: 'Plant A · DC', status: 'online', load: 19, battery: 100 },
  { id: 'NODE-005', name: 'Outdoor Mesh #7', location: 'Yard · East', status: 'offline', load: 0, battery: 12 },
  { id: 'NODE-006', name: 'Pump Vibration', location: 'Plant B · Lvl 1', status: 'online', load: 56, battery: 77 },
]

const ALERT_TEMPLATES: Omit<Alert, 'id' | 'at'>[] = [
  { level: 'warn', device: 'NODE-003', message: 'Coldroom temperature drifting above setpoint (+1.4°C).' },
  { level: 'info', device: 'NODE-004', message: 'Firmware sync completed (v2.14.1).' },
  { level: 'danger', device: 'NODE-005', message: 'Heartbeat lost. Last seen 4 minutes ago.' },
  { level: 'info', device: 'NODE-001', message: 'Pressure stabilised within band.' },
  { level: 'warn', device: 'NODE-006', message: 'Vibration RMS above baseline (+18%).' },
]

const clamp = (n: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, n))

const nowHMS = () => {
  const d = new Date()
  const p = (n: number) => n.toString().padStart(2, '0')
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}

function buildHeatmap(): number[][] {
  // 7 days × 24 hours, biased toward business hours
  return Array.from({ length: 7 }, (_, d) =>
    Array.from({ length: 24 }, (_, h) => {
      const peak = Math.exp(-Math.pow((h - 13) / 4, 2))
      const weekday = d < 5 ? 1 : 0.55
      return clamp(peak * weekday * (0.7 + Math.random() * 0.5), 0.02, 1)
    }),
  )
}

export default function Dashboard() {
  const [devices, setDevices] = useState<Device[]>(SEED_DEVICES)
  const [alerts, setAlerts] = useState<Alert[]>([
    { id: crypto.randomUUID(), at: nowHMS(), ...ALERT_TEMPLATES[0] },
    { id: crypto.randomUUID(), at: nowHMS(), ...ALERT_TEMPLATES[2] },
  ])

  const [temp, setTemp] = useState(22.4)
  const [humidity, setHumidity] = useState(48)
  const [pressure, setPressure] = useState(1012)
  const [throughput, setThroughput] = useState(412)
  const [healthScore, setHealthScore] = useState(86)

  const [tempSeries, setTempSeries] = useState<number[]>(() =>
    Array.from({ length: 40 }, (_, i) => 21 + Math.sin(i / 3) + Math.random() * 0.4),
  )

  const [heatmap] = useState(buildHeatmap)
  const tick = useRef(0)

  useEffect(() => {
    const iv = setInterval(() => {
      tick.current += 1
      setTemp((v) => clamp(v + (Math.random() - 0.5) * 0.6, 18, 28))
      setHumidity((v) => clamp(v + (Math.random() - 0.5) * 2, 30, 70))
      setPressure((v) => clamp(v + (Math.random() - 0.5) * 1.5, 1000, 1025))
      setThroughput((v) => clamp(v + (Math.random() - 0.5) * 30, 280, 560))
      setHealthScore((v) => clamp(v + (Math.random() - 0.5) * 2, 60, 99))

      setTempSeries((s) => [
        ...s.slice(1),
        clamp(s[s.length - 1] + (Math.random() - 0.5) * 0.8, 18, 28),
      ])

      if (tick.current % 6 === 0) {
        setDevices((ds) =>
          ds.map((d) =>
            d.id === 'NODE-003'
              ? { ...d, load: clamp(d.load + (Math.random() - 0.5) * 10, 0, 100) }
              : d.id === 'NODE-005'
                ? { ...d, battery: clamp(d.battery - 0.1, 0, 100) }
                : { ...d, load: clamp(d.load + (Math.random() - 0.5) * 6, 0, 100) },
          ),
        )
      }

      if (tick.current % 5 === 0) {
        const tpl = ALERT_TEMPLATES[Math.floor(Math.random() * ALERT_TEMPLATES.length)]
        setAlerts((a) =>
          [{ id: crypto.randomUUID(), at: nowHMS(), ...tpl }, ...a].slice(0, 5),
        )
      }
    }, 1200)
    return () => clearInterval(iv)
  }, [])

  const online = useMemo(() => devices.filter((d) => d.status === 'online').length, [devices])
  const degraded = useMemo(() => devices.filter((d) => d.status === 'degraded').length, [devices])
  const offline = useMemo(() => devices.filter((d) => d.status === 'offline').length, [devices])

  // Synthetic grouped-bar series (per shift × per metric)
  const barLabels = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
  const barSeries = [
    { name: 'Pump A', color: 'var(--color-chart-1)', data: [62, 78, 71, 84, 90, 55, 42] },
    { name: 'Pump B', color: 'var(--color-chart-2)', data: [40, 52, 48, 60, 71, 38, 30] },
    { name: 'HVAC', color: 'var(--color-chart-3)', data: [88, 92, 85, 90, 95, 70, 60] },
    { name: 'Compr.', color: 'var(--color-chart-4)', data: [30, 36, 41, 35, 48, 22, 18] },
  ]

  return (
    <div className="mx-auto max-w-[1500px] px-4 py-6 sm:px-6 lg:px-8">
          {/* Header */}
          <header className="mb-6 flex flex-wrap items-center justify-between gap-3">
            <motion.div
              initial={{ opacity: 0, y: -6 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.3 }}
              className="flex items-center gap-3"
            >
              <SidebarTrigger />
              <div className="flex size-10 items-center justify-center rounded-lg bg-[var(--color-primary)] text-[var(--color-primary-foreground)] shadow-[0_0_24px_rgba(255,255,255,0.12)]">
                <Activity className="size-5" aria-hidden />
              </div>
              <div>
                <h1
                  className="text-xl font-semibold tracking-tight"
                  style={{ textShadow: '0 0 12px rgba(167,139,250,0.35)' }}
                >
                  IoT Control Center
                </h1>
                <p className="font-mono text-[11px] uppercase tracking-wider text-[var(--color-muted)]">
                  real-time monitoring · {devices.length} nodes · {nowHMS()}
                </p>
              </div>
            </motion.div>

            <div className="flex items-center gap-2">
              <Badge tone="ok">
                <StatusDot tone="ok" />
                {online} online
              </Badge>
              <Badge tone="warn">
                <StatusDot tone="warn" />
                {degraded} degraded
              </Badge>
              <Badge tone="danger">
                <StatusDot tone="danger" />
                {offline} offline
              </Badge>
              <Button variant="cta" size="sm">
                <Power className="size-3.5" aria-hidden />
                Failover
              </Button>
            </div>
          </header>

          {/* Bento grid — 12-col */}
          <div className="grid grid-cols-12 gap-4">
            {/* Hero stat row */}
            <KpiTile
              className="col-span-12 sm:col-span-6 lg:col-span-3"
              icon={<Thermometer className="size-4" aria-hidden />}
              label="Temperature"
              unit="°C"
              value={temp}
              delta={+0.3}
              accent="var(--color-chart-1)"
            />
            <KpiTile
              className="col-span-12 sm:col-span-6 lg:col-span-3"
              icon={<Droplets className="size-4" aria-hidden />}
              label="Humidity"
              unit="%"
              value={humidity}
              delta={-1.2}
              accent="#e4e4e7"
            />
            <KpiTile
              className="col-span-12 sm:col-span-6 lg:col-span-3"
              icon={<Gauge className="size-4" aria-hidden />}
              label="Pressure"
              unit="hPa"
              value={pressure}
              delta={+0.8}
              accent="#a1a1aa"
            />
            <KpiTile
              className="col-span-12 sm:col-span-6 lg:col-span-3"
              icon={<Zap className="size-4" aria-hidden />}
              label="Throughput"
              unit="msg/s"
              value={throughput}
              delta={+12}
              accent="var(--color-chart-1)"
            />

            {/* Gradient bars — large hero chart */}
            <Card hairline className="col-span-12 lg:col-span-8">
              <CardHeader>
                <div>
                  <CardTitle>Equipment utilisation · last 7 days</CardTitle>
                  <p className="mt-1 text-sm text-[var(--color-text)]">
                    Hourly runtime aggregated by asset class
                  </p>
                </div>
                <Badge tone="ok">
                  <StatusDot tone="ok" /> live
                </Badge>
              </CardHeader>
              <CardContent>
                <GradientBars labels={barLabels} series={barSeries} height={260} />
              </CardContent>
            </Card>

            {/* Radial gauge */}
            <Card hairline className="col-span-12 sm:col-span-6 lg:col-span-4">
              <CardHeader>
                <CardTitle>Fleet health index</CardTitle>
                <Badge tone="info">SLA 99.5%</Badge>
              </CardHeader>
              <CardContent>
                <div className="flex items-center justify-center pt-2 pb-1">
                  <RadialGauge
                    value={healthScore}
                    label="composite score"
                    from="#71717a"
                    to="var(--color-primary)"
                    size={220}
                  />
                </div>
                <div className="mt-2 grid grid-cols-3 gap-2 text-center font-mono text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
                  <div>
                    <div className="text-base text-[var(--color-text)]">
                      {(healthScore * 0.99).toFixed(0)}
                    </div>
                    avail
                  </div>
                  <div>
                    <div className="text-base text-[var(--color-text)]">
                      {(healthScore - 4).toFixed(0)}
                    </div>
                    latency
                  </div>
                  <div>
                    <div className="text-base text-[var(--color-text)]">
                      {(healthScore + 2).toFixed(0)}
                    </div>
                    integrity
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Heatmap */}
            <Card hairline className="col-span-12 lg:col-span-7">
              <CardHeader>
                <div>
                  <CardTitle>Activity density</CardTitle>
                  <p className="mt-1 text-sm text-[var(--color-text)]">
                    Telemetry events · 7d × 24h window
                  </p>
                </div>
                <Badge tone="muted">UTC</Badge>
              </CardHeader>
              <CardContent>
                <Heatmap matrix={heatmap} />
              </CardContent>
            </Card>

            {/* Sites */}
            <Card hairline className="col-span-12 lg:col-span-5">
              <CardHeader>
                <CardTitle>Edge sites</CardTitle>
                <Badge tone="muted">6 regions</Badge>
              </CardHeader>
              <CardContent>
                <SiteGrid />
              </CardContent>
            </Card>

            {/* Temp trend */}
            <Card hairline className="col-span-12 lg:col-span-8">
              <CardHeader>
                <CardTitle>Temperature trend</CardTitle>
                <Badge tone="ok">
                  <StatusDot tone="ok" /> live
                </Badge>
              </CardHeader>
              <CardContent>
                <Sparkline data={tempSeries} color="var(--color-primary)" label="last 40 samples (°C)" />
              </CardContent>
            </Card>

            {/* Alerts */}
            <Card hairline className="col-span-12 lg:col-span-4">
              <CardHeader>
                <div className="flex items-center gap-2">
                  <AlertTriangle className="size-4 text-[var(--color-warn)]" aria-hidden />
                  <CardTitle>Alert stream</CardTitle>
                </div>
                <Badge tone="warn">stream</Badge>
              </CardHeader>
              <CardContent>
                <AlertFeed alerts={alerts} />
              </CardContent>
            </Card>

            {/* Devices */}
            <Card hairline className="col-span-12">
              <CardHeader>
                <div className="flex items-center gap-2">
                  <Cpu className="size-4 text-zinc-300" aria-hidden />
                  <CardTitle>Edge devices</CardTitle>
                </div>
                <div className="flex items-center gap-2 text-[var(--color-muted)]">
                  <Wifi className="size-4" aria-hidden />
                  <span className="font-mono text-xs">{devices.length} total</span>
                </div>
              </CardHeader>
              <CardContent className="px-0">
                <DeviceList devices={devices} />
              </CardContent>
            </Card>
          </div>

          <footer className="mt-8 text-center font-mono text-[11px] uppercase tracking-wider text-[var(--color-muted)]">
            Generated with UI/UX Pro Max · Bento · React 19 · Tailwind 4 · Motion
          </footer>
    </div>
  )
}

/** Bento-style KPI tile that wraps the existing KpiCard with a gradient flourish. */
function KpiTile({
  className,
  icon,
  label,
  unit,
  value,
  delta,
  accent,
}: {
  className?: string
  icon: React.ReactNode
  label: string
  unit: string
  value: number
  delta: number
  accent: string
}) {
  return (
    <div className={className}>
      <KpiCard title={label} unit={unit} value={value} delta={delta} icon={icon} accent={accent} />
    </div>
  )
}
