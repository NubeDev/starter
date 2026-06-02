import 'package:flutter/material.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

/// Visual variants for [NubeButton] — mirror shadcn's button variants.
enum NubeButtonVariant { primary, secondary, outline, ghost, destructive }

/// Size scale for [NubeButton].
enum NubeButtonSize { sm, md, lg }

/// Flat, Tailwind/shadcn-style button. Use instead of `ElevatedButton`,
/// `OutlinedButton`, and `TextButton` from Material in feature code so the
/// UI keeps a single, non-Material look across the app.
///
/// Example:
/// ```dart
/// NubeButton(
///   label: 'Save changes',
///   icon: LucideIcons.check,
///   onPressed: _save,
/// )
/// ```
class NubeButton extends StatelessWidget {
  const NubeButton({
    super.key,
    required this.label,
    this.onPressed,
    this.icon,
    this.trailing,
    this.variant = NubeButtonVariant.primary,
    this.size = NubeButtonSize.md,
    this.loading = false,
    this.expand = false,
  });

  final String label;
  final VoidCallback? onPressed;
  final IconData? icon;
  final IconData? trailing;
  final NubeButtonVariant variant;
  final NubeButtonSize size;
  final bool loading;
  final bool expand;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final enabled = onPressed != null && !loading;

    final (height, hPad, fontSize, iconSize, radius) = switch (size) {
      NubeButtonSize.sm => (32.0, 12.0, 13.0, 14.0, 6.0),
      NubeButtonSize.md => (40.0, 16.0, 14.0, 16.0, 8.0),
      NubeButtonSize.lg => (48.0, 20.0, 15.0, 18.0, 10.0),
    };

    final (bg, fg, border) = switch (variant) {
      NubeButtonVariant.primary => (
          enabled ? t.leaf : t.surface2,
          isDark ? const Color(0xFF0A0A0A) : Colors.white,
          null,
        ),
      NubeButtonVariant.secondary => (
          t.surface2,
          t.text,
          null,
        ),
      NubeButtonVariant.outline => (
          t.ghostFill,
          t.text,
          t.ghostBorder,
        ),
      NubeButtonVariant.ghost => (
          t.ghostFill,
          t.text,
          t.ghostBorder,
        ),
      NubeButtonVariant.destructive => (
          enabled ? t.danger : t.surface2,
          Colors.white,
          null,
        ),
    };

    return _Pressable(
      enabled: enabled,
      onTap: onPressed,
      borderRadius: BorderRadius.circular(radius),
      hoverColor: variant == NubeButtonVariant.primary ||
              variant == NubeButtonVariant.destructive
          ? Colors.white.withValues(alpha: 0.08)
          : t.surface2.withValues(alpha: 0.75),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 120),
        curve: Curves.easeOut,
        height: height,
        constraints: expand ? null : const BoxConstraints(minWidth: 0),
        width: expand ? double.infinity : null,
        padding: EdgeInsets.symmetric(horizontal: hPad),
        decoration: BoxDecoration(
          color: bg,
          borderRadius: BorderRadius.circular(radius),
          border: border != null ? Border.all(color: border) : null,
        ),
        child: Row(
          mainAxisSize: expand ? MainAxisSize.max : MainAxisSize.min,
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            if (loading)
              SizedBox(
                width: iconSize,
                height: iconSize,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: fg,
                ),
              )
            else if (icon != null)
              Icon(icon, size: iconSize, color: fg),
            if ((loading || icon != null) && label.isNotEmpty)
              const SizedBox(width: 8),
            if (label.isNotEmpty)
              Text(
                label,
                style: TextStyle(
                  color: fg.withValues(alpha: enabled ? 1.0 : 0.5),
                  fontSize: fontSize,
                  fontWeight: FontWeight.w500,
                  letterSpacing: 0,
                ),
              ),
            if (trailing != null) ...[
              const SizedBox(width: 8),
              Icon(trailing, size: iconSize, color: fg),
            ],
          ],
        ),
      ),
    );
  }
}

/// Flat card with a hairline border. Replaces Material's elevated [Card].
class NubeCard extends StatelessWidget {
  const NubeCard({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.all(16),
    this.onTap,
  });

  final Widget child;
  final EdgeInsetsGeometry padding;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final card = Container(
      decoration: BoxDecoration(
        color: t.surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: t.border),
      ),
      padding: padding,
      child: child,
    );
    if (onTap == null) return card;
    return _Pressable(
      onTap: onTap,
      borderRadius: BorderRadius.circular(12),
      hoverColor: t.surface2,
      child: card,
    );
  }
}

/// Inline yellow callout — matches the React `--callout` semantic token.
/// Use sparingly: underlines + small badges, per the design system rules.
class NubeBadge extends StatelessWidget {
  const NubeBadge(this.label, {super.key, this.icon});
  final String label;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: t.callout,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 12, color: t.calloutForeground),
            const SizedBox(width: 4),
          ],
          Text(
            label,
            style: TextStyle(
              color: t.calloutForeground,
              fontSize: 11,
              fontWeight: FontWeight.w600,
              letterSpacing: 0.2,
            ),
          ),
        ],
      ),
    );
  }
}

