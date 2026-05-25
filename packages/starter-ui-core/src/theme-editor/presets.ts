// Curated preset gallery.
//
// Before stage 4 the ten preset palettes were declared inline here as
// `ThemeStyleProps` overrides on `defaultLightThemeStyles` /
// `defaultDarkThemeStyles`. Stage 4 moved that data into
// `@nube/starter-theme-tokens` (`NAMED_PALETTES`) so the web editor
// and the RN runtime read the exact same gallery.
//
// To add a preset: append a `NamedPalette` to `NAMED_PALETTES` in
// `packages/starter-theme-tokens/src/palette.ts`. The editor picks it
// up automatically through the re-export below.
//
// Attribution: palettes adapted from tweakcn
// (https://github.com/jnsahaj/tweakcn). Original work
// Copyright (c) 2024 Sahaj Jain. Apache License 2.0.
// Modifications Copyright (c) starter contributors.

import { NAMED_PALETTES } from "@nube/starter-theme-tokens";

import type { ThemePreset } from "./types.js";

export const DEFAULT_PRESETS: readonly ThemePreset[] = NAMED_PALETTES;
