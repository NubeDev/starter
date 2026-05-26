import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/theme_providers.dart';

/// Settings screen — theme mode and locale picker.
class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    final locale = ref.watch(localeProvider);

    return Scaffold(
      appBar: AppBar(title: Text(AppLocalizations.of(context).settings)),
      body: ListView(
        children: [
          const _SectionHeader('Theme'),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: SegmentedButton<ThemeMode>(
              segments: [
                ButtonSegment(
                  value: ThemeMode.system,
                  label: Text(AppLocalizations.of(context).themeSystem),
                  icon: const Icon(Icons.brightness_auto),
                ),
                ButtonSegment(
                  value: ThemeMode.light,
                  label: Text(AppLocalizations.of(context).themeLight),
                  icon: const Icon(Icons.light_mode),
                ),
                ButtonSegment(
                  value: ThemeMode.dark,
                  label: Text(AppLocalizations.of(context).themeDark),
                  icon: const Icon(Icons.dark_mode),
                ),
              ],
              selected: {themeMode},
              onSelectionChanged: (set) {
                ref.read(themeModeProvider.notifier).set(set.first);
              },
            ),
          ),
          const SizedBox(height: 24),
          const _SectionHeader('Language'),
          RadioGroup<Locale?>(
            groupValue: locale,
            onChanged: (v) => ref.read(localeProvider.notifier).set(v),
            child: Column(
              children: [
                RadioListTile<Locale?>(
                  title: Text(AppLocalizations.of(context).themeSystem),
                  value: null,
                ),
                RadioListTile<Locale?>(
                  title: Text(AppLocalizations.of(context).languageEnglish),
                  value: const Locale('en'),
                ),
                RadioListTile<Locale?>(
                  title: Text(AppLocalizations.of(context).languageSpanish),
                  value: const Locale('es'),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader(this.title);
  final String title;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 24, 16, 8),
      child: Text(
        title,
        style: Theme.of(context).textTheme.titleSmall?.copyWith(
              color: Theme.of(context).colorScheme.primary,
            ),
      ),
    );
  }
}
