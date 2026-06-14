import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';

/// Page header: uppercase eyebrow + headline — the recurring page-top recipe,
/// ported from the React `PageHeader`.
class PageHeader extends StatelessWidget {
  const PageHeader({required this.eyebrow, required this.title, super.key});

  final String eyebrow;
  final String title;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(eyebrow.toUpperCase(), style: RubixText.label),
          Text(title, style: RubixText.headlineMobile),
        ],
      ),
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
