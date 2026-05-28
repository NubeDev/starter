import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';

import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/core/theme/theme_providers.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/settings/presentation/pin_settings_section.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';
import 'package:rubix_flutter/shared/widgets/scaffold/ambient_glow_background.dart';

/// Settings — Figma-aligned: hero with serif-italic accent, dense
/// sections (Appearance · Language · Security · Account).
class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final t = Theme.of(context).nube;
    final themeMode = ref.watch(themeModeProvider);
    final locale = ref.watch(localeProvider);

    return AmbientGlowBackground(
      child: Scaffold(
        backgroundColor: Colors.transparent,
        body: SafeArea(
          child: ListView(
            padding: const EdgeInsets.fromLTRB(20, 12, 20, 32),
            children: [
              const SizedBox(height: 6),
              const _Hero(),
              const SizedBox(height: 10),
              Text(
                'Tune the console. Your preferences sync locally.',
                style: TextStyle(color: t.muted, fontSize: 14, height: 1.45),
              ),
              const SizedBox(height: 22),

              const _SectionHeader('Appearance'),
              const SizedBox(height: 10),
              _SegmentedRow<ThemeMode>(
                value: themeMode,
                onChanged: (m) =>
                    ref.read(themeModeProvider.notifier).set(m),
                items: [
                  _SegItem(ThemeMode.system, l.themeSystem, LucideIcons.monitor),
                  _SegItem(ThemeMode.light, l.themeLight, LucideIcons.sun),
                  _SegItem(ThemeMode.dark, l.themeDark, LucideIcons.moon),
                ],
              ),

              const SizedBox(height: 24),
              const _SectionHeader('Language'),
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
              const _SectionHeader('Security'),
              const SizedBox(height: 10),
              const PinSettingsSection(),

              const SizedBox(height: 24),
              const _SectionHeader('Account'),
              const SizedBox(height: 10),
              _SignOutRow(
                label: l.signOut,
                onTap: () async {
                  final ok = await showNubeConfirmDialog(
                    context,
                    title: 'Sign out?',
                    message: 'You\'ll need to re-enter credentials next time.',
                    confirmLabel: l.signOut,
                    destructive: true,
                  );
                  if (ok == true) {
                    await ref
                        .read(authControllerProvider.notifier)
                        .logout();
                  }
                },
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────
// Hero
// ────────────────────────────────────────────────────────────────────────

class _Hero extends StatelessWidget {
  const _Hero();
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final italic = accentItalicTextStyle(context, fontSize: 38);
    return Text.rich(
      TextSpan(
        children: [
          TextSpan(
            text: 'Settings &\n',
            style: TextStyle(
              color: t.text,
              fontSize: 38,
              fontWeight: FontWeight.w600,
              height: 1.05,
              letterSpacing: -0.8,
            ),
          ),
          TextSpan(text: 'account.', style: italic),
        ],
      ),
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
        letterSpacing: 1.2,
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────
// Segmented control
// ────────────────────────────────────────────────────────────────────────

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
            Expanded(
              child: _SegmentButton(
                item: item,
                selected: item.value == value,
                onTap: () => onChanged(item.value),
              ),
            ),
        ],
      ),
    );
  }
}

class _SegmentButton<T> extends StatefulWidget {
  const _SegmentButton({
    required this.item,
    required this.selected,
    required this.onTap,
  });
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
                  fontWeight:
                      widget.selected ? FontWeight.w600 : FontWeight.w500,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────
// Language list
// ────────────────────────────────────────────────────────────────────────

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
  const _LangRow({
    required this.item,
    required this.selected,
    required this.onTap,
  });
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
                    fontWeight:
                        widget.selected ? FontWeight.w600 : FontWeight.w500,
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

// ────────────────────────────────────────────────────────────────────────
// Sign out — destructive row.
// ────────────────────────────────────────────────────────────────────────

class _SignOutRow extends StatefulWidget {
  const _SignOutRow({required this.label, required this.onTap});
  final String label;
  final Future<void> Function() onTap;
  @override
  State<_SignOutRow> createState() => _SignOutRowState();
}

class _SignOutRowState extends State<_SignOutRow> {
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
        onTap: () => widget.onTap(),
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          decoration: BoxDecoration(
            color: _hover
                ? t.danger.withValues(alpha: 0.06)
                : t.surface,
            borderRadius: BorderRadius.circular(10),
            border: Border.all(
              color: _hover
                  ? t.danger.withValues(alpha: 0.3)
                  : t.border,
            ),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
          child: Row(
            children: [
              Container(
                width: 32,
                height: 32,
                decoration: BoxDecoration(
                  color: t.danger.withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(8),
                ),
                alignment: Alignment.center,
                child: Icon(LucideIcons.logOut, size: 15, color: t.danger),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  widget.label,
                  style: TextStyle(
                    color: t.danger,
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              Icon(LucideIcons.chevronRight, size: 16, color: t.danger),
            ],
          ),
        ),
      ),
    );
  }
}
