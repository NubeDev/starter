// `<Input>` — wraps `TextInput`. Mirrors `starter-ui-kit/src/components/ui/input.tsx`.
//
// Web reads/writes `value` via the host element's `value` prop; on RN
// we forward through `value` / `onChangeText` (which is what RN
// canonically exposes — `onChange` would fight RN's event shape).

import * as React from "react";
import { StyleSheet, TextInput, View } from "react-native";

import { useTheme } from "./theme.js";

export interface InputProps {
  value?: string;
  defaultValue?: string;
  onChangeText?: (text: string) => void;
  placeholder?: string;
  disabled?: boolean;
  secureTextEntry?: boolean;
  invalid?: boolean;
  /** Required for non-decorative inputs — read by VoiceOver/TalkBack. */
  accessibilityLabel?: string;
  accessibilityHint?: string;
  testID?: string;
  keyboardType?: "default" | "email-address" | "numeric" | "phone-pad" | "url";
  autoCapitalize?: "none" | "sentences" | "words" | "characters";
  autoCorrect?: boolean;
}

export function Input(props: InputProps): React.ReactElement {
  const {
    value,
    defaultValue,
    onChangeText,
    placeholder,
    disabled = false,
    secureTextEntry,
    invalid = false,
    accessibilityLabel,
    accessibilityHint,
    testID,
    keyboardType,
    autoCapitalize,
    autoCorrect,
  } = props;
  const t = useTheme();
  const styles = StyleSheet.create({
    wrap: {
      borderRadius: t.radius("3xl"),
      borderWidth: 1,
      borderColor: invalid ? t.color("destructive") : t.color("border"),
      backgroundColor: t.color("input"),
      opacity: disabled ? 0.5 : 1,
    },
    input: {
      paddingHorizontal: t.space(3),
      paddingVertical: t.space(2),
      fontSize: t.fontSize("sm"),
      color: t.color("foreground"),
      minHeight: 36,
    },
  });
  return (
    <View style={styles.wrap}>
      <TextInput
        accessible
        accessibilityLabel={accessibilityLabel}
        accessibilityHint={accessibilityHint}
        accessibilityState={{ disabled }}
        testID={testID}
        value={value}
        defaultValue={defaultValue}
        onChangeText={onChangeText}
        placeholder={placeholder}
        placeholderTextColor={t.color("muted-foreground")}
        editable={!disabled}
        secureTextEntry={secureTextEntry}
        keyboardType={keyboardType}
        autoCapitalize={autoCapitalize}
        autoCorrect={autoCorrect}
        style={styles.input}
      />
    </View>
  );
}
