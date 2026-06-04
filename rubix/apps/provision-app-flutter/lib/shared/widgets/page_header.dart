import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/look.dart';

/// Page header: uppercase eyebrow + headline — the recurring page-top recipe,
/// ported from the React `PageHeader`. Set [accentTitle] to render the headline
/// in the DNA teal heading-accent (the Figma treatment for entity names like
/// "aidan").
class PageHeader extends StatelessWidget {
  const PageHeader({
    required this.eyebrow,
    required this.title,
    this.accentTitle = false,
    super.key,
  });

  final String eyebrow;
  final String title;
  final bool accentTitle;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(eyebrow.toUpperCase(), style: RubixText.label),
          accentTitle
              ? HeadingAccent(title)
              : Text(title, style: RubixText.headlineMobile),
        ],
      ),
    );
  }
}

/// A headline rendered in the DNA teal heading-accent — the recurring Figma
/// treatment for entity names (e.g. the teal "aidan" on Device detail / Place).
/// Reads the live accent from the resolved [Look] so it tracks the theme.
class HeadingAccent extends StatelessWidget {
  const HeadingAccent(this.text, {this.style, super.key});

  final String text;

  /// Optional base style to tint; defaults to the mobile headline ramp.
  final TextStyle? style;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: (style ?? RubixText.headlineMobile)
          .copyWith(color: context.look.accent),
    );
  }
}

/// Uppercase architectural section label — the React `SectionLabel`.
class SectionLabel extends StatelessWidget {
  const SectionLabel(this.label, {super.key});
  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Text(label.toUpperCase(), style: RubixText.label),
    );
  }
}
