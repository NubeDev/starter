import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/shared/widgets/glass.dart';

/// A single toast: one at a time, auto-dismisses. Ported from the React
/// `Toast`/`ToastProvider`. Driven by [toastProvider]; rendered by [ToastHost]
/// floating above the nav dock. Call `ref.read(toastProvider.notifier).show()`.
@immutable
class ToastData {
  const ToastData({required this.id, required this.text, required this.accent});
  final int id;
  final String text;
  final Color accent;
}

final toastProvider = NotifierProvider<ToastNotifier, ToastData?>(ToastNotifier.new);

class ToastNotifier extends Notifier<ToastData?> {
  int _counter = 0;
  Timer? _timer;

  @override
  ToastData? build() {
    ref.onDispose(() => _timer?.cancel());
    return null;
  }

  void show(String text, {Color accent = RubixTokens.primary}) {
    _counter += 1;
    state = ToastData(id: _counter, text: text, accent: accent);
    _timer?.cancel();
    _timer = Timer(const Duration(milliseconds: 3400), () => state = null);
  }
}

/// Renders the current toast above the dock with a spring entrance. Mount once,
/// high in the shell's stack.
class ToastHost extends ConsumerWidget {
  const ToastHost({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final toast = ref.watch(toastProvider);
    return Positioned(
      left: RubixTokens.margin,
      right: RubixTokens.margin,
      bottom: 112,
      child: IgnorePointer(
        child: AnimatedSwitcher(
          duration: const Duration(milliseconds: 260),
          transitionBuilder: (child, anim) => FadeTransition(
            opacity: anim,
            child: SlideTransition(
              position: Tween(
                begin: const Offset(0, 0.4),
                end: Offset.zero,
              ).animate(CurvedAnimation(parent: anim, curve: Curves.easeOutBack)),
              child: child,
            ),
          ),
          child: toast == null
              ? const SizedBox.shrink()
              : Center(
                  key: ValueKey(toast.id),
                  child: GlassSurface(
                    strong: true,
                    borderRadius: BorderRadius.circular(999),
                    boxShadow: GlassShadow.glow(toast.accent, alpha: 0.55),
                    padding:
                        const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                    child: Text(
                      toast.text,
                      textAlign: TextAlign.center,
                      style: const TextStyle(
                        color: RubixTokens.ink,
                        fontSize: 14,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ),
        ),
      ),
    );
  }
}
