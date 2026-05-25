// theme/provider.tsx — RN ThemeProvider.
//
// Per APP-SHELL.md §Theme + dark mode: the same layout-prefs store the
// web app uses (`starter-ui-core/theme-editor`) is initialised from
// `Appearance.getColorScheme()` here and updated via
// `Appearance.addChangeListener`. The actual token resolution lives in
// `@nube/starter-theme-tokens`; this provider is just the RN-side
// glue.
//
// Block 4 ships the smallest viable surface: a `ThemeProvider` that
// reads the OS colour scheme and exposes a typed context with the
// resolved foreground / background pair. Full token resolution +
// presets land with the dashboard renderer in Block 5.

import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { Appearance, type ColorSchemeName } from 'react-native';

export interface ThemeTokens {
  /** Either 'light' or 'dark' — never null at the consumer layer. */
  readonly mode: 'light' | 'dark';
  readonly background: string;
  readonly foreground: string;
  readonly accent: string;
  readonly border: string;
}

const LIGHT: ThemeTokens = {
  mode: 'light',
  background: '#FFFFFF',
  foreground: '#0B1220',
  accent: '#3B82F6',
  border: '#E5E7EB',
};
const DARK: ThemeTokens = {
  mode: 'dark',
  background: '#0B1220',
  foreground: '#F8FAFC',
  accent: '#60A5FA',
  border: '#1F2937',
};

function tokensFor(scheme: ColorSchemeName): ThemeTokens {
  return scheme === 'dark' ? DARK : LIGHT;
}

const ThemeCtx = createContext<ThemeTokens>(LIGHT);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [tokens, setTokens] = useState<ThemeTokens>(() =>
    tokensFor(Appearance.getColorScheme()),
  );

  useEffect(() => {
    const sub = Appearance.addChangeListener(({ colorScheme }) => {
      setTokens(tokensFor(colorScheme));
    });
    return () => sub.remove();
  }, []);

  return <ThemeCtx.Provider value={tokens}>{children}</ThemeCtx.Provider>;
}

export function useTheme(): ThemeTokens {
  return useContext(ThemeCtx);
}
