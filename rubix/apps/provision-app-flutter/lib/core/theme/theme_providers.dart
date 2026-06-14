import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:provision_app/core/theme/app_themes.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/core/theme/statuses.dart';

const _themeKeyPref = 'rbx.provision.theme';

/// The long-term theme key, persisted to shared_preferences. Mirrors the React
/// app's `ThemeProvider` localStorage seed.
final themeKeyProvider =
    NotifierProvider<ThemeKeyNotifier, ThemeKey>(ThemeKeyNotifier.new);

class ThemeKeyNotifier extends Notifier<ThemeKey> {
  @override
  ThemeKey build() => defaultTheme;

  Future<void> load() async {
    final prefs = await SharedPreferences.getInstance();
    state = themeKeyFromName(prefs.getString(_themeKeyPref)) ?? defaultTheme;
  }

  Future<void> set(ThemeKey key) async {
    state = key;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_themeKeyPref, key.name);
  }
}

/// The live device/connection status — transient (not persisted), tints the
/// accent. Set by the auth controller as the session moves through
/// offline → pairing → online/fault.
final statusProvider =
    NotifierProvider<StatusNotifier, StatusKey?>(StatusNotifier.new);

class StatusNotifier extends Notifier<StatusKey?> {
  @override
  StatusKey? build() => null;

  // ignore: use_setters_to_change_properties
  void set(StatusKey? status) => state = status;
}

/// The resolved [Look] — recomputed whenever the theme or status changes.
final lookProvider = Provider<Look>((ref) {
  final theme = themes[ref.watch(themeKeyProvider)]!;
  final status = ref.watch(statusProvider);
  return Look.resolve(theme, status);
});
