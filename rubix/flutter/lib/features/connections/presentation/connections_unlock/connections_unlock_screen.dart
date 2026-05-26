import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:lucide_icons/lucide_icons.dart';

import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/settings/data/settings_providers.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';

/// Lock screen shown before `/connections*` when a PIN is set and
/// the session has not yet been unlocked.
class ConnectionsUnlockScreen extends ConsumerStatefulWidget {
  const ConnectionsUnlockScreen({super.key, this.redirectTo});

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
    final t = Theme.of(context).nube;
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Scaffold(
      backgroundColor: t.bg,
      body: SafeArea(
        child: Column(
          children: [
            NubeTopBar(
              title: 'Locked',
              onBack: () => context.canPop()
                  ? context.pop()
                  : context.go('/home'),
            ),
            Expanded(
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 360),
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Container(
                          width: 56,
                          height: 56,
                          decoration: BoxDecoration(
                            color: t.leaf.withValues(alpha: 0.12),
                            borderRadius: BorderRadius.circular(14),
                            border: Border.all(
                              color: t.leaf.withValues(alpha: 0.3),
                            ),
                          ),
                          alignment: Alignment.center,
                          child: Icon(LucideIcons.lock, size: 24, color: t.leaf),
                        ),
                        const SizedBox(height: 16),
                        Text(
                          'Enter PIN',
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            color: t.text,
                            fontSize: 18,
                            fontWeight: FontWeight.w700,
                            letterSpacing: -0.2,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          'A PIN is required to access connections.',
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            color: t.muted,
                            fontSize: 13,
                            height: 1.4,
                          ),
                        ),
                        const SizedBox(height: 24),
                        NubeField(
                          controller: _controller,
                          label: 'PIN',
                          autofocus: true,
                          obscureText: true,
                          keyboardType: TextInputType.number,
                          inputFormatters: [
                            FilteringTextInputFormatter.digitsOnly,
                          ],
                          maxLength: 12,
                          textAlign: TextAlign.center,
                          onSubmitted: (_) => _submit(),
                          errorText: _error,
                        ),
                        const SizedBox(height: 16),
                        NubeButton(
                          label: 'Unlock',
                          icon: LucideIcons.unlock,
                          expand: true,
                          loading: _checking,
                          onPressed: _checking ? null : _submit,
                        ),
                        const SizedBox(height: 12),
                        // Subtle hint to keep the page calm
                        if (!isDark)
                          const SizedBox.shrink()
                        else
                          const SizedBox.shrink(),
                      ],
                    ),
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
