import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:rubix_flutter/core/i18n/generated/app_localizations.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/features/auth/data/auth_controller.dart';
import 'package:rubix_flutter/features/auth/data/auth_state.dart';
import 'package:rubix_flutter/features/connections/data/connection_credentials_store.dart';
import 'package:rubix_flutter/features/connections/presentation/connections_list/connections_controller.dart';
import 'package:rubix_flutter/features/home/presentation/home_controller.dart';
import 'package:rubix_flutter/shared/widgets/dashboard/dashboard.dart';
import 'package:rubix_flutter/shared/widgets/error_panel.dart';
import 'package:rubix_flutter/shared/widgets/loading_indicator.dart';
import 'package:rubix_flutter/shared/widgets/nube_widgets.dart';
import 'package:rubix_flutter/shared/widgets/scaffold/ambient_glow_background.dart';
import 'package:go_router/go_router.dart';

/// Main Home screen — Figma-aligned ambient hero with metric tiles.
class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = AppLocalizations.of(context);
    final activeAsync = ref.watch(activeConnectionProvider);
    final healthAsync = ref.watch(agentHealthProvider);
    final userAsync = ref.watch(currentUserProvider);
    final authState = ref.watch(authControllerProvider).value;
    final unauthenticated = authState is AuthUnauthenticated;
    final t = Theme.of(context).nube;

    final displayName = _displayName(userAsync.value?.email);

    return AmbientGlowBackground(
      child: Scaffold(
        backgroundColor: Colors.transparent,
        body: SafeArea(
          child: ListView(
            padding: const EdgeInsets.fromLTRB(20, 12, 20, 24),
            children: [
              // ─── Hero ──────────────────────────────────────────────────
              healthAsync.when(
                loading: () => const _AgentPillSkeleton(),
                error: (e, _) => _AgentPill.offline(label: l.agentUnreachable),
                data: (health) => switch (health) {
                  AgentHealthOk() => _AgentPill.online(
                      label: '${l.agentHealthy.toUpperCase()} · 24',
                    ),
                  AgentHealthBadStatus(:final statusCode) =>
                    _AgentPill.offline(
                      label: '${l.agentUnreachable} ($statusCode)',
                    ),
                  AgentHealthUnreachable() =>
                    _AgentPill.offline(label: l.agentUnreachable),
                },
              ),
              const SizedBox(height: 18),
              _Hero(name: displayName),
              const SizedBox(height: 10),
              Text(
                "Everything's running. 24 devices, 1 site.",
                style: TextStyle(
                  color: t.muted,
                  fontSize: 14,
                  height: 1.45,
                ),
              ),
              const SizedBox(height: 22),
              const _DevicesOnlineTile(),
              const SizedBox(height: 12),
              IntrinsicHeight(
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: const [
                    Expanded(child: _EnergyTile()),
                    SizedBox(width: 12),
                    Expanded(child: _AlertsTile()),
                  ],
                ),
              ),
              const SizedBox(height: 12),
              activeAsync.when(
                loading: () => const Padding(
                  padding: EdgeInsets.symmetric(vertical: 24),
                  child: LoadingIndicator(),
                ),
                error: (e, _) => ErrorPanel(
                  message: l.activeConnectionSection,
                  onRetry: () => ref.invalidate(activeConnectionProvider),
                ),
                data: (conn) {
                  if (conn == null) {
                    return _ActiveConnectionCard.placeholder(
                      onTap: () => context.push('/connections'),
                    );
                  }
                  return _ActiveConnectionCard(
                    label: conn.label,
                    baseUrl: conn.baseUrl,
                    healthAsync: healthAsync,
                    onTap: () => context.push('/connections'),
                  );
                },
              ),
              if (unauthenticated) ...[
                const SizedBox(height: 16),
                _SignInPanel(reason: authState.reason),
              ] else if (userAsync.hasError) ...[
                const SizedBox(height: 16),
                ErrorPanel(
                  message: l.currentUserError,
                  onRetry: () => ref.invalidate(currentUserProvider),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }

  static String _displayName(String? email) {
    if (email == null || email.isEmpty) return 'Lina';
    final local = email.split('@').first;
    if (local.isEmpty) return 'Lina';
    return local[0].toUpperCase() + local.substring(1);
  }
}

// ---------------------------------------------------------------------------
// Hero — display heading with serif-italic name accent.
// ---------------------------------------------------------------------------
class _Hero extends StatelessWidget {
  const _Hero({required this.name});
  final String name;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final greeting = _greetingFor(DateTime.now().hour);
    final base = TextStyle(
      color: t.text,
      fontSize: 38,
      height: 1.02,
      fontWeight: FontWeight.w500,
      letterSpacing: -1.4,
    );
    return Text.rich(
      TextSpan(
        children: [
          TextSpan(text: '$greeting,\n', style: base),
          TextSpan(
            text: '$name.',
            style: accentItalicTextStyle(context, fontSize: 38).copyWith(
              color: t.text,
              height: 1.02,
            ),
          ),
        ],
      ),
    );
  }

  static String _greetingFor(int hour) {
    if (hour < 5) return 'Good evening';
    if (hour < 12) return 'Good morning';
    if (hour < 18) return 'Good afternoon';
    return 'Good evening';
  }
}

// ---------------------------------------------------------------------------
// Agent status pill — small dark pill with coloured dot.
// ---------------------------------------------------------------------------
class _AgentPill extends StatelessWidget {
  const _AgentPill._({
    required this.label,
    required this.dotColor,
  });

  factory _AgentPill.online({required String label}) => _AgentPill._(
        label: label,
        dotColor: const Color(0xFF21C45D),
      );

  factory _AgentPill.offline({required String label}) => _AgentPill._(
        label: label.toUpperCase(),
        dotColor: const Color(0xFFEF4343),
      );

  final String label;
  final Color dotColor;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: t.surface,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(color: t.border),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 8,
              height: 8,
              decoration: BoxDecoration(
                color: dotColor,
                shape: BoxShape.circle,
                boxShadow: [
                  BoxShadow(
                    color: dotColor.withValues(alpha: 0.55),
                    blurRadius: 6,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Text(
              label,
              style: TextStyle(
                color: t.text,
                fontSize: 11,
                fontWeight: FontWeight.w600,
                letterSpacing: 2.0,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _AgentPillSkeleton extends StatelessWidget {
  const _AgentPillSkeleton();
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Align(
      alignment: Alignment.centerLeft,
      child: Container(
        width: 160,
        height: 32,
        decoration: BoxDecoration(
          color: t.surface,
          borderRadius: BorderRadius.circular(999),
          border: Border.all(color: t.border),
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Tiles
// ---------------------------------------------------------------------------
class _DevicesOnlineTile extends StatelessWidget {
  const _DevicesOnlineTile();

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return NubeGlowCard(
      tone: NubeGlowTone.teal,
      borderRadius: 18,
      padding: const EdgeInsets.fromLTRB(20, 18, 20, 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _Eyebrow('Devices online'),
          const SizedBox(height: 10),
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Text(
                '24',
                style: TextStyle(
                  color: t.text,
                  fontSize: 44,
                  fontWeight: FontWeight.w500,
                  letterSpacing: -1.8,
                  height: 1.0,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
              const SizedBox(width: 8),
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Text(
                  'of 24',
                  style: TextStyle(
                    color: t.muted,
                    fontSize: 14,
                  ),
                ),
              ),
              const Spacer(),
              const SizedBox(
                width: 130,
                child: NubeMiniSparkline(
                  values: [10, 12, 11, 14, 13, 16, 15, 18, 17, 19, 21, 22, 24],
                  tone: NubeGlowTone.teal,
                  height: 44,
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          Container(
            width: 38,
            height: 2,
            decoration: BoxDecoration(
              color: t.callout,
              borderRadius: BorderRadius.circular(999),
            ),
          ),
          const SizedBox(height: 10),
          Text(
            'All systems operational',
            style: TextStyle(color: t.muted, fontSize: 13),
          ),
        ],
      ),
    );
  }
}

class _EnergyTile extends StatelessWidget {
  const _EnergyTile();
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return NubeGlowCard(
      borderRadius: 16,
      padding: const EdgeInsets.fromLTRB(16, 14, 16, 14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _Eyebrow('Energy today'),
          const SizedBox(height: 8),
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Text(
                '39.7',
                style: TextStyle(
                  color: t.text,
                  fontSize: 30,
                  fontWeight: FontWeight.w500,
                  letterSpacing: -1.2,
                  height: 1.0,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
              const SizedBox(width: 4),
              Padding(
                padding: const EdgeInsets.only(bottom: 4),
                child: Text(
                  'kWh',
                  style: TextStyle(color: t.muted, fontSize: 13),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              Icon(LucideIcons.arrowUpRight, size: 13, color: t.success),
              const SizedBox(width: 4),
              Text(
                '5% lower',
                style: TextStyle(
                  color: t.success,
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _AlertsTile extends StatelessWidget {
  const _AlertsTile();
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return NubeGlowCard(
      borderRadius: 16,
      padding: const EdgeInsets.fromLTRB(16, 14, 16, 14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _Eyebrow('Alerts'),
          const SizedBox(height: 8),
          Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              Text(
                '3',
                style: TextStyle(
                  color: t.text,
                  fontSize: 30,
                  fontWeight: FontWeight.w500,
                  letterSpacing: -1.2,
                  height: 1.0,
                  fontFeatures: const [FontFeature.tabularFigures()],
                ),
              ),
              const SizedBox(width: 10),
              Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  color: t.callout,
                  shape: BoxShape.circle,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            '2 warnings, 1 info',
            style: TextStyle(color: t.muted, fontSize: 12),
          ),
        ],
      ),
    );
  }
}

class _ActiveConnectionCard extends StatelessWidget {
  const _ActiveConnectionCard({
    required this.label,
    required this.baseUrl,
    required this.healthAsync,
    this.onTap,
  })  : isPlaceholder = false;

  const _ActiveConnectionCard.placeholder({this.onTap})
      : label = 'No active connection',
        baseUrl = 'Pick a connection to get started',
        healthAsync = const AsyncValue.data(null),
        isPlaceholder = true;

  final String label;
  final String baseUrl;
  final AsyncValue<dynamic> healthAsync;
  final VoidCallback? onTap;
  final bool isPlaceholder;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final online = !isPlaceholder &&
        healthAsync.maybeWhen(
          data: (h) => h is AgentHealthOk,
          orElse: () => false,
        );
    final statusColor = online
        ? t.success
        : (isPlaceholder ? t.muted : t.danger);
    final statusLabel = isPlaceholder
        ? 'Not connected'
        : (online ? 'Connected' : 'Offline');

    return NubeGlowCard(
      tone: NubeGlowTone.none,
      borderRadius: 18,
      padding: const EdgeInsets.fromLTRB(20, 18, 20, 18),
      onTap: onTap,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _Eyebrow('Active connection'),
          const SizedBox(height: 10),
          Text(
            label,
            style: TextStyle(
              color: t.text,
              fontSize: 20,
              fontWeight: FontWeight.w600,
              letterSpacing: -0.4,
              height: 1.15,
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  color: statusColor,
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: 8),
              Text(
                statusLabel,
                style: TextStyle(
                  color: statusColor,
                  fontSize: 13,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(width: 12),
              Flexible(
                child: Text(
                  baseUrl,
                  style: TextStyle(color: t.muted, fontSize: 13),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Divider(height: 1, color: t.border),
          const SizedBox(height: 14),
          Row(
            children: const [
              Expanded(child: _MetricStat(label: 'Latency', value: '12 ms')),
              Expanded(child: _MetricStat(label: 'Uptime', value: '99.9%')),
              Expanded(child: _MetricStat(label: 'Agent', value: 'v2.4.1')),
            ],
          ),
        ],
      ),
    );
  }
}

class _MetricStat extends StatelessWidget {
  const _MetricStat({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label.toUpperCase(),
          style: TextStyle(
            color: t.muted,
            fontSize: 10,
            fontWeight: FontWeight.w600,
            letterSpacing: 1.6,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          value,
          style: TextStyle(
            color: t.text,
            fontSize: 14,
            fontWeight: FontWeight.w600,
            fontFeatures: const [FontFeature.tabularFigures()],
          ),
        ),
      ],
    );
  }
}

class _Eyebrow extends StatelessWidget {
  const _Eyebrow(this.label);
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
        letterSpacing: 2.2,
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
