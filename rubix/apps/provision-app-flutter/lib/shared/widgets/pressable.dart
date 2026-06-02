import 'package:flutter/material.dart';

/// A spring-y scale-on-tap wrapper — the Flutter analogue of framer-motion's
/// `whileTap={{ scale: 0.97 }}` that the React app puts on every pressable
/// glass surface. Scales down while pressed, springs back on release.
class Pressable extends StatefulWidget {
  const Pressable({
    required this.child,
    this.onTap,
    this.scale = 0.97,
    this.semanticLabel,
    super.key,
  });

  final Widget child;
  final VoidCallback? onTap;

  /// How far to scale down while held (0.88–0.97 in the React app).
  final double scale;
  final String? semanticLabel;

  @override
  State<Pressable> createState() => _PressableState();
}

class _PressableState extends State<Pressable> {
  bool _down = false;

  @override
  Widget build(BuildContext context) {
    final interactive = widget.onTap != null;
    final child = AnimatedScale(
      scale: _down ? widget.scale : 1,
      duration: const Duration(milliseconds: 120),
      curve: Curves.easeOut,
      child: widget.child,
    );

    if (!interactive) {
      return widget.semanticLabel != null
          ? Semantics(label: widget.semanticLabel, child: child)
          : child;
    }

    return Semantics(
      button: true,
      label: widget.semanticLabel,
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: (_) => setState(() => _down = true),
          onTapUp: (_) => setState(() => _down = false),
          onTapCancel: () => setState(() => _down = false),
          onTap: widget.onTap,
          child: child,
        ),
      ),
    );
  }
}
