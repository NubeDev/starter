import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';

/// Writable boolean point rendered as a switch (e.g. a relay/output).
/// Optimistic local flip in the preview (no command dispatch yet). Ported from
/// the React `ToggleWidget`.
class ToggleWidget extends StatefulWidget {
  const ToggleWidget({
    required this.title,
    required this.accent,
    this.on = false,
    super.key,
  });

  final String title;
  final bool on;
  final Color accent;

  @override
  State<ToggleWidget> createState() => _ToggleWidgetState();
}

class _ToggleWidgetState extends State<ToggleWidget> {
  late bool _on = widget.on;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(widget.title.toUpperCase(), style: RubixText.label),
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(
              _on ? 'On' : 'Off',
              style: const TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w600,
                color: RubixTokens.inkVariant,
              ),
            ),
            GlassToggle(
              on: _on,
              accent: widget.accent,
              semanticLabel: widget.title,
              onToggle: () => setState(() => _on = !_on),
            ),
          ],
        ),
      ],
    );
  }
}
