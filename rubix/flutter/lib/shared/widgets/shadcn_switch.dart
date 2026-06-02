import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

/// Minimal shadcn-style switch with no Material ripple.
class ShadcnSwitch extends StatefulWidget {
  const ShadcnSwitch({
    super.key,
    required this.value,
    required this.onChanged,
    this.enabled = true,
  });

  final bool value;
  final ValueChanged<bool>? onChanged;
  final bool enabled;

  @override
  State<ShadcnSwitch> createState() => _ShadcnSwitchState();
}

class _ShadcnSwitchState extends State<ShadcnSwitch> {
  bool _hover = false;
  bool _down = false;

  bool get _enabled => widget.enabled && widget.onChanged != null;

  void _toggle() {
    if (!_enabled) return;
    widget.onChanged?.call(!widget.value);
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final onFill = NubeTokens.dark.leaf;

    final trackColor = widget.value ? onFill : Colors.transparent;
    final borderColor = widget.value ? Colors.transparent : t.border;

    final overlay = !_enabled
        ? Colors.transparent
        : _down
            ? Colors.black.withValues(alpha: 0.08)
            : _hover
                ? Colors.black.withValues(alpha: 0.05)
                : Colors.transparent;

    return Semantics(
      container: true,
      toggled: widget.value,
      enabled: _enabled,
      button: true,
      child: MouseRegion(
        cursor: _enabled ? SystemMouseCursors.click : SystemMouseCursors.basic,
        onEnter: (_) => setState(() => _hover = true),
        onExit: (_) => setState(() {
          _hover = false;
          _down = false;
        }),
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: _toggle,
          onTapDown: _enabled ? (_) => setState(() => _down = true) : null,
          onTapUp: _enabled ? (_) => setState(() => _down = false) : null,
          onTapCancel: _enabled ? () => setState(() => _down = false) : null,
          child: AnimatedOpacity(
            duration: const Duration(milliseconds: 120),
            opacity: _enabled ? 1 : 0.55,
            child: SizedBox(
              width: 36,
              height: 20,
              child: Stack(
                children: [
                  AnimatedContainer(
                    duration: const Duration(milliseconds: 150),
                    curve: Curves.easeOut,
                    decoration: BoxDecoration(
                      color: trackColor,
                      borderRadius: BorderRadius.circular(999),
                      border: Border.all(color: borderColor, width: 1),
                    ),
                  ),
                  Positioned.fill(
                    child: IgnorePointer(
                      child: AnimatedContainer(
                        duration: const Duration(milliseconds: 50),
                        curve: Curves.easeOut,
                        decoration: BoxDecoration(
                          color: overlay,
                          borderRadius: BorderRadius.circular(999),
                        ),
                      ),
                    ),
                  ),
                  AnimatedPositioned(
                    duration: const Duration(milliseconds: 150),
                    curve: Curves.easeOut,
                    top: 2,
                    left: widget.value ? 18 : 2,
                    child: Container(
                      width: 16,
                      height: 16,
                      decoration: const BoxDecoration(
                        color: Colors.white,
                        shape: BoxShape.circle,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
