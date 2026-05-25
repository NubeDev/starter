// `<Spinner>` — RN's `ActivityIndicator` themed via tokens. Mirrors
// `starter-ui-kit/src/components/ui/spinner.tsx` (which renders the
// `Loader2` lucide icon with `role="status"` and `aria-label="Loading"`).

import * as React from "react";
import { ActivityIndicator } from "react-native";

import { useTheme } from "./theme.js";

export interface SpinnerProps {
  size?: "small" | "large";
  color?: string;
  accessibilityLabel?: string;
  accessibilityHint?: string;
  testID?: string;
}

export function Spinner({
  size = "small",
  color,
  accessibilityLabel = "Loading",
  accessibilityHint,
  testID,
}: SpinnerProps): React.ReactElement {
  const t = useTheme();
  return (
    <ActivityIndicator
      accessible
      accessibilityRole="progressbar"
      accessibilityLabel={accessibilityLabel}
      accessibilityHint={accessibilityHint}
      testID={testID}
      size={size}
      color={color ?? t.color("primary")}
    />
  );
}
