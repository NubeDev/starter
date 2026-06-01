// Inline-SVG illustrations for the Energy dashboard.
//
// These are NOT lucide glyphs — they're chunky, multi-path, gradient-
// filled scene illustrations meant to anchor the bento cards. Each
// accepts an `accent` for one-off colourisation and `className` for
// sizing. Motion (spin / bob / charge) is delegated to CSS keyframes
// defined in `./animations.css` so the prefers-reduced-motion guard
// in that file covers them automatically.

import * as React from "react";

interface IllustrationProps {
  className?: string;
  title?: string;
}

/* --------------------------- Solar panel ----------------------------- */

export function SolarPanelIllustration(
  { className, title = "Solar array" }: IllustrationProps,
): React.ReactElement {
  return (
    <svg
      viewBox="0 0 160 120"
      className={className}
      role="img"
      aria-label={title}
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        <linearGradient id="nrg-sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#0F1B3A" />
          <stop offset="100%" stopColor="#1E1B4B" />
        </linearGradient>
        <radialGradient id="nrg-sun" cx="50%" cy="50%" r="50%">
          <stop offset="0%"  stopColor="#fef3c7" />
          <stop offset="55%" stopColor="#fbbf24" />
          <stop offset="100%" stopColor="#f59e0b" />
        </radialGradient>
        <linearGradient id="nrg-panel" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%"  stopColor="#1e3a8a" />
          <stop offset="55%" stopColor="#1e40af" />
          <stop offset="100%" stopColor="#312E81" />
        </linearGradient>
      </defs>

      <rect x="0" y="0" width="160" height="120" rx="14" fill="url(#nrg-sky)" />
      {/* Sun + halo */}
      <circle cx="124" cy="32" r="18" fill="url(#nrg-sun)" opacity="0.95" />
      <circle cx="124" cy="32" r="28" fill="#fbbf24" opacity="0.12" />
      <circle cx="124" cy="32" r="40" fill="#fbbf24" opacity="0.06" />
      {/* Rays */}
      {[0, 45, 90, 135, 180, 225, 270, 315].map((deg) => {
        const r1 = 22; const r2 = 30;
        const rad = (deg * Math.PI) / 180;
        const x1 = 124 + Math.cos(rad) * r1;
        const y1 = 32  + Math.sin(rad) * r1;
        const x2 = 124 + Math.cos(rad) * r2;
        const y2 = 32  + Math.sin(rad) * r2;
        return <line key={deg} x1={x1} y1={y1} x2={x2} y2={y2} stroke="#fcd34d" strokeWidth="1.5" strokeLinecap="round" opacity="0.85" />;
      })}

      {/* Ground */}
      <rect x="0" y="92" width="160" height="28" fill="#0a0a23" opacity="0.6" />

      {/* Panel — perspective trapezoid */}
      <g>
        <polygon points="20,90 140,90 124,58 36,58" fill="url(#nrg-panel)" stroke="#0ea5e9" strokeWidth="0.6" opacity="0.95" />
        {/* Cell grid */}
        {[0, 1, 2, 3].map((row) => (
          <line key={"h" + row} x1={26 + row * 3} y1={66 + row * 7} x2={134 - row * 3} y2={66 + row * 7}
                stroke="#38bdf8" strokeWidth="0.4" opacity="0.55" />
        ))}
        {[0, 1, 2, 3, 4, 5, 6, 7].map((col) => {
          const xTop = 36 + (col / 7) * 88;
          const xBot = 20 + (col / 7) * 120;
          return <line key={"v" + col} x1={xTop} y1="58" x2={xBot} y2="90" stroke="#38bdf8" strokeWidth="0.4" opacity="0.5" />;
        })}
        {/* Glint */}
        <polygon points="60,68 78,68 70,84 52,84" fill="#67e8f9" opacity="0.22" />

        {/* Posts */}
        <rect x="44"  y="90" width="3" height="14" fill="#475569" />
        <rect x="113" y="90" width="3" height="14" fill="#475569" />
      </g>
    </svg>
  );
}

/* ----------------------------- Wind turbine -------------------------- */

