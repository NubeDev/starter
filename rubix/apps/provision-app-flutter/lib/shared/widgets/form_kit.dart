import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/shared/widgets/glass.dart';

/// Glass-styled form primitives shared across Place / Connect / Devices etc.
/// Ported from the React `FormKit.tsx`. One concept per export.

/// A labelled form row — uppercase label above its control.
class Field extends StatelessWidget {
  const Field({required this.label, required this.child, super.key});
  final String label;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: 6),
          child: Text(label.toUpperCase(), style: RubixText.label),
        ),
        child,
      ],
    );
  }
}

/// Glass text input — the React `TextInput`. A frosted field with an accent
/// focus ring. Pass a [controller] or rely on [onChanged].
class GlassTextField extends StatelessWidget {
  const GlassTextField({
    this.controller,
    this.value,
    this.onChanged,
    this.onSubmitted,
    this.hint,
    this.obscureText = false,
    this.keyboardType,
    this.semanticLabel,
    this.maxLines = 1,
    this.monospace = false,
    super.key,
  });

  final TextEditingController? controller;
  final String? value;
  final ValueChanged<String>? onChanged;
  final VoidCallback? onSubmitted;
  final String? hint;
  final bool obscureText;
  final TextInputType? keyboardType;
  final String? semanticLabel;
  final int maxLines;
  final bool monospace;

  @override
  Widget build(BuildContext context) {
    final radius = BorderRadius.circular(
      maxLines > 1 ? RubixTokens.radiusMd : RubixTokens.radiusMd,
    );
    return GlassSurface(
      borderRadius: radius,
      // TextField needs a Material ancestor for ink/selection; keep it
      // transparent so the glass surface shows through unchanged.
      child: Material(
        type: MaterialType.transparency,
        child: TextField(
          controller: controller,
          onChanged: onChanged,
          onSubmitted: onSubmitted == null ? null : (_) => onSubmitted!(),
          obscureText: obscureText,
          keyboardType: keyboardType,
          maxLines: obscureText ? 1 : maxLines,
          style: TextStyle(
            color: RubixTokens.ink,
            fontSize: maxLines > 1 ? 12 : 16,
            fontFamily: monospace ? 'monospace' : null,
          ),
          cursorColor: RubixTokens.primary,
          decoration: InputDecoration(
            isDense: true,
            hintText: hint,
            hintStyle: const TextStyle(color: RubixTokens.inkMuted),
            contentPadding: const EdgeInsets.symmetric(
              horizontal: 16,
              vertical: 14,
            ),
            border: InputBorder.none,
            focusedBorder: OutlineInputBorder(
              borderRadius: radius,
              borderSide: BorderSide(
                color: RubixTokens.primary.withValues(alpha: 0.6),
                width: 2,
              ),
            ),
            enabledBorder: InputBorder.none,
          ),
        ),
      ),
    );
  }
}

/// Glass dropdown — the React `Picker` (a styled `<select>`). Shows
/// [placeholder] when no value is selected.
class GlassPicker extends StatelessWidget {
  const GlassPicker({
    required this.value,
    required this.options,
    required this.placeholder,
    required this.onChanged,
    super.key,
  });

  final String value;
  final List<PickerOption> options;
  final String placeholder;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    return GlassSurface(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: DropdownButtonHideUnderline(
          child: DropdownButton<String>(
            value: value.isEmpty ? null : value,
            isExpanded: true,
            hint: Text(
              placeholder,
              style: const TextStyle(color: RubixTokens.inkMuted, fontSize: 16),
            ),
            dropdownColor: RubixTokens.surfaceLow,
            iconEnabledColor: RubixTokens.inkMuted,
            style: const TextStyle(color: RubixTokens.ink, fontSize: 16),
            padding: const EdgeInsets.symmetric(vertical: 10),
            items: [
              for (final o in options)
                DropdownMenuItem(value: o.value, child: Text(o.label)),
            ],
            onChanged: (v) {
              if (v != null) onChanged(v);
            },
          ),
        ),
      ),
    );
  }
}

class PickerOption {
  const PickerOption({required this.value, required this.label});
  final String value;
  final String label;
}

/// Pill toggle with a spring-sliding knob — the React `Toggle`. [accent] tints
/// the "on" track (defaults to the primary teal).
class GlassToggle extends StatelessWidget {
  const GlassToggle({
    required this.on,
    required this.onToggle,
    this.accent = RubixTokens.primary,
    this.semanticLabel,
    super.key,
  });

  final bool on;
  final VoidCallback onToggle;
  final Color accent;
  final String? semanticLabel;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      toggled: on,
      label: semanticLabel,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onToggle,
        child: MouseRegion(
          cursor: SystemMouseCursors.click,
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 200),
            width: 44,
            height: 24,
            padding: const EdgeInsets.all(2),
            decoration: BoxDecoration(
              color: on ? accent : const Color(0x24FFFFFF),
              borderRadius: BorderRadius.circular(999),
            ),
            alignment: on ? Alignment.centerRight : Alignment.centerLeft,
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 200),
              curve: Curves.easeOut,
              width: 20,
              height: 20,
              decoration: const BoxDecoration(
                color: Colors.white,
                shape: BoxShape.circle,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
