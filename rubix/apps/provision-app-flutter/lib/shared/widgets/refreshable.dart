import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/core/api/refresh.dart';
import 'package:provision_app/core/theme/look.dart';

/// Wraps a scrollable page in a pull-to-refresh gesture wired to the shared
/// [refreshProvider]. Pulling down bumps the refresh signal — every list
/// controller that watches it re-fetches — so this works for any screen without
/// per-screen refresh plumbing.
///
/// The [child] should be the page's scroll view; this forces
/// [AlwaysScrollableScrollPhysics] so the pull gesture is available even when
/// the content is shorter than the viewport. The indicator is tinted to the
/// active DNA accent on the dark surface.
class Refreshable extends ConsumerWidget {
  const Refreshable({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final look = context.look;
    return RefreshIndicator(
      color: look.accent,
      backgroundColor: look.base,
      displacement: 72, // clear the floating TopBar
      onRefresh: () => ref.read(refreshProvider.notifier).refreshAndSettle(),
      child: child,
    );
  }
}