export function WindTurbineIllustration(
  { className, title = "Wind turbine" }: IllustrationProps,
): React.ReactElement {
  return (
    <svg
      viewBox="0 0 160 120"
      className={className}
      role="img"
      aria-label={title}
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        <linearGradient id="nrg-sky-wind" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#082F49" />
          <stop offset="100%" stopColor="#0C1334" />
        </linearGradient>
        <linearGradient id="nrg-blade" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%"  stopColor="#e0f2fe" />
          <stop offset="100%" stopColor="#67e8f9" />
        </linearGradient>
        <linearGradient id="nrg-mast" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#cbd5e1" />
          <stop offset="100%" stopColor="#64748b" />
        </linearGradient>
      </defs>

      <rect x="0" y="0" width="160" height="120" rx="14" fill="url(#nrg-sky-wind)" />

      {/* Wind streaks */}
      <path d="M10 30 Q40 22 70 30" stroke="#22d3ee" strokeWidth="1" fill="none" opacity="0.32" strokeLinecap="round" />
      <path d="M14 46 Q44 36 80 44" stroke="#22d3ee" strokeWidth="0.8" fill="none" opacity="0.22" strokeLinecap="round" />
      <path d="M100 70 Q124 60 150 66" stroke="#22d3ee" strokeWidth="0.8" fill="none" opacity="0.22" strokeLinecap="round" />

      {/* Distant turbine */}
      <g opacity="0.45" transform="translate(120 60) scale(0.55)">
        <rect x="-2" y="0" width="4" height="50" fill="#94a3b8" />
        <circle cx="0" cy="-4" r="3" fill="#cbd5e1" />
        <g className="nrg-spin-slow" transform="translate(0 -4)">
          <ellipse cx="0" cy="-18" rx="2.6" ry="14" fill="#e2e8f0" />
          <ellipse cx="15.6" cy="9" rx="2.6" ry="14" fill="#e2e8f0" transform="rotate(120 0 0)" />
          <ellipse cx="-15.6" cy="9" rx="2.6" ry="14" fill="#e2e8f0" transform="rotate(240 0 0)" />
        </g>
      </g>

      {/* Ground */}
      <rect x="0" y="98" width="160" height="22" fill="#020617" opacity="0.7" />
      <path d="M0 100 Q40 92 80 96 T160 98 L160 120 L0 120 Z" fill="#0f172a" opacity="0.85" />

      {/* Main mast */}
      <polygon points="58,98 62,98 64,32 56,32" fill="url(#nrg-mast)" />
      <circle cx="60" cy="32" r="4.5" fill="#94a3b8" stroke="#475569" strokeWidth="0.6" />

      {/* Rotor — spins via CSS */}
      <g className="nrg-spin-med" transform="translate(60 32)">
        <ellipse cx="0"     cy="-26" rx="3.2" ry="22" fill="url(#nrg-blade)" stroke="#0891b2" strokeWidth="0.4" />
        <ellipse cx="22.5"  cy="13"  rx="3.2" ry="22" fill="url(#nrg-blade)" stroke="#0891b2" strokeWidth="0.4" transform="rotate(120 0 0)" />
        <ellipse cx="-22.5" cy="13"  rx="3.2" ry="22" fill="url(#nrg-blade)" stroke="#0891b2" strokeWidth="0.4" transform="rotate(240 0 0)" />
        <circle cx="0" cy="0" r="3" fill="#67e8f9" />
      </g>
    </svg>
  );
}

/* ----------------------------- Water droplet ------------------------- */

export function WaterDropletIllustration(
  { className, title = "Water flow" }: IllustrationProps,
): React.ReactElement {
  return (
    <svg
      viewBox="0 0 160 120"
      className={className}
      role="img"
      aria-label={title}
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        <linearGradient id="nrg-water-bg" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#0c2247" />
          <stop offset="100%" stopColor="#0a1a3a" />
        </linearGradient>
        <radialGradient id="nrg-drop" cx="50%" cy="60%" r="60%">
          <stop offset="0%"  stopColor="#e0f2fe" />
          <stop offset="55%" stopColor="#38bdf8" />
          <stop offset="100%" stopColor="#0284c7" />
        </radialGradient>
      </defs>

      <rect x="0" y="0" width="160" height="120" rx="14" fill="url(#nrg-water-bg)" />

      {/* Background ripples */}
      <ellipse cx="80" cy="92" rx="58" ry="6" fill="none" stroke="#38bdf8" strokeWidth="0.8" opacity="0.35" />
      <ellipse cx="80" cy="92" rx="42" ry="4" fill="none" stroke="#38bdf8" strokeWidth="0.8" opacity="0.45" />
      <ellipse cx="80" cy="92" rx="26" ry="3" fill="none" stroke="#38bdf8" strokeWidth="0.8" opacity="0.55" />

      {/* Droplet — bob via CSS */}
      <g className="nrg-bob">
        <path
          d="M80 22 C 96 48 110 64 110 78 C 110 96 96 108 80 108 C 64 108 50 96 50 78 C 50 64 64 48 80 22 Z"
          fill="url(#nrg-drop)"
          stroke="#7dd3fc"
          strokeWidth="0.6"
        />
        {/* Highlight */}
        <path
          d="M68 60 C 64 70 64 78 68 86"
          stroke="#e0f2fe"
          strokeWidth="3"
          fill="none"
          strokeLinecap="round"
          opacity="0.7"
        />
        <circle cx="70" cy="50" r="3.5" fill="#f0f9ff" opacity="0.85" />
      </g>
    </svg>
  );
}

/* ------------------------------ Hydro dam ---------------------------- */

