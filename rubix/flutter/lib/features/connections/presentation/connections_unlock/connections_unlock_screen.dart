import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:rubix_flutter/features/settings/data/settings_providers.dart';

/// Lock screen shown before `/connections*` when a PIN is set and
/// the session has not yet been unlocked. Correct entry flips
/// [pinUnlockedProvider] and routes the user on; the gate in
/// `app_router.dart` then lets them through.
class ConnectionsUnlockScreen extends ConsumerStatefulWidget {
  const ConnectionsUnlockScreen({super.key, this.redirectTo});

  /// Where to send the user after a successful unlock. Defaults to
  /// `/connections`.
  final String? redirectTo;

  @override
  ConsumerState<ConnectionsUnlockScreen> createState() =>
      _ConnectionsUnlockScreenState();
}

class _ConnectionsUnlockScreenState
    extends ConsumerState<ConnectionsUnlockScreen> {
  final _controller = TextEditingController();
  String? _error;
  bool _checking = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (_checking) return;
    setState(() {
      _checking = true;
      _error = null;
    });
    final entered = _controller.text.trim();
    final stored = await ref.read(connectionsPinProvider.future);
    if (!mounted) return;
    if (stored != null && entered == stored) {
      ref.read(pinUnlockedProvider.notifier).unlock();
      final dest = widget.redirectTo ?? '/connections';
      context.go(dest);
      return;
    }
    setState(() {
      _checking = false;
      _error = 'Incorrect PIN.';
      _controller.clear();
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Locked'),
        leading: IconButton(
          icon: const Icon(Icons.arrow_back),
          onPressed: () => context.canPop()
              ? context.pop()
              : context.go('/home'),
        ),
      ),
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 320),
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Icon(
                  Icons.lock,
                  size: 56,
                  color: theme.colorScheme.primary,
                ),
                const SizedBox(height: 16),
                Text(
                  'Enter PIN to access connections',
                  textAlign: TextAlign.center,
                  style: theme.textTheme.titleMedium,
                ),
                const SizedBox(height: 24),
                TextField(
                  controller: _controller,
                  autofocus: true,
                  obscureText: true,
                  keyboardType: TextInputType.number,
                  inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                  maxLength: 12,
                  textAlign: TextAlign.center,
                  onSubmitted: (_) => _submit(),
                  decoration: const InputDecoration(
                    labelText: 'PIN',
                    border: OutlineInputBorder(),
                  ),
                ),
                if (_error != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    _error!,
                    textAlign: TextAlign.center,
                    style: TextStyle(color: theme.colorScheme.error),
                  ),
                ],
                const SizedBox(height: 16),
                FilledButton(
                  onPressed: _checking ? null : _submit,
                  child: _checking
                      ? const SizedBox(
                          height: 20,
                          width: 20,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('Unlock'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
