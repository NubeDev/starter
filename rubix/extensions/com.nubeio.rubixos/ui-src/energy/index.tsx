// `EnergyPage` — dramatic single-page bento dashboard for energy
// generation (solar / wind / water / hydro / storage).
//
// v1 is presentation-quality with realistic mock data. Wiring to
// live warehouse templates is a follow-up — the existing templates
// are BMS-oriented (power & water reading) and don't yet cover
// generation by source.
//
// Layout (lg+):
//
//   ┌─────────────────── Hero gradient strip ────────────────────────┐
//   ├ Total gen (2x) ┬ Mix donut ─────────────────────────────────┐  │
//   ├ CO2 avoided    │                                             │  │
//   ├ Source vignettes (5x small, full-width) ────────────────────┤  │
//   ├ Generation timeline (full-width) ───────────────────────────┤  │
//   ├ Top sites    ┬ Live status ┬ Weather (3-up) ────────────────┘  │
//   └────────────────────────────────────────────────────────────────┘

import * as React from "react";

import {
  EXTENSION_ID,
} from "../types";

import { HeroGradient } from "./hero-gradient";
import { NavRail, type NavSection } from "./nav-rail";
import {
  Co2AvoidedTile,
  GenerationTimelineTile,
  LiveStatusTile,
  MixDonutTile,
  SourceVignettes,
  TopSitesTile,
  TotalGenerationTile,
  WeatherTile,
} from "./kpi-tiles";

export function EnergyPage(): React.ReactElement {
  const [section, setSection] = React.useState<NavSection>("overview");

  return (
    <div
      data-ext-id={EXTENSION_ID}
      data-ext-page="energy"
      className="nrg-shell rounded-2xl p-3 sm:p-4"
      style={{ fontFamily: '"Fira Sans", ui-sans-serif, system-ui, sans-serif' }}
    >
      <div className="flex items-start gap-3 sm:gap-4">
        <NavRail active={section} onSelect={setSection} />

        <div className="flex min-w-0 flex-1 flex-col gap-4">
          <HeroGradient
            title={titleFor(section)}
            subtitle={subtitleFor(section)}
            breadcrumb={["Rubix-OS", "Energy", sectionLabel(section)]}
            rightEl={
              <div className="hidden items-center gap-2 rounded-full border border-white/20 bg-white/10 px-3 py-1 text-xs text-white backdrop-blur sm:inline-flex">
                <span className="inline-block size-1.5 rounded-full bg-lime-300 shadow-[0_0_10px_2px_rgba(163,230,53,0.7)]" />
                live · v1 mock data
              </div>
            }
          />

          {/* Bento grid */}
          <div className="grid grid-cols-1 gap-3 lg:grid-cols-3">
            <TotalGenerationTile />
            <MixDonutTile />
            <Co2AvoidedTile />

            <SourceVignettes />

            <GenerationTimelineTile />

            <TopSitesTile />
            <LiveStatusTile />
            <WeatherTile />
          </div>

          <footer className="mt-2 text-[0.65rem] uppercase tracking-[0.16em] text-slate-500">
            v1 presentation · mock data · live warehouse wiring follow-up
          </footer>
        </div>
      </div>
    </div>
  );
}

function sectionLabel(s: NavSection): string {
  switch (s) {
    case "overview": return "Overview";
    case "solar":    return "Solar";
    case "wind":     return "Wind";
    case "water":    return "Water";
    case "hydro":    return "Hydro";
    case "storage":  return "Storage";
    case "settings": return "Settings";
  }
}

function titleFor(s: NavSection): string {
  if (s === "overview") return "Energy Dashboard";
  return `${sectionLabel(s)} · Energy Dashboard`;
}

function subtitleFor(s: NavSection): string {
  switch (s) {
    case "overview":
      return "Renewable generation across the portfolio — solar, wind, water and hydro.";
    case "solar":
      return "Photovoltaic farms — instant output, inverter health, weather impact.";
    case "wind":
      return "Onshore turbines — rotor speed, output, wind-direction trends.";
    case "water":
      return "Pumped-storage and run-of-river — flow rate, head, downstream demand.";
    case "hydro":
      return "Dams and penstocks — reservoir reserve, spillway state, throttling.";
    case "storage":
      return "Battery stacks — state of charge, throughput, cycle health.";
    case "settings":
      return "Tile preferences, units, alert thresholds.";
  }
}