export function HydroDamIllustration(
  { className, title = "Hydro dam" }: IllustrationProps,
): React.ReactElement {
  return (
    <svg
      viewBox="0 0 160 120"
      className={className}
      role="img"
      aria-label={title}
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        <linearGradient id="nrg-hydro-sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#1e1b4b" />
          <stop offset="100%" stopColor="#0F0F23" />
        </linearGradient>
        <linearGradient id="nrg-dam" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#94a3b8" />
          <stop offset="100%" stopColor="#1e293b" />
        </linearGradient>
        <linearGradient id="nrg-reservoir" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#38bdf8" />
          <stop offset="100%" stopColor="#1e3a8a" />
        </linearGradient>
        <linearGradient id="nrg-spill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#e0f2fe" />
          <stop offset="100%" stopColor="#38bdf8" />
        </linearGradient>
      </defs>

      <rect x="0" y="0" width="160" height="120" rx="14" fill="url(#nrg-hydro-sky)" />

      {/* Mountains */}
      <polygon points="-4,72 30,32 50,52 78,18 110,52 134,30 164,68 164,120 -4,120" fill="#312E81" opacity="0.65" />
      <polygon points="-4,82 22,52 44,68 66,40 92,66 118,46 164,80 164,120 -4,120" fill="#1e1b4b" opacity="0.85" />

      {/* Reservoir */}
      <rect x="14" y="60" width="84" height="26" fill="url(#nrg-reservoir)" />
      <path d="M14 62 Q40 58 60 64 T98 62 L98 60 L14 60 Z" fill="#7dd3fc" opacity="0.4" />

      {/* Dam wall */}
      <polygon points="98,42 116,42 124,108 96,108" fill="url(#nrg-dam)" />
      <line x1="100" y1="56" x2="120" y2="56" stroke="#0f172a" strokeWidth="0.6" opacity="0.6" />
      <line x1="100" y1="72" x2="121" y2="72" stroke="#0f172a" strokeWidth="0.6" opacity="0.6" />
      <line x1="100" y1="88" x2="122" y2="88" stroke="#0f172a" strokeWidth="0.6" opacity="0.6" />

      {/* Spillway */}
      <path d="M104 70 Q108 84 122 100 L130 108 L118 108 Q104 96 102 80 Z" fill="url(#nrg-spill)" opacity="0.92" />
      <path d="M108 74 Q110 86 122 100" stroke="#f0f9ff" strokeWidth="0.6" fill="none" opacity="0.7" />

      {/* River downstream */}
      <path d="M118 108 Q140 104 160 110 L160 120 L118 120 Z" fill="#0c4a6e" opacity="0.9" />
    </svg>
  );
}

/* ------------------------------ Battery ------------------------------ */

export function BatteryIllustration({
  className,
  title = "Battery storage",
  socPct = 71,
}: IllustrationProps & { socPct?: number }): React.ReactElement {
  const clamped = Math.max(0, Math.min(100, socPct));
  const fillWidth = (104 * clamped) / 100;
  return (
    <svg
      viewBox="0 0 160 120"
      className={className}
      role="img"
      aria-label={title}
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        <linearGradient id="nrg-bat-bg" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#1a2e0c" />
          <stop offset="100%" stopColor="#0F0F23" />
        </linearGradient>
        <linearGradient id="nrg-bat-fill" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%"  stopColor="#65a30d" />
          <stop offset="100%" stopColor="#a3e635" />
        </linearGradient>
        <linearGradient id="nrg-bat-shell" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%"  stopColor="#475569" />
          <stop offset="100%" stopColor="#1e293b" />
        </linearGradient>
      </defs>

      <rect x="0" y="0" width="160" height="120" rx="14" fill="url(#nrg-bat-bg)" />

      {/* Bolt */}
      <path d="M118 30 L108 56 L120 56 L110 84 L132 50 L120 50 Z"
            fill="#a3e635" opacity="0.18" />

      {/* Battery shell */}
      <rect x="20" y="42" width="112" height="44" rx="6" fill="url(#nrg-bat-shell)" stroke="#94a3b8" strokeWidth="0.8" />
      <rect x="132" y="56" width="6" height="16" rx="1.5" fill="#94a3b8" />

      {/* Inner well */}
      <rect x="24" y="46" width="104" height="36" rx="4" fill="#0a0a23" />

      {/* Charge fill */}
      <rect
        x="24"
        y="46"
        width={fillWidth}
        height="36"
        rx="4"
        fill="url(#nrg-bat-fill)"
        className="nrg-charge"
      />

      {/* Charge gridlines */}
      {[25, 50, 75].map((pct) => (
        <line
          key={pct}
          x1={24 + (104 * pct) / 100}
          y1="46"
          x2={24 + (104 * pct) / 100}
          y2="82"
          stroke="#0a0a23"
          strokeWidth="1.2"
          opacity="0.65"
        />
      ))}

      {/* SOC label */}
      <text x="76" y="106" textAnchor="middle" fill="#a3e635" fontSize="11" fontWeight="700"
            style={{ fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace" }}>
        {clamped}% SoC
      </text>
    </svg>
  );
}