/// Internal: tap target with no Material ink ripple, just a fast hover fade.
class _Pressable extends StatefulWidget {
  const _Pressable({
    required this.child,
    required this.onTap,
    required this.borderRadius,
    required this.hoverColor,
    this.enabled = true,
  });

  final Widget child;
  final VoidCallback? onTap;
  final BorderRadius borderRadius;
  final Color hoverColor;
  final bool enabled;

  @override
  State<_Pressable> createState() => _PressableState();
}

class _PressableState extends State<_Pressable> {
  bool _hover = false;
  bool _down = false;

  @override
  Widget build(BuildContext context) {
    final overlay = !widget.enabled || widget.onTap == null
        ? Colors.transparent
        : _down
            ? widget.hoverColor.withValues(alpha: 0.6)
            : _hover
                ? widget.hoverColor.withValues(alpha: 0.3)
                : Colors.transparent;

    return MouseRegion(
      cursor: widget.enabled && widget.onTap != null
          ? SystemMouseCursors.click
          : SystemMouseCursors.basic,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: widget.enabled ? (_) => setState(() => _down = true) : null,
        onTapUp: widget.enabled ? (_) => setState(() => _down = false) : null,
        onTapCancel: widget.enabled ? () => setState(() => _down = false) : null,
        onTap: widget.enabled ? widget.onTap : null,
        child: Stack(
          children: [
            widget.child,
            Positioned.fill(
              child: IgnorePointer(
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 50),
                  decoration: BoxDecoration(
                    color: overlay,
                    borderRadius: widget.borderRadius,
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Flat hairline-bordered top bar for push-routed screens (no Material
/// [AppBar], no shadow, no surface tint, Lucide back chevron).
class NubeTopBar extends StatelessWidget implements PreferredSizeWidget {
  const NubeTopBar({
    super.key,
    required this.title,
    this.onBack,
    this.actions = const [],
  });

  final String title;
  final VoidCallback? onBack;
  final List<NubeTopBarAction> actions;

  @override
  Size get preferredSize => const Size.fromHeight(56);

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final back = onBack ??
        (Navigator.of(context).canPop() ? () => Navigator.of(context).pop() : null);

    return Container(
      height: 56,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        color: t.bg,
        border: Border(bottom: BorderSide(color: t.border)),
      ),
      child: Row(
        children: [
          if (back != null)
            _NubeChromeIcon(icon: LucideIcons.arrowLeft, onTap: back),
          const SizedBox(width: 4),
          Expanded(
            child: Text(
              title,
              style: TextStyle(
                color: t.text,
                fontSize: 15,
                fontWeight: FontWeight.w600,
                letterSpacing: -0.1,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          for (final a in actions) ...[
            const SizedBox(width: 4),
            _NubeChromeIcon(
              icon: a.icon,
              onTap: a.onTap,
              tooltip: a.tooltip,
              danger: a.danger,
            ),
          ],
          const SizedBox(width: 4),
        ],
      ),
    );
  }
}

/// Action item for [NubeTopBar].
class NubeTopBarAction {
  const NubeTopBarAction({
    required this.icon,
    required this.onTap,
    this.tooltip,
    this.danger = false,
  });
  final IconData icon;
  final VoidCallback onTap;
  final String? tooltip;
  final bool danger;
}

class _NubeChromeIcon extends StatefulWidget {
  const _NubeChromeIcon({
    required this.icon,
    required this.onTap,
    this.tooltip,
    this.danger = false,
  });
  final IconData icon;
  final VoidCallback onTap;
  final String? tooltip;
  final bool danger;

  @override
  State<_NubeChromeIcon> createState() => _NubeChromeIconState();
}

class _NubeChromeIconState extends State<_NubeChromeIcon> {
  bool _hover = false;
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final fg = widget.danger ? t.danger : t.muted;
    final btn = MouseRegion(
      cursor: SystemMouseCursors.click,
      onEnter: (_) => setState(() => _hover = true),
      onExit: (_) => setState(() => _hover = false),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 120),
          width: 36,
          height: 36,
          decoration: BoxDecoration(
            color: _hover ? t.surface2 : Colors.transparent,
            borderRadius: BorderRadius.circular(8),
          ),
          alignment: Alignment.center,
          child: Icon(widget.icon, size: 18, color: fg),
        ),
      ),
    );
    return widget.tooltip != null
        ? Tooltip(message: widget.tooltip!, child: btn)
        : btn;
  }
}

/// shadcn-style text field. Wraps Material [TextFormField] with the Nube
/// label + helper layout so feature code doesn't repeat the styling.
class NubeField extends StatelessWidget {
  const NubeField({
    super.key,
    required this.controller,
    required this.label,
    this.hint,
    this.helperText,
    this.errorText,
    this.prefixIcon,
    this.suffixIcon,
    this.onSuffixTap,
    this.obscureText = false,
    this.keyboardType,
    this.autocorrect = true,
    this.autofocus = false,
    this.maxLength,
    this.inputFormatters,
    this.textAlign,
    this.onSubmitted,
    this.validator,
  });

  final TextEditingController controller;
  final String label;
  final String? hint;
  final String? helperText;
  final String? errorText;
  final IconData? prefixIcon;
  final IconData? suffixIcon;
  final VoidCallback? onSuffixTap;
  final bool obscureText;
  final TextInputType? keyboardType;
  final bool autocorrect;
  final bool autofocus;
  final int? maxLength;
  final List<dynamic>? inputFormatters;
  final TextAlign? textAlign;
  final ValueChanged<String>? onSubmitted;
  final String? Function(String?)? validator;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: TextStyle(
            color: t.text,
            fontSize: 13,
            fontWeight: FontWeight.w500,
            height: 1.2,
          ),
        ),
        const SizedBox(height: 6),
        TextFormField(
          controller: controller,
          autofocus: autofocus,
          obscureText: obscureText,
          keyboardType: keyboardType,
          autocorrect: autocorrect,
          maxLength: maxLength,
          textAlign: textAlign ?? TextAlign.start,
          inputFormatters: inputFormatters?.cast(),
          onFieldSubmitted: onSubmitted,
          validator: validator,
          style: TextStyle(color: t.text, fontSize: 14),
          decoration: InputDecoration(
            hintText: hint,
            counterText: '',
            prefixIcon: prefixIcon != null
                ? Padding(
                    padding: const EdgeInsets.only(left: 10, right: 4),
                    child: Icon(prefixIcon, size: 16, color: t.muted),
                  )
                : null,
            prefixIconConstraints: const BoxConstraints(minWidth: 32, minHeight: 32),
            suffixIcon: suffixIcon != null
                ? IconButton(
                    icon: Icon(suffixIcon, size: 16, color: t.muted),
                    splashRadius: 16,
                    onPressed: onSuffixTap,
                  )
                : null,
          ),
        ),
        if (helperText != null && errorText == null) ...[
          const SizedBox(height: 4),
          Text(
            helperText!,
            style: TextStyle(color: t.muted, fontSize: 12),
          ),
        ],
        if (errorText != null) ...[
          const SizedBox(height: 4),
          Text(
            errorText!,
            style: TextStyle(color: t.danger, fontSize: 12),
          ),
        ],
      ],
    );
  }
}

/// Banner-style inline alert (replaces Material SnackBar in form errors).
class NubeAlert extends StatelessWidget {
  const NubeAlert.error(this.message, {super.key}) : _kind = _AlertKind.error;
  const NubeAlert.info(this.message, {super.key}) : _kind = _AlertKind.info;

