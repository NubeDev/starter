import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/core/theme/theme_providers.dart';
import 'package:rubix_flutter/features/settings/presentation/pin_settings_section.dart';

/// Settings screen — theme mode and locale picker.
class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).nube;
    final themeMode = ref.watch(themeModeProvider);
    final locale = ref.watch(localeProvider);

    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        Text(
          l.settings,
          style: TextStyle(
            color: t.text,
            fontSize: 20,
            fontWeight: FontWeight.w700,
            letterSpacing: -0.3,
          ),
        ),
        const SizedBox(height: 20),

        _SectionHeader('Appearance'),
        const SizedBox(height: 10),
        _SegmentedRow<ThemeMode>(
          value: themeMode,
          onChanged: (m) => ref.read(themeModeProvider.notifier).set(m),
          items: [
            _SegItem(ThemeMode.system, l.themeSystem, LucideIcons.monitor),
            _SegItem(ThemeMode.light, l.themeLight, LucideIcons.sun),
            _SegItem(ThemeMode.dark, l.themeDark, LucideIcons.moon),
          ],
        ),

        const SizedBox(height: 24),
        _SectionHeader('Language'),
        const SizedBox(height: 10),
        _LanguageList(
          value: locale,
          onChanged: (loc) => ref.read(localeProvider.notifier).set(loc),
          items: [
            _LangItem(null, l.themeSystem),
            _LangItem(const Locale('en'), l.languageEnglish),
            _LangItem(const Locale('es'), l.languageSpanish),
          ],
        ),

        const SizedBox(height: 24),
        _SectionHeader('Security'),
        const SizedBox(height: 10),
        const PinSettingsSection(),
      ],
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader(this.label);
  final String label;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Text(
      label.toUpperCase(),
      style: TextStyle(
        color: t.muted,
        fontSize: 11,
        fontWeight: FontWeight.w600,
        letterSpacing: 0.6,
      ),
    );
  }
}

class _SegItem<T> {
  const _SegItem(this.value, this.label, this.icon);
  final T value;
  final String label;
  final IconData icon;
}

class _SegmentedRow<T> extends StatelessWidget {
  const _SegmentedRow({
    required this.value,
    required this.onChanged,
    required this.items,
  });
  final T value;
  final ValueChanged<T> onChanged;
  final List<_SegItem<T>> items;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      padding: const EdgeInsets.all(4),
      decoration: BoxDecoration(
        color: t.surface,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: t.border),
      ),
      child: Row(
        children: [
          for (final item in items)
            Expanded(child: _SegmentButton(item: item, selected: item.value == value, onTap: () => onChanged(item.value))),
        ],
      ),
    );
  }
}

class _SegmentButton<T> extends StatefulWidget {
  const _SegmentButton({required this.item, required this.selected, required this.onTap});
  final _SegItem<T> item;
  final bool selected;
  final VoidCallback onTap;
  @override
  State<_SegmentButton<T>> createState() => _SegmentButtonState<T>();
}

class _SegmentButtonState<T> extends State<_SegmentButton<T>> {
  bool _hover = false;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final bg = widget.selected
        ? (isDark ? const Color(0xFF142F33) : const Color(0xFFF2F7F7))
        : (_hover ? t.surface2 : Colors.transparent);
    final fg = widget.selected ? t.leaf : t.muted;
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          padding: const EdgeInsets.symmetric(vertical: 8),
          decoration: BoxDecoration(
            color: bg,
            borderRadius: BorderRadius.circular(6),
          ),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(widget.item.icon, size: 14, color: fg),
              const SizedBox(width: 6),
              Text(
                widget.item.label,
                style: TextStyle(
                  color: fg,
                  fontSize: 12.5,
                  fontWeight: widget.selected ? FontWeight.w600 : FontWeight.w500,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _LangItem {
  const _LangItem(this.value, this.label);
  final Locale? value;
  final String label;
}

class _LanguageList extends StatelessWidget {
  const _LanguageList({
    required this.value,
    required this.onChanged,
    required this.items,
  });
  final Locale? value;
  final ValueChanged<Locale?> onChanged;
  final List<_LangItem> items;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      decoration: BoxDecoration(
        color: t.surface,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: t.border),
      ),
      child: Column(
        children: [
          for (var i = 0; i < items.length; i++) ...[
            if (i > 0) Divider(height: 1, color: t.border),
            _LangRow(
              item: items[i],
              selected: items[i].value == value,
              onTap: () => onChanged(items[i].value),
            ),
          ],
        ],
      ),
    );
  }
}

class _LangRow extends StatefulWidget {
  const _LangRow({required this.item, required this.selected, required this.onTap});
  final _LangItem item;
  final bool selected;
  final VoidCallback onTap;
  @override
  State<_LangRow> createState() => _LangRowState();
}

class _LangRowState extends State<_LangRow> {
  bool _hover = false;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          color: _hover ? t.surface2 : Colors.transparent,
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  widget.item.label,
                  style: TextStyle(
                    color: t.text,
                    fontSize: 13.5,
                    fontWeight: widget.selected ? FontWeight.w600 : FontWeight.w500,
                  ),
                ),
              ),
              if (widget.selected)
                Icon(LucideIcons.check, size: 16, color: t.leaf),
            ],
          ),
        ),
      ),
    );
  }
}
