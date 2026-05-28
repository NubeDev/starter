import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/auth/data/auth_state.dart';
import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/home/presentation/home_controller.dart';
import 'package:rubix_flutter/shared/widgets/error_panel.dart';
import 'package:rubix_flutter/shared/widgets/loading_indicator.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';
import 'package:rubix_flutter/shared/widgets/unreachable_panel.dart';

/// Pinned home screen — chassis validation for Block 5.
// Deprecated: Use the new Figma-aligned HomeScreen in home_screen_rethemed.dart
@Deprecated('Use HomeScreen in home_screen_rethemed.dart')
class HomeScreenOld extends ConsumerWidget {
  const HomeScreenOld({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final activeAsync = ref.watch(activeConnectionProvider);
    final healthAsync = ref.watch(agentHealthProvider);
    final userAsync = ref.watch(currentUserProvider);
    final authState = ref.watch(authControllerProvider).value;
    final unauthenticated = authState is AuthUnauthenticated;

    return RefreshIndicator(
      onRefresh: () async {
        ref
          ..invalidate(agentHealthProvider)
          ..invalidate(currentUserProvider);
        await Future.wait<void>([
          ref.read(agentHealthProvider.future),
          ref.read(currentUserProvider.future).catchError((_) {
            throw Exception('ignored');
          }),
        ]);
      },
      child: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          _SectionHeader(label: l.agentHealthSection),
          const SizedBox(height: 10),
          healthAsync.when(
            loading: () => const _PillSkeleton(),
            error: (e, _) => _HealthPill.unreachable(label: l.agentUnreachable),
            data: (health) => switch (health) {
              AgentHealthOk() => _HealthPill.ok(label: l.agentHealthy),
              AgentHealthBadStatus(:final statusCode) =>
                _HealthPill.unreachable(
                  label: '${l.agentUnreachable} ($statusCode)',
                ),
              AgentHealthUnreachable() =>
                _HealthPill.unreachable(label: l.agentUnreachable),
            },
          ),
          const SizedBox(height: 28),

          _SectionHeader(label: l.currentUserSection),
          const SizedBox(height: 10),
          if (unauthenticated)
            _SignInPanel(reason: authState.reason)
          else
            userAsync.when(
              loading: () => const Padding(
                padding: EdgeInsets.symmetric(vertical: 24),
                child: LoadingIndicator(),
              ),
              error: (e, _) => ErrorPanel(
                message: l.currentUserError,
                onRetry: () => ref.invalidate(currentUserProvider),
              ),
              data: (user) => NubeCard(
                child: _InfoRow(
                  icon: LucideIcons.user,
                  title: user.email,
                  subtitle: user.role,
                ),
              ),
            ),
          const SizedBox(height: 28),

          _SectionHeader(label: l.activeConnectionSection),
          const SizedBox(height: 10),
          activeAsync.when(
            loading: () => const Padding(
              padding: EdgeInsets.symmetric(vertical: 24),
              child: LoadingIndicator(),
            ),
            error: (e, _) => ErrorPanel(message: e.toString()),
            data: (conn) {
              if (conn == null) {
                return UnreachablePanel(
                  onRetry: () => context.push('/connections'),
                );
              }
              return NubeCard(
                onTap: () => context.push('/connections'),
                child: _InfoRow(
                  icon: LucideIcons.link,
                  title: conn.label,
                  subtitle: conn.baseUrl,
                  trailing: Icon(
                    LucideIcons.settings2,
                    size: 16,
                    color: Theme.of(context).nube.muted,
                  ),
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Text(
      label.toUpperCase(),
      style: TextStyle(
        color: t.muted,
        fontSize: 11,
        fontWeight: FontWeight.w600,
        letterSpacing: 0.6,
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    this.trailing,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Row(
      children: [
        Container(
          width: 36,
          height: 36,
          decoration: BoxDecoration(
            color: t.surface2,
            borderRadius: BorderRadius.circular(8),
          ),
          alignment: Alignment.center,
          child: Icon(icon, size: 16, color: t.muted),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                title,
                style: TextStyle(
                  color: t.text,
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                  height: 1.25,
                ),
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 2),
              Text(
                subtitle,
                style: TextStyle(
                  color: t.muted,
                  fontSize: 12.5,
                  height: 1.3,
                ),
                overflow: TextOverflow.ellipsis,
              ),
            ],
          ),
        ),
        if (trailing != null) ...[
          const SizedBox(width: 8),
          trailing!,
        ],
      ],
    );
  }
}

class _HealthPill extends StatelessWidget {
  const _HealthPill._({
    required this.label,
    required this.color,
    required this.icon,
  });

  factory _HealthPill.ok({required String label}) => _HealthPill._(
        label: label,
        color: const Color(0xFF21C45D),
        icon: LucideIcons.checkCircle,
      );

  factory _HealthPill.unreachable({required String label}) => _HealthPill._(
        label: label,
        color: const Color(0xFFEF4343),
        icon: LucideIcons.alertCircle,
      );

  final String label;
  final Color color;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.10),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: color.withValues(alpha: 0.30)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(
              color: color,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 8),
          Text(
            label,
            style: TextStyle(
              color: color,
              fontSize: 12.5,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}

class _PillSkeleton extends StatelessWidget {
  const _PillSkeleton();

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 120,
      height: 26,
      decoration: BoxDecoration(
        color: Theme.of(context).nube.surface2,
        borderRadius: BorderRadius.circular(6),
      ),
    );
  }
}

class _SignInPanel extends ConsumerWidget {
  const _SignInPanel({this.reason});
  final String? reason;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final t = Theme.of(context).nube;
    return NubeCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(LucideIcons.userX, size: 16, color: t.muted),
              const SizedBox(width: 8),
              Text(
                'Not signed in',
                style: TextStyle(
                  color: t.text,
                  fontSize: 14,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
          if (reason != null) ...[
            const SizedBox(height: 4),
            Text(
              reason!,
              style: TextStyle(color: t.muted, fontSize: 12),
            ),
          ],
          const SizedBox(height: 12),
          NubeButton(
            label: 'Sign in',
            icon: LucideIcons.logIn,
            size: NubeButtonSize.sm,
            onPressed: () => showDialog<void>(
              context: context,
              builder: (_) => const _SignInDialog(),
            ),
          ),
        ],
      ),
    );
  }
}

class _SignInDialog extends ConsumerStatefulWidget {
  const _SignInDialog();

  @override
  ConsumerState<_SignInDialog> createState() => _SignInDialogState();
}

/// Dev defaults — mirror the constants in add_connection_screen.dart so
/// re-signing in on a connection whose creds got lost (e.g. the old
/// flutter_secure_storage web blob became unreadable after the switch
/// to localStorage) is one click.
const _devEmail = 'op@example.com';
const _devPassword = 'rubix-dev-passwd';

class _SignInDialogState extends ConsumerState<_SignInDialog> {
  final _emailController = TextEditingController(text: _devEmail);
  final _passwordController = TextEditingController(text: _devPassword);
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final email = _emailController.text.trim();
    final password = _passwordController.text;
    if (email.isEmpty || password.isEmpty) {
      setState(() => _error = 'Email and password are required');
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await ref.read(authControllerProvider.notifier).login(
            email: email,
            password: password,
          );
      // Persist creds against the active connection so subsequent
      // launches auto-login.
      final active = ref.read(activeConnectionProvider).value;
      if (active != null) {
        await ref.read(connectionCredentialsStoreProvider).write(
              active.id,
              ConnectionCredentials(email: email, password: password),
            );
      }
      if (!mounted) return;
      Navigator.of(context).pop();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Sign in'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          NubeField(
            controller: _emailController,
            label: 'Email',
            prefixIcon: LucideIcons.mail,
            keyboardType: TextInputType.emailAddress,
            autocorrect: false,
          ),
          const SizedBox(height: 12),
          NubeField(
            controller: _passwordController,
            label: 'Password',
            obscureText: true,
            prefixIcon: LucideIcons.lock,
          ),
          if (_error != null) ...[
            const SizedBox(height: 8),
            Text(
              _error!,
              style: TextStyle(
                color: Theme.of(context).nube.danger,
                fontSize: 12,
              ),
            ),
          ],
        ],
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        NubeButton(
          label: 'Sign in',
          size: NubeButtonSize.sm,
          loading: _busy,
          onPressed: _busy ? null : _submit,
        ),
      ],
    );
  }
}
