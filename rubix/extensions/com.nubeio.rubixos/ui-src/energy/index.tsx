// `EnergyPage` — single-page bento dashboard for energy generation
// (solar / wind / water / hydro / storage).
//
// Sibling of `/usage`: same glass surfaces, same typography, same
// density. Navigation lives in the host sidebar (Energy entry);
// section switching is a pill strip inside the hero band.
//
// Layout: hero band (with breadcrumb + title + section pills) →
// bento grid below. No two-column shell, no duplicate left rail.

import * as React from "react";

import { EXTENSION_ID } from "../types";

import { HeroGradient, type EnergySection } from "./hero-gradient";
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
  const [section, setSection] = React.useState<EnergySection>("overview");

  return (
    <div
      data-ext-id={EXTENSION_ID}
      data-ext-page="energy"
      className="flex min-w-0 flex-col gap-4 p-3 sm:p-4"
    >
      <HeroGradient
        title={titleFor(section)}
        subtitle={subtitleFor(section)}
        breadcrumb={["Rubix-OS", "Energy", sectionLabel(section)]}
        section={section}
        onSelectSection={setSection}
        rightEl={
          <div className="hidden items-center gap-2 rounded-full border border-border/60 bg-muted/30 px-3 py-1 text-xs text-muted-foreground sm:inline-flex">
            <span className="inline-block size-1.5 rounded-full bg-emerald-500 shadow-[0_0_10px_2px_color-mix(in_oklab,var(--color-primary)_50%,transparent)]" />
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

      <footer className="ext-eyebrow mt-2">
        v1 presentation · mock data · live warehouse wiring follow-up
      </footer>
    </div>
  );
}

function sectionLabel(s: EnergySection): string {
  switch (s) {
    case "overview": return "Overview";
    case "solar":    return "Solar";
    case "wind":     return "Wind";
    case "water":    return "Water";
    case "hydro":    return "Hydro";
    case "storage":  return "Storage";
  }
}

function titleFor(s: EnergySection): string {
  if (s === "overview") return "Energy Dashboard";
  return `${sectionLabel(s)} · Energy Dashboard`;
}

function subtitleFor(s: EnergySection): string {
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
  }
}
