import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:rubix_flutter/features/settings/data/settings_providers.dart';

/// Settings section that lets the user set, change, or remove the
/// optional PIN that gates the `/connections` route.
class PinSettingsSection extends ConsumerWidget {
  const PinSettingsSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pinAsync = ref.watch(connectionsPinProvider);
    final theme = Theme.of(context);

    return pinAsync.when(
      loading: () => const Padding(
        padding: EdgeInsets.all(16),
        child: LinearProgressIndicator(),
      ),
      error: (e, _) => Padding(
        padding: const EdgeInsets.all(16),
        child: Text(
          'Failed to load PIN setting: $e',
          style: TextStyle(color: theme.colorScheme.error),
        ),
      ),
      data: (pin) {
        final hasPin = pin != null && pin.isNotEmpty;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ListTile(
              leading: Icon(
                hasPin ? Icons.lock_outline : Icons.lock_open_outlined,
              ),
              title: const Text('Connections PIN'),
              subtitle: Text(
                hasPin
                    ? 'A PIN is required to view or change connections.'
                    : 'No PIN set — the connections page is unprotected.',
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: Wrap(
                spacing: 12,
                children: [
                  FilledButton.tonalIcon(
                    onPressed: () => _showSetPinDialog(context, ref,
                        existing: hasPin),
                    icon: Icon(hasPin ? Icons.edit : Icons.add),
                    label: Text(hasPin ? 'Change PIN' : 'Set PIN'),
                  ),
                  if (hasPin)
                    OutlinedButton.icon(
                      onPressed: () => _confirmRemove(context, ref),
                      icon: const Icon(Icons.delete_outline),
                      label: const Text('Remove PIN'),
                    ),
                ],
              ),
            ),
          ],
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
    final ok = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        title: const Text('Remove PIN?'),
        content: const Text(
          'The connections page will no longer require a PIN.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton.tonal(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Remove'),
          ),
        ],
      ),
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
    return AlertDialog(
      title: Text(widget.replacing ? 'Change PIN' : 'Set PIN'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            controller: _pin,
            autofocus: true,
            obscureText: true,
            keyboardType: TextInputType.number,
            inputFormatters: [FilteringTextInputFormatter.digitsOnly],
            maxLength: 12,
            decoration: const InputDecoration(
              labelText: 'PIN',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _confirm,
            obscureText: true,
            keyboardType: TextInputType.number,
            inputFormatters: [FilteringTextInputFormatter.digitsOnly],
            maxLength: 12,
            onSubmitted: (_) => _submit(),
            decoration: const InputDecoration(
              labelText: 'Confirm PIN',
              border: OutlineInputBorder(),
            ),
          ),
          if (_error != null)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _submit,
          child: const Text('Save'),
        ),
      ],
    );
  }
}
