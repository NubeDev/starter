import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/router/app_router.dart';
import 'package:provision_app/shared/widgets/mood_backdrop.dart';

/// Root widget — wires the router and the dark Rubix theme into a
/// [MaterialApp.router]. The visual identity (glass, gradients, accents) lives
/// in the glass widgets + the `look` provider, not in [ThemeData].
class ProvisionApp extends ConsumerWidget {
  const ProvisionApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(appRouterProvider);
    return MaterialApp.router(
      title: 'Rubix Provision',
      debugShowCheckedModeBanner: false,
      theme: buildRubixTheme(),
      routerConfig: router,
      // Wrap every route — gate included — in the obsidian base + animated mood
      // backdrop, so the frosted glass blurs the dark gradient on every screen
      // (not a bare white canvas). The router's own Scaffolds are transparent
      // on top of this.
      builder: (context, child) => LookScope(
        look: ref.watch(lookProvider),
        child: ColoredBox(
          color: RubixTokens.obsidian,
          child: Stack(
            children: [
              const MoodBackdrop(),
              if (child != null) child,
            ],
          ),
        ),
      ),
    );
  }
}
