import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';

import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/core/theme/theme_providers.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/home/presentation/home_controller.dart';
import 'package:rubix_flutter/shared/widgets/shadcn_switch.dart';
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
    final email = ref.watch(currentUserProvider).value?.email ?? '—';

    return AmbientGlowBackground(
      child: Scaffold(
        backgroundColor: Colors.transparent,
        body: SafeArea(
          child: ListView(
            padding: const EdgeInsets.fromLTRB(20, 12, 20, 32),
            children: [
              const SizedBox(height: 6),
              const _PreferencesGlassPill(),
              const SizedBox(height: 18),
              const _Hero(),
              const SizedBox(height: 10),
              Text(
                'Signed in as $email',
                style: TextStyle(color: t.muted, fontSize: 13, height: 1.45),
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
              const _AppLockRow(),

              const SizedBox(height: 18),
              _SignOutButton(
                label: 'Sign out',
                onTap: () async {
                  await ref.read(authControllerProvider.notifier).logout();
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
            text: 'Settings\n',
            style: TextStyle(
              color: t.text,
              fontSize: 38,
              fontWeight: FontWeight.w600,
              height: 1.05,
              letterSpacing: -0.8,
            ),
          ),
          TextSpan(text: '& account.', style: italic),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────
// Preferences glass pill — node 6:206–6:208.
// ─────────────────────────────────────────────────────────────────────

class _PreferencesGlassPill extends StatelessWidget {
  const _PreferencesGlassPill();

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    const dot = Color(0xFF21C45D);
    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: t.surface,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(color: t.border),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 8,
              height: 8,
              decoration: BoxDecoration(
                color: dot,
                shape: BoxShape.circle,
                boxShadow: [
                  BoxShadow(
                    color: dot.withValues(alpha: 0.55),
                    blurRadius: 6,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Text(
              'PREFERENCES',
              style: TextStyle(
                color: t.text,
                fontSize: 11,
                fontWeight: FontWeight.w600,
                letterSpacing: 2.0,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────
// App lock toggle — node 6:227–6:231.
// ─────────────────────────────────────────────────────────────────────

class _AppLockRow extends StatefulWidget {
  const _AppLockRow();
  @override
  State<_AppLockRow> createState() => _AppLockRowState();
}

class _AppLockRowState extends State<_AppLockRow> {
  // Defaults ON per design. Local state — backed by PIN preferences
  // when the feature is wired up; for now this is purely visual.
  bool _on = true;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return SizedBox(
      height: 56,
      child: Container(
        decoration: BoxDecoration(
          color: t.surface,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: t.border),
        ),
        padding: const EdgeInsets.symmetric(horizontal: 14),
        child: Row(
          children: [
            Expanded(
              child: Text(
                'App lock (PIN)',
                style: TextStyle(
                  color: t.text,
                  fontSize: 14,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ),
            ShadcnSwitch(
              value: _on,
              onChanged: (v) => setState(() => _on = v),
            ),
          ],
        ),
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
  bool _press = false;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final pressTint = t.surface2.withValues(alpha: 0.65);
    final bg = widget.selected
        ? (isDark ? const Color(0xFF142F33) : const Color(0xFFF2F7F7))
        : (_press ? pressTint : (_hover ? t.surface2 : Colors.transparent));
    final fg = widget.selected ? t.leaf : t.muted;
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() {
        _hover = false;
        _press = false;
      }),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: (_) => setState(() => _press = true),
        onTapUp: (_) => setState(() => _press = false),
        onTapCancel: () => setState(() => _press = false),
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 50),
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
  bool _press = false;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final bg = _press
        ? t.surface2.withValues(alpha: 0.7)
        : (_hover ? t.surface2 : Colors.transparent);
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() {
        _hover = false;
        _press = false;
      }),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: (_) => setState(() => _press = true),
        onTapUp: (_) => setState(() => _press = false),
        onTapCancel: () => setState(() => _press = false),
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 50),
          color: bg,
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

// ─────────────────────────────────────────────────────────────────────
// Sign out — full-width outlined danger button, node 6:232–6:237.
// ─────────────────────────────────────────────────────────────────────

class _SignOutButton extends StatefulWidget {
  const _SignOutButton({required this.label, required this.onTap});
  final String label;
  final Future<void> Function() onTap;
  @override
  State<_SignOutButton> createState() => _SignOutButtonState();
}

class _SignOutButtonState extends State<_SignOutButton> {
  bool _hover = false;
  bool _press = false;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final bg = _press
        ? t.danger.withValues(alpha: 0.12)
        : (_hover ? t.danger.withValues(alpha: 0.08) : t.ghostFill);
    return MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() {
        _hover = false;
        _press = false;
      }),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: (_) => setState(() => _press = true),
        onTapUp: (_) => setState(() => _press = false),
        onTapCancel: () => setState(() => _press = false),
        onTap: () => widget.onTap(),
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 50),
          height: 52,
          decoration: BoxDecoration(
            color: bg,
            borderRadius: BorderRadius.circular(14),
            border: Border.all(color: t.ghostBorder, width: 1),
          ),
          alignment: Alignment.center,
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(LucideIcons.logOut, size: 16, color: t.danger),
              const SizedBox(width: 8),
              Text(
                widget.label,
                style: TextStyle(
                  color: t.danger,
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
