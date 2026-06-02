import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/statuses.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/auth/data/auth_controller.dart';
import 'package:provision_app/shared/widgets/glass.dart';

/// Floating status bar: live connection dot + the signed-in principal + logout.
/// Sits above the page content. Ported from the React `TopBar` — the page
/// bodies pad past it via their top padding.
class TopBar extends ConsumerStatefulWidget {
  const TopBar({super.key});

  @override
  ConsumerState<TopBar> createState() => _TopBarState();
}

class _TopBarState extends ConsumerState<TopBar>
    with SingleTickerProviderStateMixin {
  late final AnimationController _breath = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 2500),
  )..repeat(reverse: true);

  @override
  void dispose() {
    _breath.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final status = ref.watch(statusProvider);
    final auth = ref.watch(authControllerProvider);
    final s = status != null ? statuses[status] : null;
    final dotColor = s?.accent ?? look.accent;

    return Positioned(
      left: RubixTokens.margin,
      right: RubixTokens.margin,
      top: MediaQuery.of(context).padding.top + 8,
      child: GlassSurface(
        strong: true,
        borderRadius: BorderRadius.circular(999),
        boxShadow: GlassShadow.glass,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        child: Row(
          children: [
            FadeTransition(
              opacity: Tween(begin: 0.5, end: 1.0).animate(_breath),
              child: Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(color: dotColor, shape: BoxShape.circle),
              ),
            ),
            const SizedBox(width: 8),
            const Icon(LucideIcons.wifi, size: 14, color: RubixTokens.inkMuted),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                auth.user?.email ?? 'Connected',
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: RubixTokens.ink,
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            if (s != null)
              Text(s.label.toUpperCase(), style: RubixText.label),
            const SizedBox(width: 4),
            _LogoutButton(onTap: () => ref.read(authControllerProvider.notifier).logout()),
          ],
        ),
      ),
    );
  }
}

class _LogoutButton extends StatelessWidget {
  const _LogoutButton({required this.onTap});
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: Semantics(
          button: true,
          label: 'Log out',
          child: const SizedBox(
            width: 28,
            height: 28,
            child: Icon(LucideIcons.logOut, size: 16, color: RubixTokens.inkMuted),
          ),
        ),
      ),
    );
  }
}