  final String message;
  final _AlertKind _kind;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final (bg, fg, border, icon) = switch (_kind) {
      _AlertKind.error => (
          t.danger.withValues(alpha: 0.08),
          t.danger,
          t.danger.withValues(alpha: 0.25),
          LucideIcons.alertCircle,
        ),
      _AlertKind.info => (
          t.surface2,
          t.text,
          t.border,
          LucideIcons.info,
        ),
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: border),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 16, color: fg),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              message,
              style: TextStyle(color: fg, fontSize: 13, height: 1.35),
            ),
          ),
        ],
      ),
    );
  }
}

enum _AlertKind { error, info }

/// shadcn-style confirmation dialog. Returns `true` if the user confirms.
Future<bool?> showNubeConfirmDialog(
  BuildContext context, {
  required String title,
  required String message,
  required String confirmLabel,
  String cancelLabel = 'Cancel',
  bool destructive = false,
}) {
  return showDialog<bool>(
    context: context,
    barrierColor: Colors.black.withValues(alpha: 0.45),
    builder: (ctx) {
      final t = Theme.of(ctx).nube;
      return Dialog(
        backgroundColor: t.surface,
        elevation: 0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(color: t.border),
        ),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 380),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  title,
                  style: TextStyle(
                    color: t.text,
                    fontSize: 15,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  message,
                  style: TextStyle(
                    color: t.muted,
                    fontSize: 13,
                    height: 1.45,
                  ),
                ),
                const SizedBox(height: 20),
                Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    NubeButton(
                      label: cancelLabel,
                      variant: NubeButtonVariant.ghost,
                      size: NubeButtonSize.sm,
                      onPressed: () => Navigator.of(ctx).pop(false),
                    ),
                    const SizedBox(width: 8),
                    NubeButton(
                      label: confirmLabel,
                      variant: destructive
                          ? NubeButtonVariant.destructive
                          : NubeButtonVariant.primary,
                      size: NubeButtonSize.sm,
                      onPressed: () => Navigator.of(ctx).pop(true),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      );
    },
  );
}
