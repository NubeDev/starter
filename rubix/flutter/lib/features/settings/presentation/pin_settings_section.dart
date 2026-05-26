import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';

import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/settings/data/settings_providers.dart';
import 'package:rubix_flutter/shared/widgets/loading_indicator.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

/// Settings section that lets the user set, change, or remove the
/// optional PIN that gates the `/connections` route.
class PinSettingsSection extends ConsumerWidget {
  const PinSettingsSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pinAsync = ref.watch(connectionsPinProvider);
    final t = Theme.of(context).nube;

    return pinAsync.when(
      loading: () => const Padding(
        padding: EdgeInsets.all(16),
        child: LoadingIndicator(),
      ),
      error: (e, _) => Padding(
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 8),
        child: NubeAlert.error('Failed to load PIN setting: $e'),
      ),
      data: (pin) {
        final hasPin = pin != null && pin.isNotEmpty;
        return NubeCard(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    width: 32,
                    height: 32,
                    decoration: BoxDecoration(
                      color: hasPin
                          ? t.leaf.withValues(alpha: 0.12)
                          : t.surface2,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    alignment: Alignment.center,
                    child: Icon(
                      hasPin ? LucideIcons.lock : LucideIcons.unlock,
                      size: 15,
                      color: hasPin ? t.leaf : t.muted,
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Connections PIN',
                          style: TextStyle(
                            color: t.text,
                            fontSize: 14,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          hasPin
                              ? 'A PIN is required to view or change connections.'
                              : 'No PIN set — the connections page is unprotected.',
                          style: TextStyle(
                            color: t.muted,
                            fontSize: 12.5,
                            height: 1.35,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 14),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  NubeButton(
                    label: hasPin ? 'Change PIN' : 'Set PIN',
                    icon: hasPin ? LucideIcons.pencil : LucideIcons.plus,
                    variant: NubeButtonVariant.secondary,
                    size: NubeButtonSize.sm,
                    onPressed: () =>
                        _showSetPinDialog(context, ref, existing: hasPin),
                  ),
                  if (hasPin)
                    NubeButton(
                      label: 'Remove PIN',
                      icon: LucideIcons.trash2,
                      variant: NubeButtonVariant.outline,
                      size: NubeButtonSize.sm,
                      onPressed: () => _confirmRemove(context, ref),
                    ),
                ],
              ),
            ],
          ),
        );
      },
    );
  }

  Future<void> _showSetPinDialog(
    BuildContext context,
    WidgetRef ref, {
    required bool existing,
  }) async {
    final result = await showDialog<String>(
      context: context,
      barrierColor: Colors.black.withValues(alpha: 0.45),
      builder: (_) => _SetPinDialog(replacing: existing),
    );
    if (result == null) return;
    await setConnectionsPin(ref, result);
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(existing ? 'PIN updated' : 'PIN set')),
      );
    }
  }

  Future<void> _confirmRemove(BuildContext context, WidgetRef ref) async {
    final ok = await showNubeConfirmDialog(
      context,
      title: 'Remove PIN?',
      message: 'The connections page will no longer require a PIN.',
      confirmLabel: 'Remove',
      destructive: true,
    );
    if (ok != true) return;
    await setConnectionsPin(ref, null);
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('PIN removed')),
      );
    }
  }
}

class _SetPinDialog extends StatefulWidget {
  const _SetPinDialog({required this.replacing});

  final bool replacing;

  @override
  State<_SetPinDialog> createState() => _SetPinDialogState();
}

class _SetPinDialogState extends State<_SetPinDialog> {
  final _pin = TextEditingController();
  final _confirm = TextEditingController();
  String? _error;

  @override
  void dispose() {
    _pin.dispose();
    _confirm.dispose();
    super.dispose();
  }

  void _submit() {
    final p = _pin.text.trim();
    final c = _confirm.text.trim();
    if (p.length < 4) {
      setState(() => _error = 'PIN must be at least 4 digits.');
      return;
    }
    if (p != c) {
      setState(() => _error = 'PINs do not match.');
      return;
    }
    Navigator.of(context).pop(p);
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Dialog(
      backgroundColor: t.surface,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: t.border),
      ),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 360),
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                widget.replacing ? 'Change PIN' : 'Set PIN',
                style: TextStyle(
                  color: t.text,
                  fontSize: 15,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 16),
              NubeField(
                controller: _pin,
                label: 'PIN',
                autofocus: true,
                obscureText: true,
                keyboardType: TextInputType.number,
                inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                maxLength: 12,
              ),
              const SizedBox(height: 12),
              NubeField(
                controller: _confirm,
                label: 'Confirm PIN',
                obscureText: true,
                keyboardType: TextInputType.number,
                inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                maxLength: 12,
                onSubmitted: (_) => _submit(),
                errorText: _error,
              ),
              const SizedBox(height: 20),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  NubeButton(
                    label: 'Cancel',
                    variant: NubeButtonVariant.ghost,
                    size: NubeButtonSize.sm,
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                  const SizedBox(width: 8),
                  NubeButton(
                    label: 'Save',
                    size: NubeButtonSize.sm,
                    onPressed: _submit,
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
